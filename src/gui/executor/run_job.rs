//! Single job execution logic

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use crate::agent::AgentRegistry;
use crate::config::Config;
use crate::git::GitManager;
use crate::job::JobManager;
use crate::{Job, JobStatus, LogEvent, SessionMode};

use super::chain::run_chain_job;
use super::git_utils::calculate_git_numstat;
use super::log_forwarder::spawn_log_forwarder;
use super::ExecutorEvent;

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

    let job_work_dir = job
        .workspace_path
        .clone()
        .unwrap_or_else(|| work_dir.clone());

    let job_git_manager =
        if job.workspace_path.is_some() && job.workspace_path.as_ref() != Some(work_dir) {
            GitManager::new(&job_work_dir).ok()
        } else {
            None
        };
    let effective_git_manager = job_git_manager.as_ref().or(git_manager);

    let (worktree_path, _is_isolated) = if let Some(existing_worktree) =
        job.git_worktree_path.as_ref().filter(|p| p.exists())
    {
        let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
            "Reusing worktree: {}",
            existing_worktree.display()
        ))));
        (existing_worktree.clone(), true)
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
        (job_work_dir.clone(), false)
    };

    let agent_config = config
        .get_agent_for_job(&job.agent_id, &job.mode)
        .unwrap_or_default();

    let is_repl = matches!(agent_config.session_mode, SessionMode::Repl);
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
    let log_forwarder = spawn_log_forwarder(log_rx, event_tx.clone(), Arc::clone(job_manager), job_id);

    match adapter
        .run(&job, &worktree_path, &agent_config, log_tx)
        .await
    {
        Ok(result) => {
            let Ok(mut manager) = job_manager.lock() else {
                let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                    "Job #{} completed but lock poisoned",
                    job_id
                ))));
                return;
            };
            if let Some(j) = manager.get_mut(job_id) {
                j.sent_prompt = result.sent_prompt.clone();

                // Copy token usage from agent result
                j.input_tokens = result.input_tokens;
                j.output_tokens = result.output_tokens;
                j.cache_read_tokens = result.cache_read_tokens;
                j.cache_write_tokens = result.cache_write_tokens;
                j.cost_usd = result.cost_usd;

                if let Some(output) = &result.output_text {
                    j.full_response = Some(output.clone());
                    j.parse_result(output);
                }

                if result.session_id.is_some() {
                    j.bridge_session_id = result.session_id.clone();
                }

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
            manager.touch();
        }
        Err(e) => {
            let error = e.to_string();
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(error.clone());
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

    let _ = log_forwarder.await;

    if let Ok(mut manager) = job_manager.lock() {
        manager.release_job_locks(job_id);
    }
}

/// Setup worktree for a job, returning (worktree_path, is_isolated) or None if failed and required.
fn setup_worktree(
    git_manager: Option<&GitManager>,
    job_id: u64,
    is_multi_agent_job: bool,
    force_worktree: bool,
    job_work_dir: &PathBuf,
    event_tx: &Sender<ExecutorEvent>,
    job_manager: &Arc<Mutex<JobManager>>,
    job: &mut Job,
) -> Option<(PathBuf, bool)> {
    if let Some(git) = git_manager {
        match git.create_worktree(job_id) {
            Ok(worktree_info) => {
                let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                    "Created worktree: {}",
                    worktree_info.path.display()
                ))));
                job.git_worktree_path = Some(worktree_info.path.clone());
                job.base_branch = Some(worktree_info.base_branch.clone());
                job.branch_name = Some(worktree_info.branch_name.clone());
                if let Ok(mut manager) = job_manager.lock() {
                    if let Some(j) = manager.get_mut(job_id) {
                        j.git_worktree_path = Some(worktree_info.path.clone());
                        j.base_branch = Some(worktree_info.base_branch);
                        j.branch_name = Some(worktree_info.branch_name);
                    }
                    // Notify GUI that job data changed (worktree path is now set)
                    manager.touch();
                }
                Some((worktree_info.path, true))
            }
            Err(e) => {
                if is_multi_agent_job || force_worktree {
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
                    if let Ok(mut manager) = job_manager.lock() {
                        manager.set_status(job_id, JobStatus::Failed);
                        if let Some(j) = manager.get_mut(job_id) {
                            j.error_message = Some(format!("Worktree creation failed: {}", e));
                        }
                    }
                    return None;
                }
                let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                    "Failed to create worktree: {}",
                    e
                ))));
                Some((job_work_dir.clone(), false))
            }
        }
    } else if is_multi_agent_job || force_worktree {
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
        if let Ok(mut manager) = job_manager.lock() {
            manager.set_status(job_id, JobStatus::Failed);
            if let Some(j) = manager.get_mut(job_id) {
                j.error_message = Some(format!("Git repository required for {}", reason));
            }
        }
        None
    } else {
        Some((job_work_dir.clone(), false))
    }
}
