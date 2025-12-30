//! Chain job execution logic

mod worktree;

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::agent::{AgentRegistry, ChainProgressEvent, ChainRunner, ChainStepResult};
use crate::config::Config;
use crate::git::GitManager;
use crate::job::JobManager;
use crate::{ChainStepSummary, Job, JobResult, JobStatus, LogEvent};

use super::log_forwarder::spawn_log_forwarder;
use super::ExecutorEvent;
use worktree::setup_chain_worktree;

/// Convert a ChainStepResult to a ChainStepSummary (clones string fields)
fn step_result_to_summary(step_result: &ChainStepResult) -> ChainStepSummary {
    ChainStepSummary {
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
    }
}

/// Run a job that is actually a chain of modes
pub async fn run_chain_job(
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

    let chain = match config.get_chain(&chain_name) {
        Some(c) => c.clone(),
        None => {
            let error = format!("Chain '{}' not found", chain_name);
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(&error)));
            if let Ok(mut manager) = job_manager.lock() {
                if let Some(j) = manager.get_mut(job_id) {
                    j.fail(&error);
                }
                manager.touch();
            }
            let _ = event_tx.send(ExecutorEvent::JobFailed(job_id, error));
            return;
        }
    };

    {
        let Ok(mut manager) = job_manager.lock() else {
            let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                "Chain job #{} failed: lock poisoned",
                job_id
            ))));
            return;
        };
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

    // Multi-agent jobs always require worktrees for isolation
    let is_multi_agent_job = job.group_id.is_some();
    let should_use_worktree =
        config.settings.use_worktree || is_multi_agent_job || job.force_worktree;

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
        match setup_chain_worktree(
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
            None => return,
        }
    } else {
        (job_work_dir.clone(), false)
    };

    let (log_tx, log_rx) = tokio::sync::mpsc::channel::<LogEvent>(100);
    let log_forwarder = spawn_log_forwarder(log_rx, event_tx.clone(), Arc::clone(job_manager), job_id);

    let chain_runner = ChainRunner::new(config, agent_registry, &worktree_path);
    let (progress_tx, progress_rx) = std::sync::mpsc::channel::<ChainProgressEvent>();
    let event_tx_progress = event_tx.clone();
    let job_manager_progress = Arc::clone(job_manager);
    let progress_job_id = job_id;
    let total_steps_for_progress = chain.steps.len();
    let progress_forwarder = tokio::spawn(async move {
        while let Ok(progress) = progress_rx.recv() {
            if let Ok(mut manager) = job_manager_progress.lock() {
                if let Some(j) = manager.get_mut(progress_job_id) {
                    if progress.is_starting {
                        j.chain_current_step = Some(progress.step_index);
                    } else {
                        j.chain_current_step = Some(progress.step_index + 1);
                        if let Some(step_result) = &progress.step_result {
                            let summary = step_result_to_summary(step_result);
                            let state = step_result
                                .job_result
                                .as_ref()
                                .and_then(|jr| jr.state.clone());
                            // Clone mode before potentially cloning summary
                            let mode = summary.mode.clone();
                            // Only clone summary if we need it for history
                            let step_summary = if j.chain_step_history.len() <= step_result.step_index {
                                let clone = summary.clone();
                                j.chain_step_history.push(summary);
                                clone
                            } else {
                                summary
                            };
                            let _ = event_tx_progress.send(ExecutorEvent::ChainStepCompleted {
                                job_id: progress_job_id,
                                step_index: step_result.step_index,
                                total_steps: total_steps_for_progress,
                                mode,
                                state,
                                step_summary,
                            });
                        }
                    }
                }
            }
        }
    });

    let chain_result = chain_runner
        .run_chain(&chain_name, &chain, &job, log_tx, Some(progress_tx))
        .await;

    tokio::time::sleep(Duration::from_millis(50)).await;
    progress_forwarder.abort();
    let _ = progress_forwarder.await;

    let total_steps = chain_result.step_results.len();
    if let Ok(mut manager) = job_manager.lock() {
        if let Some(j) = manager.get_mut(job_id) {
            let mut combined_details = Vec::new();
            let mut total_files_changed = 0;
            let mut step_history = Vec::new();

            for step_result in &chain_result.step_results {
                let summary = step_result_to_summary(step_result);

                if step_result.skipped {
                    combined_details.push(format!("[{}] skipped", summary.mode));
                } else {
                    if let Some(title) = &summary.title {
                        combined_details.push(format!("[{}] {}", summary.mode, title));
                    }
                    total_files_changed += summary.files_changed;
                }

                step_history.push(summary);
            }

            j.chain_step_history = step_history;
            j.chain_current_step = Some(total_steps);

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
        manager.touch();
    }

    let _ = event_tx.send(ExecutorEvent::ChainCompleted {
        job_id,
        chain_name,
        steps_executed: chain_result
            .step_results
            .iter()
            .filter(|r| !r.skipped)
            .count(),
        success: chain_result.success,
    });

    let _ = log_forwarder.await;

    if let Ok(mut manager) = job_manager.lock() {
        manager.release_job_locks(job_id);
    }
}
