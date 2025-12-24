//! Job executor for the GUI
//!
//! Runs in a background thread and processes queued jobs

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use crate::agent::{AgentRegistry, ChainProgressEvent, ChainRunner};
use crate::config::Config;
use crate::git::GitManager;
use crate::job::JobManager;
use crate::{ChainStepSummary, Job, JobResult, JobStatus, LogEvent, LogEventKind, SessionMode};

/// Message to send back to GUI
#[derive(Debug, Clone)]
pub enum ExecutorEvent {
    /// Job started running
    JobStarted(u64),
    /// Job completed successfully
    JobCompleted(u64),
    /// Job failed with error
    JobFailed(u64, String),
    /// Chain step completed
    ChainStepCompleted {
        job_id: u64,
        step_index: usize,
        total_steps: usize,
        mode: String,
        state: Option<String>,
        /// Summary of the completed step for UI display
        step_summary: ChainStepSummary,
    },
    /// Chain completed
    ChainCompleted {
        job_id: u64,
        chain_name: String,
        steps_executed: usize,
        success: bool,
    },
    /// Log message
    Log(LogEvent),
    /// Permission request from Bridge (tool approval needed)
    PermissionNeeded {
        job_id: u64,
        request_id: String,
        session_id: String,
        tool_name: String,
        tool_input: std::collections::HashMap<String, serde_json::Value>,
    },
}

/// Start the job executor in a background thread
pub fn start_executor(
    work_dir: PathBuf,
    config: Arc<RwLock<Config>>,
    job_manager: Arc<Mutex<JobManager>>,
    event_tx: Sender<ExecutorEvent>,
    max_concurrent_jobs: Arc<AtomicUsize>,
) {
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(executor_loop(
            work_dir,
            config,
            job_manager,
            event_tx,
            max_concurrent_jobs,
        ));
    });
}

async fn executor_loop(
    work_dir: PathBuf,
    config: Arc<RwLock<Config>>,
    job_manager: Arc<Mutex<JobManager>>,
    event_tx: Sender<ExecutorEvent>,
    max_concurrent_jobs: Arc<AtomicUsize>,
) {
    let agent_registry = AgentRegistry::new();

    // Try to get git manager
    let git_manager = GitManager::new(&work_dir).ok();

    loop {
        let should_use_worktree = config
            .read()
            .map(|cfg| cfg.settings.use_worktree)
            .unwrap_or(false);

        // Check for queued jobs and handle file locks
        let (running_count, next_queued) = {
            let mut manager = job_manager.lock().unwrap();
            let running = manager
                .jobs()
                .iter()
                .filter(|j| j.status == JobStatus::Running)
                .count();

            // First, check if any blocked jobs can be unblocked
            // (their blocking job has finished)
            let blocked_jobs: Vec<_> = manager
                .jobs()
                .iter()
                .filter(|j| j.status == JobStatus::Blocked)
                .map(|j| (j.id, j.blocked_by, j.source_file.clone()))
                .collect();

            for (job_id, blocked_by, source_file) in blocked_jobs {
                // Check if the blocking job still holds the lock
                if let Some(blocking_id) = blocked_by {
                    if manager
                        .get_blocking_job(&source_file, Some(job_id))
                        .is_none()
                    {
                        // Lock is free, unblock this job
                        if let Some(j) = manager.get_mut(job_id) {
                            j.status = JobStatus::Queued;
                            j.blocked_by = None;
                            j.blocked_file = None;
                        }
                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                            "Job #{} unblocked (Job #{} finished)",
                            job_id, blocking_id
                        ))));
                    }
                }
            }

            // Find next queued job (skip blocked jobs)
            let next: Option<Job> = manager
                .jobs()
                .iter()
                .find(|j| j.status == JobStatus::Queued)
                .map(|j| (*j).clone());

            (running, next)
        };

        // Start next job if we have capacity (read current limit each iteration)
        if running_count < max_concurrent_jobs.load(Ordering::Relaxed) {
            if let Some(job) = next_queued {
                // Check file lock (only when not using worktrees)
                let is_multi_agent = job.group_id.is_some();
                let needs_lock_check =
                    !should_use_worktree && !is_multi_agent && !job.force_worktree;

                if needs_lock_check {
                    let mut manager = job_manager.lock().unwrap();

                    // Check if the source file is locked by another job
                    if let Some(blocking_job_id) =
                        manager.get_blocking_job(&job.source_file, Some(job.id))
                    {
                        // File is locked - mark job as blocked
                        if let Some(j) = manager.get_mut(job.id) {
                            j.status = JobStatus::Blocked;
                            j.blocked_by = Some(blocking_job_id);
                            j.blocked_file = Some(job.source_file.clone());
                        }
                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                            "Job #{} blocked by Job #{} (file: {})",
                            job.id,
                            blocking_job_id,
                            job.source_file
                                .file_name()
                                .map(|f| f.to_string_lossy().to_string())
                                .unwrap_or_else(|| job.source_file.display().to_string())
                        ))));
                        // Continue to next iteration - this job can't start yet
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }

                    // Acquire the lock
                    manager.try_lock_file(&job.source_file, job.id);
                }

                // Spawn job in background so we can start multiple jobs concurrently
                let work_dir = work_dir.clone();
                let config_snapshot = config.read().map(|cfg| cfg.clone()).unwrap_or_default();
                let job_manager = Arc::clone(&job_manager);
                let agent_registry = agent_registry.clone();
                let git_manager = git_manager.clone();
                let event_tx = event_tx.clone();

                tokio::spawn(async move {
                    run_job(
                        &work_dir,
                        &config_snapshot,
                        &job_manager,
                        &agent_registry,
                        git_manager.as_ref(),
                        &event_tx,
                        job,
                    )
                    .await;
                });
            }
        }

        // Sleep before checking again
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

