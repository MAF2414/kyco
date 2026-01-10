//! Single job execution logic

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use crate::agent::AgentRegistry;
use crate::config::Config;
use crate::git::GitManager;
use crate::job::JobManager;
use crate::{Job, JobStatus, LogEvent};

use super::ExecutorEvent;
use super::JobLockGuard;
use super::chain::run_chain_job;
use super::git_utils::calculate_git_numstat_async;
use super::log_forwarder::spawn_log_forwarder;
use super::worktree_paths::remap_job_paths_to_worktree;
use super::worktree_setup::setup_worktree;

/// Run a single job (non-chain)
pub async fn run_job(
    work_dir: &PathBuf,
    config: &Config,
    job_manager: &Arc<Mutex<JobManager>>,
    agent_registry: &AgentRegistry,
    git_manager: Option<&GitManager>,
    event_tx: &Sender<ExecutorEvent>,
    mut job: Job,
) {
    let job_id = job.id;
    let _job_locks = JobLockGuard::new(Arc::clone(job_manager), job_id);

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

    // Validate job inputs before marking as running.
    if job.source_line == 0 {
        let error = "Invalid job input: source_line must be >= 1".to_string();
        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(error.clone())));
        if let Ok(mut manager) = job_manager.lock() {
            if let Some(j) = manager.get_mut(job_id) {
                j.fail(error.clone());
            }
            manager.touch();
        }
        let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
        return;
    }

    let workspace_root = job.workspace_path.as_ref().unwrap_or(work_dir);
    let resolved_source_file = if job.source_file.is_absolute() {
        job.source_file.clone()
    } else {
        workspace_root.join(&job.source_file)
    };

    // Check if this is a prompt-only job (source_file equals workspace root)
    // Prompt-only jobs have no specific source file - they use the workspace as a placeholder
    let is_prompt_only_job =
        resolved_source_file == *workspace_root || job.source_file.to_string_lossy() == "prompt";

    // Only validate source file existence if it's not a prompt-only job
    if !is_prompt_only_job {
        if !resolved_source_file.exists() {
            let error = format!("Source file not found: {}", resolved_source_file.display());
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(error.clone())));
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(error.clone());
                }
                manager.touch();
            }
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
            return;
        }
        if !resolved_source_file.is_file() {
            let error = format!(
                "Invalid job input: source file is not a file: {}",
                resolved_source_file.display()
            );
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(error.clone())));
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(error.clone());
                }
                manager.touch();
            }
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
            return;
        }
    }
    if !job.source_file.is_absolute() {
        job.source_file = resolved_source_file;
        if let Ok(mut manager) = job_manager.lock() {
            if let Some(j) = manager.get_mut(job_id) {
                j.source_file = job.source_file.clone();
            }
            manager.touch();
        }
    }

    {
        let Ok(mut manager) = job_manager.lock() else {
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                "Job #{} failed: lock poisoned",
                job_id
            ))));
            return;
        };
        manager.set_status(job_id, JobStatus::Running);
    }

    let _ = event_tx.send(ExecutorEvent::JobStarted(job_id));
    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
        "Starting job #{}",
        job_id
    ))));

    // Multi-agent jobs always require worktrees for isolation
    let is_multi_agent_job = job.group_id.is_some();

    // Check if the mode/chain has a use_worktree override
    let mode_use_worktree = config
        .mode
        .get(&job.mode)
        .and_then(|m| m.use_worktree)
        .or_else(|| config.chain.get(&job.mode).and_then(|c| c.use_worktree));

    let should_use_worktree = match mode_use_worktree {
        Some(true) => true,   // Mode/chain explicitly enables worktree
        Some(false) => false, // Mode/chain explicitly disables worktree
        None => config.settings.use_worktree || is_multi_agent_job || job.force_worktree,
    };

    // Check if we have a custom workspace different from work_dir (before taking ownership)
    let has_custom_workspace = job.workspace_path.as_ref().is_some_and(|p| p != work_dir);
    // Take ownership of workspace_path or clone work_dir
    let job_work_dir = job
        .workspace_path
        .take()
        .unwrap_or_else(|| work_dir.clone());
    let workspace_root = job_work_dir.clone();

    let job_git_manager = if has_custom_workspace {
        GitManager::new(&job_work_dir).ok()
    } else {
        None
    };
    let effective_git_manager = job_git_manager.as_ref().or(git_manager);

    // Take existing worktree path if it exists and is valid (avoid clone by taking ownership)
    let (worktree_path, is_in_worktree) =
        if let Some(existing_worktree) = job.git_worktree_path.take().filter(|p| p.exists()) {
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                "Reusing worktree: {}",
                existing_worktree.display()
            ))));
            (existing_worktree, true)
        } else if should_use_worktree {
            match setup_worktree(
                effective_git_manager,
                job_id,
                is_multi_agent_job,
                job.force_worktree,
                &job_work_dir,
                event_tx,
                job_manager,
                &mut job,
            ) {
                Some(result) => result,
                None => return, // Early return on required worktree failure
            }
        } else {
            // No worktree needed - move job_work_dir instead of cloning
            (job_work_dir, false)
        };

    if is_in_worktree {
        let remap = remap_job_paths_to_worktree(&mut job, &workspace_root, &worktree_path);
        if remap.copied_source_file {
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(
                "Copied source file into worktree to preserve isolation",
            )));
        }
        if remap.remapped {
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.source_file = job.source_file.clone();
                    j.scope = job.scope.clone();
                    j.target.clone_from(&job.target);
                }
                manager.touch();
            }
        }
    }

    let mut agent_config = config
        .get_agent_for_job(&job.agent_id, &job.mode)
        .unwrap_or_default();

    // When using a worktree, automatically allow git commands for committing
    if is_in_worktree {
        let git_tools = [
            "git",
            "Bash(git:*)",
            "Bash(git add:*)",
            "Bash(git commit:*)",
            "Bash(git status:*)",
            "Bash(git diff:*)",
            "Bash(git log:*)",
        ];
        for tool in git_tools {
            let tool_str = tool.to_string();
            if !agent_config.allowed_tools.contains(&tool_str) {
                agent_config.allowed_tools.push(tool_str);
            }
        }
    }

    // All agents now use persistent sessions (SessionMode removed)
    let is_repl = true;
    if let Ok(mut manager) = job_manager.lock() {
        if let Some(j) = manager.get_mut(job_id) {
            j.is_repl = is_repl;
        }
    }

    let adapter = match agent_registry.get_for_config(&agent_config) {
        Some(a) => a,
        None => {
            let error = format!("No adapter found for agent '{}'", job.agent_id);
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(error.clone())));
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(error.clone());
                }
                manager.touch();
            }
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
            return;
        }
    };

    let (log_tx, log_rx) = tokio::sync::mpsc::channel::<LogEvent>(100);
    let log_forwarder =
        spawn_log_forwarder(log_rx, event_tx.clone(), Arc::clone(job_manager), job_id);

    // Track git stats info for async calculation after lock release
    let mut git_stats_info: Option<(usize, Option<String>)> = None;

    match adapter
        .run(&job, &worktree_path, &agent_config, log_tx)
        .await
    {
        Ok(mut result) => {
            let Ok(mut manager) = job_manager.lock() else {
                let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                    "Job #{} completed but lock poisoned",
                    job_id
                ))));
                return;
            };
            if let Some(j) = manager.get_mut(job_id) {
                let was_cancel_requested = j.cancel_requested;
                // Use take() to move values instead of cloning
                j.sent_prompt = result.sent_prompt.take();

                // Copy token usage from agent result (primitives, no allocation)
                j.input_tokens = result.input_tokens;
                j.output_tokens = result.output_tokens;
                j.cache_read_tokens = result.cache_read_tokens;
                j.cache_write_tokens = result.cache_write_tokens;
                j.cost_usd = result.cost_usd;

                // Take output_text to avoid clone; parse_result only needs a reference
                if let Some(output) = result.output_text.take() {
                    j.parse_result(&output);
                    j.full_response = Some(output);
                }

                // Move session_id instead of cloning
                j.bridge_session_id = result.session_id.take();

                // Restore worktree path so continuation jobs can reuse it
                // (it was taken earlier with .take() to avoid cloning)
                if is_in_worktree {
                    j.git_worktree_path = Some(worktree_path.clone());
                }

                let files_changed = result.changed_files.len();
                // Store info for async git stats calculation after lock release
                if files_changed > 0 && is_in_worktree {
                    git_stats_info = Some((files_changed, j.base_branch.clone()));
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
                    let error = if was_cancel_requested {
                        "Job aborted by user".to_string()
                    } else {
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    };
                    j.fail(error.clone());
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                        "Job #{} failed: {}",
                        job_id, error
                    ))));
                    let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
                }
            }
            manager.touch();
        }
        Err(e) => {
            let mut error = e.to_string();
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    if j.cancel_requested {
                        error = "Job aborted by user".to_string();
                    }
                    j.fail(error.clone());
                    // Restore worktree path for potential retry/continuation
                    if is_in_worktree {
                        j.git_worktree_path = Some(worktree_path.clone());
                    }
                }
                manager.touch();
            }
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                "Job #{} error: {}",
                job_id, error
            ))));
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
        }
    }

    // Calculate git stats asynchronously after releasing the lock
    // This avoids blocking the async runtime with synchronous git operations
    if let Some((files_changed, base_branch)) = git_stats_info {
        let (lines_added, lines_removed) =
            calculate_git_numstat_async(&worktree_path, base_branch.as_deref()).await;

        // Re-acquire lock to update file stats
        if let Ok(mut manager) = job_manager.lock() {
            if let Some(j) = manager.get_mut(job_id) {
                j.set_file_stats(files_changed, lines_added, lines_removed);
            }
            manager.touch();
        }
    }

    let _ = log_forwarder.await;
}
