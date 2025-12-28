//! Worktree setup for chain jobs

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use crate::git::GitManager;
use crate::job::JobManager;
use crate::{Job, JobStatus, LogEvent};

use crate::gui::executor::ExecutorEvent;

/// Setup worktree for a chain job, returning (worktree_path, is_isolated) or None if failed and required.
pub(super) fn setup_chain_worktree(
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
                if let Ok(mut manager) = job_manager.lock() {
                    if let Some(j) = manager.get_mut(job_id) {
                        j.git_worktree_path = Some(worktree_info.path.clone());
                        j.base_branch = Some(worktree_info.base_branch);
                    }
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