async fn run_job(
    work_dir: &PathBuf,
    config: &Config,
    job_manager: &Arc<Mutex<JobManager>>,
    agent_registry: &AgentRegistry,
    git_manager: Option<&GitManager>,
    event_tx: &Sender<ExecutorEvent>,
    mut job: Job,
) {
    let job_id = job.id;

    // Check if this mode is actually a chain
    if config.is_chain(&job.mode) {
        run_chain_job(
            work_dir,
            config,
            job_manager,
            agent_registry,
            git_manager,
            event_tx,
            job,
        )
        .await;
        return;
    }

    // Update status to running
    {
        let mut manager = job_manager.lock().unwrap();
        manager.set_status(job_id, JobStatus::Running);
    }

    let _ = event_tx.send(ExecutorEvent::JobStarted(job_id));
    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
        "Starting job #{}",
        job_id
    ))));

    // Determine working directory
    // Multi-agent jobs (jobs with group_id) ALWAYS require worktrees for isolation,
    // regardless of the use_worktree config setting.
    // Jobs with force_worktree=true (submitted with Shift+Enter) also use worktrees.
    let is_multi_agent_job = job.group_id.is_some();
    let should_use_worktree =
        config.settings.use_worktree || is_multi_agent_job || job.force_worktree;

    // Get the effective working directory from job's workspace_path, falling back to work_dir
    // This is crucial for multi-workspace support where jobs may target different repositories
    let job_work_dir = job
        .workspace_path
        .clone()
        .unwrap_or_else(|| work_dir.clone());

    // Create GitManager for the job's workspace (may be different from global work_dir)
    let job_git_manager =
        if job.workspace_path.is_some() && job.workspace_path.as_ref() != Some(work_dir) {
            // Job has a different workspace, create a new GitManager for it
            GitManager::new(&job_work_dir).ok()
        } else {
            None
        };
    let effective_git_manager = job_git_manager.as_ref().or(git_manager);

    // Reuse existing worktree when present (e.g., session continuation)
    let (worktree_path, _is_isolated) = if let Some(existing_worktree) =
        job.git_worktree_path.as_ref().filter(|p| p.exists())
    {
        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
            "Reusing worktree: {}",
            existing_worktree.display()
        ))));
        (existing_worktree.clone(), true)
    } else if should_use_worktree {
        if let Some(git) = effective_git_manager {
            match git.create_worktree(job_id) {
                Ok(worktree_info) => {
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                        "Created worktree: {}",
                        worktree_info.path.display()
                    ))));
                    job.git_worktree_path = Some(worktree_info.path.clone());
                    job.base_branch = Some(worktree_info.base_branch.clone());
                    // Store worktree path and base branch
                    {
                        let mut manager = job_manager.lock().unwrap();
                        if let Some(j) = manager.get_mut(job_id) {
                            j.git_worktree_path = Some(worktree_info.path.clone());
                            j.base_branch = Some(worktree_info.base_branch);
                        }
                    }
                    (worktree_info.path, true)
                }
                Err(e) => {
                    // For multi-agent jobs or force_worktree (Shift+Enter), worktree creation failure is fatal
                    if is_multi_agent_job || job.force_worktree {
                        let reason = if is_multi_agent_job {
                            "parallel execution"
                        } else {
                            "Shift+Enter submission"
                        };
                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                            "Worktree required for {} but creation failed: {}",
                            reason, e
                        ))));
                        let _ = event_tx.send(ExecutorEvent::JobFailed(
                            job_id,
                            format!("Worktree required for {}: {}", reason, e),
                        ));
                        {
                            let mut manager = job_manager.lock().unwrap();
                            manager.set_status(job_id, JobStatus::Failed);
                            if let Some(j) = manager.get_mut(job_id) {
                                j.error_message = Some(format!("Worktree creation failed: {}", e));
                            }
                        }
                        return;
                    }
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                        "Failed to create worktree: {}",
                        e
                    ))));
                    // Fall back to job's workspace path instead of global work_dir
                    (job_work_dir.clone(), false)
                }
            }
        } else {
            // No git manager available
            if is_multi_agent_job || job.force_worktree {
                let reason = if is_multi_agent_job {
                    "parallel execution"
                } else {
                    "Shift+Enter submission"
                };
                let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                    "Worktree required for {} but no git repository available",
                    reason
                ))));
                let _ = event_tx.send(ExecutorEvent::JobFailed(
                    job_id,
                    format!("Git repository required for {}", reason),
                ));
                {
                    let mut manager = job_manager.lock().unwrap();
                    manager.set_status(job_id, JobStatus::Failed);
                    if let Some(j) = manager.get_mut(job_id) {
                        j.error_message = Some(format!("Git repository required for {}", reason));
                    }
                }
                return;
            }
            // Fall back to job's workspace path instead of global work_dir
            (job_work_dir.clone(), false)
        }
    } else {
        // Worktrees disabled - use job's workspace path
        (job_work_dir.clone(), false)
    };

    // Get agent config with mode-specific tool overrides
    let agent_config = config
        .get_agent_for_job(&job.agent_id, &job.mode)
        .unwrap_or_default();

    // Mark whether this is a REPL job (for UI to show correct buttons)
    let is_repl = matches!(agent_config.session_mode, SessionMode::Repl);
    {
        let mut manager = job_manager.lock().unwrap();
        if let Some(j) = manager.get_mut(job_id) {
            j.is_repl = is_repl;
        }
    }

    // Get adapter
    let adapter = match agent_registry.get_for_config(&agent_config) {
        Some(a) => a,
        None => {
            let error = format!("No adapter found for agent '{}'", job.agent_id);
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(error.clone())));
            {
                let mut manager = job_manager.lock().unwrap();
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(error.clone());
                }
            }
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
            return;
        }
    };

    // Create log channel for the adapter
    let (log_tx, mut log_rx) = tokio::sync::mpsc::channel::<LogEvent>(100);

    // Spawn a task to forward logs
    let event_tx_clone = event_tx.clone();
    let job_manager_clone = Arc::clone(job_manager);
    let permission_job_id = job_id;
    let log_forwarder = tokio::spawn(async move {
        while let Some(log) = log_rx.recv().await {
            if let Some(args) = log.tool_args.as_ref() {
                if let Some(session_id) = args.get("session_id").and_then(|v| v.as_str()) {
                    if let Ok(mut manager) = job_manager_clone.lock() {
                        if let Some(job) = manager.get_mut(permission_job_id) {
                            job.bridge_session_id = Some(session_id.to_string());
                        }
                    }
                }
            }

            if log.kind == LogEventKind::Permission {
                let args = match log.tool_args {
                    Some(a) => a,
                    None => continue,
                };

                let Some(request_id) = args
                    .get("request_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                else {
                    continue;
                };

                let session_id = args
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let tool_name = args
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let tool_input = args
                    .get("tool_input")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect::<std::collections::HashMap<String, serde_json::Value>>()
                    })
                    .unwrap_or_default();

                let _ = event_tx_clone.send(ExecutorEvent::PermissionNeeded {
                    job_id: permission_job_id,
                    request_id,
                    session_id,
                    tool_name,
                    tool_input,
                });

                continue;
            }

            let _ = event_tx_clone.send(ExecutorEvent::Log(log));
        }
    });

    // Run the adapter
    match adapter
        .run(&job, &worktree_path, &agent_config, log_tx)
        .await
    {
        Ok(result) => {
            let mut manager = job_manager.lock().unwrap();
            if let Some(j) = manager.get_mut(job_id) {
                j.sent_prompt = result.sent_prompt.clone();

                // Parse the output for YAML result block
                if let Some(output) = &result.output_text {
                    // Always keep the full response for display (even when a structured YAML block is present).
                    j.full_response = Some(output.clone());
                    j.parse_result(output);
                }

                // Store session ID for continuation
                if result.session_id.is_some() {
                    j.bridge_session_id = result.session_id.clone();
                }

                // Calculate file stats from changed_files
                let files_changed = result.changed_files.len();
                if files_changed > 0 {
                    let (lines_added, lines_removed) = if j.git_worktree_path.is_some() {
                        calculate_git_numstat(&worktree_path, j.base_branch.as_deref())
                    } else {
                        (0, 0)
                    };
                    j.set_file_stats(files_changed, lines_added, lines_removed);
                }

                if result.success {
                    j.set_status(JobStatus::Done);
                    j.changed_files = result.changed_files;
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                        "Job #{} completed",
                        job_id
                    ))));
                    let _ = event_tx.send(ExecutorEvent::JobCompleted(job_id));
                } else {
                    let error = result.error.unwrap_or_else(|| "Unknown error".to_string());
                    j.fail(error.clone());
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                        "Job #{} failed: {}",
                        job_id, error
                    ))));
                    let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
                }
            }
        }
        Err(e) => {
            let error = e.to_string();
            {
                let mut manager = job_manager.lock().unwrap();
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(error.clone());
                }
            }
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                "Job #{} error: {}",
                job_id, error
            ))));
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
        }
    }

    // Wait for log forwarder to finish
    let _ = log_forwarder.await;

    // Release file locks held by this job
    {
        let mut manager = job_manager.lock().unwrap();
        manager.release_job_locks(job_id);
    }
}

