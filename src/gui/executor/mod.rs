//! Job executor for the GUI
//!
//! Runs in a background thread and processes queued jobs

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::agent::{AgentRegistry, ChainRunner};
use crate::config::Config;
use crate::git::GitManager;
use crate::job::JobManager;
use crate::{AgentMode, Job, JobResult, JobStatus, LogEvent};

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
}

/// Start the job executor in a background thread
pub fn start_executor(
    work_dir: PathBuf,
    config: Config,
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
    config: Config,
    job_manager: Arc<Mutex<JobManager>>,
    event_tx: Sender<ExecutorEvent>,
    max_concurrent_jobs: Arc<AtomicUsize>,
) {
    let agent_registry = AgentRegistry::with_defaults();

    // Try to get git manager
    let git_manager = GitManager::new(&work_dir).ok();

    loop {
        // Check for queued jobs
        let (running_count, next_queued) = {
            let manager = job_manager.lock().unwrap();
            let running = manager
                .jobs()
                .iter()
                .filter(|j| j.status == JobStatus::Running)
                .count();
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
                // Spawn job in background so we can start multiple jobs concurrently
                let work_dir = work_dir.clone();
                let config = config.clone();
                let job_manager = Arc::clone(&job_manager);
                let agent_registry = agent_registry.clone();
                let git_manager = git_manager.clone();
                let event_tx = event_tx.clone();

                tokio::spawn(async move {
                    run_job(
                        &work_dir,
                        &config,
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
    job: Job,
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
    let should_use_worktree = config.settings.use_worktree || is_multi_agent_job || job.force_worktree;

    let (worktree_path, _is_isolated) = if should_use_worktree {
        if let Some(git) = git_manager {
            match git.create_worktree(job_id) {
                Ok(worktree_info) => {
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                        "Created worktree: {}",
                        worktree_info.path.display()
                    ))));
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
                    (work_dir.clone(), false)
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
            (work_dir.clone(), false)
        }
    } else {
        (work_dir.clone(), false)
    };

    // Remove tag from source file before running agent
    if let Err(e) = remove_tag_from_source(&job, work_dir, &worktree_path, &config.settings.marker_prefix) {
        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
            "Failed to remove tag: {}",
            e
        ))));
        // Continue anyway
    }

    // Get agent config with mode-specific tool overrides
    let agent_config = config
        .get_agent_for_job(&job.agent_id, &job.mode)
        .unwrap_or_default();

    // Mark whether this is a REPL job (for UI to show correct buttons)
    let is_repl = agent_config.mode == AgentMode::Repl;
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
            let error = format!(
                "No adapter found for agent '{}'",
                job.agent_id
            );
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
    let log_forwarder = tokio::spawn(async move {
        while let Some(log) = log_rx.recv().await {
            let _ = event_tx_clone.send(ExecutorEvent::Log(log));
        }
    });

    // Run the adapter
    match adapter.run(&job, &worktree_path, &agent_config, log_tx).await {
        Ok(result) => {
            let mut manager = job_manager.lock().unwrap();
            if let Some(j) = manager.get_mut(job_id) {
                j.sent_prompt = result.sent_prompt.clone();

                // Parse the output for ---kyco result block
                if let Some(output) = &result.output_text {
                    j.parse_result(output);
                }

                // Calculate file stats from changed_files
                let files_changed = result.changed_files.len();
                if files_changed > 0 {
                    // TODO: Calculate lines_added/removed from git diff
                    j.set_file_stats(files_changed, 0, 0);
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
}

/// Remove the tag from the source file before running the agent
fn remove_tag_from_source(
    job: &Job,
    work_dir: &PathBuf,
    worktree_path: &PathBuf,
    marker_prefix: &str,
) -> anyhow::Result<()> {
    let Some(raw_tag_line) = &job.raw_tag_line else {
        return Ok(());
    };

    let relative_path = job.source_file.strip_prefix(work_dir).unwrap_or(&job.source_file);
    let target_file = worktree_path.join(relative_path);

    if !target_file.exists() {
        anyhow::bail!("Source file not found: {}", target_file.display());
    }

    let content = std::fs::read_to_string(&target_file)?;
    let trimmed_tag = raw_tag_line.trim();

    // Check if standalone comment or inline
    let is_standalone = trimmed_tag.starts_with("//")
        || trimmed_tag.starts_with('#')
        || trimmed_tag.starts_with("/*")
        || trimmed_tag.starts_with("--")
        || trimmed_tag.starts_with(marker_prefix);

    let has_trailing_newline = content.ends_with('\n');

    let mut new_content = String::with_capacity(content.len());
    let mut first_line = true;

    for line in content.lines() {
        let should_skip = is_standalone && line.trim() == trimmed_tag;

        if should_skip {
            continue;
        }

        if !first_line {
            new_content.push('\n');
        }
        first_line = false;

        if !is_standalone && (line == raw_tag_line || line.trim() == trimmed_tag) {
            // Inline: remove just the tag comment part, keep the code
            if let Some(marker_pos) = line.find(marker_prefix) {
                let before_marker = &line[..marker_pos];
                let comment_start = before_marker
                    .rfind("//")
                    .or_else(|| before_marker.rfind('#'))
                    .or_else(|| before_marker.rfind("--"))
                    .or_else(|| before_marker.rfind("/*"));

                if let Some(start) = comment_start {
                    new_content.push_str(line[..start].trim_end());
                    continue;
                }
            }
        }

        new_content.push_str(line);
    }

    if has_trailing_newline {
        new_content.push('\n');
    }

    std::fs::write(&target_file, new_content)?;
    Ok(())
}

/// Run a job that is actually a chain of modes
async fn run_chain_job(
    work_dir: &PathBuf,
    config: &Config,
    job_manager: &Arc<Mutex<JobManager>>,
    agent_registry: &AgentRegistry,
    git_manager: Option<&GitManager>,
    event_tx: &Sender<ExecutorEvent>,
    job: Job,
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

    // Update status to running
    {
        let mut manager = job_manager.lock().unwrap();
        manager.set_status(job_id, JobStatus::Running);
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
    let should_use_worktree = config.settings.use_worktree || is_multi_agent_job;

    let (worktree_path, _is_isolated) = if should_use_worktree {
        if let Some(git) = git_manager {
            match git.create_worktree(job_id) {
                Ok(worktree_info) => {
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                        "Created worktree: {}",
                        worktree_info.path.display()
                    ))));
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
                    // For multi-agent jobs, worktree creation failure is fatal
                    if is_multi_agent_job {
                        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                            "Multi-agent chain job requires worktree but creation failed: {}",
                            e
                        ))));
                        let _ = event_tx.send(ExecutorEvent::JobFailed(
                            job_id,
                            format!("Worktree required for parallel execution: {}", e),
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
                    (work_dir.clone(), false)
                }
            }
        } else {
            // No git manager available
            if is_multi_agent_job {
                let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(
                    "Multi-agent chain job requires git repository for worktree isolation".to_string(),
                )));
                let _ = event_tx.send(ExecutorEvent::JobFailed(
                    job_id,
                    "Git repository required for parallel execution".to_string(),
                ));
                {
                    let mut manager = job_manager.lock().unwrap();
                    manager.set_status(job_id, JobStatus::Failed);
                    if let Some(j) = manager.get_mut(job_id) {
                        j.error_message = Some("Git repository required for parallel execution".to_string());
                    }
                }
                return;
            }
            (work_dir.clone(), false)
        }
    } else {
        (work_dir.clone(), false)
    };

    // Remove tag from source file before running
    if let Err(e) = remove_tag_from_source(&job, work_dir, &worktree_path, &config.settings.marker_prefix) {
        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
            "Failed to remove tag: {}",
            e
        ))));
    }

    // Create log channel
    let (log_tx, mut log_rx) = tokio::sync::mpsc::channel::<LogEvent>(100);

    // Forward logs to event_tx
    let event_tx_clone = event_tx.clone();
    let log_forwarder = tokio::spawn(async move {
        while let Some(log) = log_rx.recv().await {
            let _ = event_tx_clone.send(ExecutorEvent::Log(log));
        }
    });

    // Create chain runner
    let chain_runner = ChainRunner::new(config, agent_registry, &worktree_path);

    // Run the chain
    let chain_result = chain_runner
        .run_chain(&chain_name, &chain, &job, log_tx)
        .await;

    // Update job with results
    {
        let mut manager = job_manager.lock().unwrap();
        if let Some(j) = manager.get_mut(job_id) {
            // Combine results from all steps
            let mut combined_details = Vec::new();
            let mut total_files_changed = 0;

            for step_result in &chain_result.step_results {
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
            }

            // Set the combined result
            j.result = Some(JobResult {
                title: Some(format!("Chain '{}' completed", chain_name)),
                details: Some(combined_details.join("\n")),
                status: Some(if chain_result.success { "success" } else { "partial" }.to_string()),
                summary: Some(chain_result.accumulated_summaries.join("\n\n")),
                state: chain_result.final_state.clone(),
                next_context: None,
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
        steps_executed: chain_result.step_results.iter().filter(|r| !r.skipped).count(),
        success: chain_result.success,
    });

    // Wait for log forwarder
    let _ = log_forwarder.await;
}