fn calculate_git_numstat(worktree: &Path, base_branch: Option<&str>) -> (usize, usize) {
    fn parse_numstat(output: &str) -> (usize, usize) {
        let mut lines_added = 0usize;
        let mut lines_removed = 0usize;

        for line in output.lines() {
            let mut parts = line.split('\t');
            let Some(added) = parts.next() else { continue };
            let Some(removed) = parts.next() else {
                continue;
            };

            if added != "-" {
                lines_added = lines_added.saturating_add(added.parse::<usize>().unwrap_or(0));
            }
            if removed != "-" {
                lines_removed = lines_removed.saturating_add(removed.parse::<usize>().unwrap_or(0));
            }
        }

        (lines_added, lines_removed)
    }

    fn run_git_numstat(worktree: &Path, args: &[&str]) -> Option<(usize, usize)> {
        let output = Command::new("git")
            .args(args)
            .current_dir(worktree)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        Some(parse_numstat(&String::from_utf8_lossy(&output.stdout)))
    }

    let mut total = (0usize, 0usize);

    // Count committed changes on the worktree branch.
    if let Some(base_branch) = base_branch {
        let range = format!("{}...HEAD", base_branch);
        if let Some((added, removed)) =
            run_git_numstat(worktree, &["diff", "--numstat", "--no-color", &range])
        {
            total.0 = total.0.saturating_add(added);
            total.1 = total.1.saturating_add(removed);
        }
    }

    // Count uncommitted changes.
    if let Some((added, removed)) = run_git_numstat(worktree, &["diff", "--numstat", "--no-color"])
    {
        total.0 = total.0.saturating_add(added);
        total.1 = total.1.saturating_add(removed);
    }

    total
}

/// Run a job that is actually a chain of modes
async fn run_chain_job(
    work_dir: &PathBuf,
    config: &Config,
    job_manager: &Arc<Mutex<JobManager>>,
    agent_registry: &AgentRegistry,
    git_manager: Option<&GitManager>,
    event_tx: &Sender<ExecutorEvent>,
    mut job: Job,
) {
    let job_id = job.id;
    let chain_name = job.mode.clone();

    // Get the chain config
    let chain = match config.get_chain(&chain_name) {
        Some(c) => c.clone(),
        None => {
            let error = format!("Chain '{}' not found", chain_name);
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(error.clone())));
            {
                let mut manager = job_manager.lock().unwrap();
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(error.clone());
                }
            }
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
            return;
        }
    };

    // Update status to running and set chain metadata
    {
        let mut manager = job_manager.lock().unwrap();
        manager.set_status(job_id, JobStatus::Running);
        if let Some(j) = manager.get_mut(job_id) {
            j.chain_name = Some(chain_name.clone());
            j.chain_total_steps = Some(chain.steps.len());
            j.chain_current_step = Some(0);
        }
    }

    let _ = event_tx.send(ExecutorEvent::JobStarted(job_id));
    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
        "Starting chain '{}' with {} steps for job #{}",
        chain_name,
        chain.steps.len(),
        job_id
    ))));

    // Determine working directory
    // Multi-agent jobs (jobs with group_id) ALWAYS require worktrees for isolation
    let is_multi_agent_job = job.group_id.is_some();
    let should_use_worktree =
        config.settings.use_worktree || is_multi_agent_job || job.force_worktree;

    // Get the effective working directory from job's workspace_path, falling back to work_dir
    let job_work_dir = job
        .workspace_path
        .clone()
        .unwrap_or_else(|| work_dir.clone());

    // Create GitManager for the job's workspace (may be different from global work_dir)
    let job_git_manager =
        if job.workspace_path.is_some() && job.workspace_path.as_ref() != Some(work_dir) {
            GitManager::new(&job_work_dir).ok()
        } else {
            None
        };
    let effective_git_manager = job_git_manager.as_ref().or(git_manager);

    // Reuse existing worktree when present (e.g., session continuation)
    let (worktree_path, _is_isolated) = if let Some(existing_worktree) =
        job.git_worktree_path.as_ref().filter(|p| p.exists())
    {
        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
            "Reusing worktree: {}",
            existing_worktree.display()
        ))));
        (existing_worktree.clone(), true)
    } else if should_use_worktree {
        if let Some(git) = effective_git_manager {
            match git.create_worktree(job_id) {
                Ok(worktree_info) => {
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                        "Created worktree: {}",
                        worktree_info.path.display()
                    ))));
                    job.git_worktree_path = Some(worktree_info.path.clone());
                    job.base_branch = Some(worktree_info.base_branch.clone());
                    {
                        let mut manager = job_manager.lock().unwrap();
                        if let Some(j) = manager.get_mut(job_id) {
                            j.git_worktree_path = Some(worktree_info.path.clone());
                            j.base_branch = Some(worktree_info.base_branch);
                        }
                    }
                    (worktree_info.path, true)
                }
                Err(e) => {
                    // For multi-agent jobs or force_worktree (Shift+Enter), worktree creation failure is fatal
                    if is_multi_agent_job || job.force_worktree {
                        let reason = if is_multi_agent_job {
                            "parallel execution"
                        } else {
                            "Shift+Enter submission"
                        };
                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                            "Worktree required for {} but creation failed: {}",
                            reason, e
                        ))));
                        let _ = event_tx.send(ExecutorEvent::JobFailed(
                            job_id,
                            format!("Worktree required for {}: {}", reason, e),
                        ));
                        {
                            let mut manager = job_manager.lock().unwrap();
                            manager.set_status(job_id, JobStatus::Failed);
                            if let Some(j) = manager.get_mut(job_id) {
                                j.error_message = Some(format!("Worktree creation failed: {}", e));
                            }
                        }
                        return;
                    }
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                        "Failed to create worktree: {}",
                        e
                    ))));
                    (job_work_dir.clone(), false)
                }
            }
        } else {
            // No git manager available
            if is_multi_agent_job || job.force_worktree {
                let reason = if is_multi_agent_job {
                    "parallel execution"
                } else {
                    "Shift+Enter submission"
                };
                let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                    "Worktree required for {} but no git repository available",
                    reason
                ))));
                let _ = event_tx.send(ExecutorEvent::JobFailed(
                    job_id,
                    format!("Git repository required for {}", reason),
                ));
                {
                    let mut manager = job_manager.lock().unwrap();
                    manager.set_status(job_id, JobStatus::Failed);
                    if let Some(j) = manager.get_mut(job_id) {
                        j.error_message = Some(format!("Git repository required for {}", reason));
                    }
                }
                return;
            }
            (job_work_dir.clone(), false)
        }
    } else {
        (job_work_dir.clone(), false)
    };

    // Create log channel
    let (log_tx, mut log_rx) = tokio::sync::mpsc::channel::<LogEvent>(100);

    // Forward logs to event_tx
    let event_tx_clone = event_tx.clone();
    let job_manager_clone = Arc::clone(job_manager);
    let permission_job_id = job_id;
    let log_forwarder = tokio::spawn(async move {
        while let Some(log) = log_rx.recv().await {
            if let Some(args) = log.tool_args.as_ref() {
                if let Some(session_id) = args.get("session_id").and_then(|v| v.as_str()) {
                    if let Ok(mut manager) = job_manager_clone.lock() {
                        if let Some(job) = manager.get_mut(permission_job_id) {
                            job.bridge_session_id = Some(session_id.to_string());
                        }
                    }
                }
            }

            if log.kind == LogEventKind::Permission {
                let args = match log.tool_args {
                    Some(a) => a,
                    None => continue,
                };

                let Some(request_id) = args
                    .get("request_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                else {
                    continue;
                };

                let session_id = args
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let tool_name = args
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let tool_input = args
                    .get("tool_input")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect::<std::collections::HashMap<String, serde_json::Value>>()
                    })
                    .unwrap_or_default();

                let _ = event_tx_clone.send(ExecutorEvent::PermissionNeeded {
                    job_id: permission_job_id,
                    request_id,
                    session_id,
                    tool_name,
                    tool_input,
                });

                continue;
            }

            let _ = event_tx_clone.send(ExecutorEvent::Log(log));
        }
    });

    // Create chain runner
    let chain_runner = ChainRunner::new(config, agent_registry, &worktree_path);

    // Create progress channel for real-time updates
    let (progress_tx, progress_rx) = std::sync::mpsc::channel::<ChainProgressEvent>();

    // Spawn a task to forward progress events to the GUI
    let event_tx_progress = event_tx.clone();
    let job_manager_progress = Arc::clone(job_manager);
    let progress_job_id = job_id;
    let total_steps_for_progress = chain.steps.len();
    let progress_forwarder = tokio::spawn(async move {
        while let Ok(progress) = progress_rx.recv() {
            // Update job's current step in real-time
            if let Ok(mut manager) = job_manager_progress.lock() {
                if let Some(j) = manager.get_mut(progress_job_id) {
                    if progress.is_starting {
                        j.chain_current_step = Some(progress.step_index);
                    } else {
                        j.chain_current_step = Some(progress.step_index + 1);
                        // Add completed step to history
                        if let Some(step_result) = &progress.step_result {
                            let summary = ChainStepSummary {
                                step_index: step_result.step_index,
                                mode: step_result.mode.clone(),
                                skipped: step_result.skipped,
                                success: step_result
                                    .agent_result
                                    .as_ref()
                                    .map(|ar| ar.success)
                                    .unwrap_or(false),
                                title: step_result
                                    .job_result
                                    .as_ref()
                                    .and_then(|jr| jr.title.clone()),
                                summary: step_result
                                    .job_result
                                    .as_ref()
                                    .and_then(|jr| jr.summary.clone()),
                                full_response: step_result.full_response.clone(),
                                error: step_result
                                    .agent_result
                                    .as_ref()
                                    .and_then(|ar| ar.error.clone()),
                                files_changed: step_result
                                    .agent_result
                                    .as_ref()
                                    .map(|ar| ar.files_changed)
                                    .unwrap_or(0),
                            };
                            // Only add if not already present
                            if j.chain_step_history.len() <= step_result.step_index {
                                j.chain_step_history.push(summary.clone());
                            }
                            // Send event for UI update
                            let _ = event_tx_progress.send(ExecutorEvent::ChainStepCompleted {
                                job_id: progress_job_id,
                                step_index: step_result.step_index,
                                total_steps: total_steps_for_progress,
                                mode: step_result.mode.clone(),
                                state: step_result
                                    .job_result
                                    .as_ref()
                                    .and_then(|jr| jr.state.clone()),
                                step_summary: summary,
                            });
                        }
                    }
                }
            }
        }
    });

    // Run the chain with progress channel
    let chain_result = chain_runner
        .run_chain(&chain_name, &chain, &job, log_tx, Some(progress_tx))
        .await;

    // Progress channel is dropped when run_chain returns, so progress_forwarder will exit
    // Give it a moment to process any remaining events
    tokio::time::sleep(Duration::from_millis(50)).await;
    progress_forwarder.abort(); // Clean up the forwarder task

    // Update job with results
    let total_steps = chain_result.step_results.len();
    {
        let mut manager = job_manager.lock().unwrap();
        if let Some(j) = manager.get_mut(job_id) {
            // Combine results from all steps
            let mut combined_details = Vec::new();
            let mut total_files_changed = 0;
            let mut step_history = Vec::new();

            for step_result in &chain_result.step_results {
                // Build ChainStepSummary for history
                let summary = ChainStepSummary {
                    step_index: step_result.step_index,
                    mode: step_result.mode.clone(),
                    skipped: step_result.skipped,
                    success: step_result
                        .agent_result
                        .as_ref()
                        .map(|ar| ar.success)
                        .unwrap_or(false),
                    title: step_result
                        .job_result
                        .as_ref()
                        .and_then(|jr| jr.title.clone()),
                    summary: step_result
                        .job_result
                        .as_ref()
                        .and_then(|jr| jr.summary.clone()),
                    full_response: step_result.full_response.clone(),
                    error: step_result
                        .agent_result
                        .as_ref()
                        .and_then(|ar| ar.error.clone()),
                    files_changed: step_result
                        .agent_result
                        .as_ref()
                        .map(|ar| ar.files_changed)
                        .unwrap_or(0),
                };
                step_history.push(summary);

                if step_result.skipped {
                    combined_details.push(format!("[{}] skipped", step_result.mode));
                } else if let Some(jr) = &step_result.job_result {
                    if let Some(title) = &jr.title {
                        combined_details.push(format!("[{}] {}", step_result.mode, title));
                    }
                    if let Some(ar) = &step_result.agent_result {
                        total_files_changed += ar.files_changed;
                    }
                }
                // Note: ChainStepCompleted events are sent in real-time by progress_forwarder,
                // so we don't send them again here to avoid duplicates
            }

            // Store chain step history in job
            j.chain_step_history = step_history;
            j.chain_current_step = Some(total_steps); // Mark as completed

            // Set the combined result
            j.result = Some(JobResult {
                title: Some(format!("Chain '{}' completed", chain_name)),
                commit_subject: None,
                commit_body: None,
                details: Some(combined_details.join("\n")),
                status: Some(
                    if chain_result.success {
                        "success"
                    } else {
                        "partial"
                    }
                    .to_string(),
                ),
                summary: Some(chain_result.accumulated_summaries.join("\n\n")),
                state: chain_result.final_state.clone(),
                next_context: None,
                raw_text: None,
            });

            j.set_file_stats(total_files_changed, 0, 0);

            if chain_result.success {
                j.set_status(JobStatus::Done);
                let _ = event_tx.send(ExecutorEvent::JobCompleted(job_id));
            } else {
                j.set_status(JobStatus::Failed);
                j.error_message = Some("Chain execution failed".to_string());
                let _ = event_tx.send(ExecutorEvent::JobFailed(
                    job_id,
                    "Chain execution failed".to_string(),
                ));
            }
        }
    }

    // Send chain completion event
    let _ = event_tx.send(ExecutorEvent::ChainCompleted {
        job_id,
        chain_name: chain_name.clone(),
        steps_executed: chain_result
            .step_results
            .iter()
            .filter(|r| !r.skipped)
            .count(),
        success: chain_result.success,
    });

    // Wait for log forwarder
    let _ = log_forwarder.await;

    // Release file locks held by this job
    {
        let mut manager = job_manager.lock().unwrap();
        manager.release_job_locks(job_id);
    }
}
