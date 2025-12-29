//! Job operations: CRUD and state management
//!
//! This module contains job lifecycle operations:
//! - Refresh jobs from manager
//! - Create jobs from selection
//! - Queue, apply, and reject jobs
//! - Multi-agent job creation

mod creation;

pub use creation::{create_job_from_selection, create_jobs_from_selection_multi, CreateJobsResult};

use crate::job::JobManager;
use crate::{Job, JobId, JobStatus, LogEvent};
use std::sync::{Arc, Mutex};

/// Refresh cached jobs from JobManager.
/// Returns (jobs, generation) tuple for change detection.
pub fn refresh_jobs(job_manager: &Arc<Mutex<JobManager>>) -> (Vec<Job>, u64) {
    if let Ok(manager) = job_manager.lock() {
        let generation = manager.generation();
        let jobs = manager.jobs().into_iter().cloned().collect();
        (jobs, generation)
    } else {
        (Vec::new(), 0)
    }
}

/// Check if jobs have changed since last refresh.
/// Returns Some(generation) if changed, None if same.
pub fn check_jobs_changed(
    job_manager: &Arc<Mutex<JobManager>>,
    last_generation: u64,
) -> Option<u64> {
    if let Ok(manager) = job_manager.lock() {
        let current = manager.generation();
        if current != last_generation {
            Some(current)
        } else {
            None
        }
    } else {
        None
    }
}

/// Queue a job for execution
pub fn queue_job(job_manager: &Arc<Mutex<JobManager>>, job_id: JobId, logs: &mut Vec<LogEvent>) {
    if let Ok(mut manager) = job_manager.lock() {
        manager.set_status(job_id, JobStatus::Queued);
        logs.push(LogEvent::system(format!("Queued job #{}", job_id)));
    }
}

/// Apply job changes (merge worktree to main)
pub fn apply_job(job_manager: &Arc<Mutex<JobManager>>, job_id: JobId, logs: &mut Vec<LogEvent>) {
    if let Ok(mut manager) = job_manager.lock() {
        manager.set_status(job_id, JobStatus::Merged);
        logs.push(LogEvent::system(format!("Applied job #{}", job_id)));
    }
}

/// Reject job changes
pub fn reject_job(job_manager: &Arc<Mutex<JobManager>>, job_id: JobId, logs: &mut Vec<LogEvent>) {
    if let Ok(mut manager) = job_manager.lock() {
        manager.set_status(job_id, JobStatus::Rejected);
        logs.push(LogEvent::system(format!("Rejected job #{}", job_id)));
    }
}

/// Kill/stop a running job
///
/// Marks the job as "cancel requested" so the executor can interrupt the active
/// Bridge session as soon as the session ID is known.
pub fn kill_job(job_manager: &Arc<Mutex<JobManager>>, job_id: JobId, logs: &mut Vec<LogEvent>) {
    if let Ok(mut manager) = job_manager.lock() {
        if let Some(job) = manager.get_mut(job_id) {
            if job.status == JobStatus::Running {
                job.cancel_requested = true;
                logs.push(LogEvent::system(format!(
                    "Stop requested for job #{}",
                    job_id
                )));
                manager.touch();
            }
        }
    }
}

/// Mark a legacy REPL job as complete
///
/// Legacy: REPL jobs used to run in a separate Terminal.app window.
pub fn mark_job_complete(
    job_manager: &Arc<Mutex<JobManager>>,
    job_id: JobId,
    logs: &mut Vec<LogEvent>,
) {
    if let Some(session) = crate::agent::get_terminal_session(job_id) {
        session.mark_completed();
    }

    if let Ok(mut manager) = job_manager.lock() {
        manager.set_status(job_id, JobStatus::Done);
        logs.push(LogEvent::system(format!(
            "Marked job #{} as complete",
            job_id
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CommentTag;
    use tempfile::tempdir;

    #[test]
    fn kill_job_marks_cancel_requested_without_failing() {
        let tmp = tempdir().expect("tempdir");
        let file_path = tmp.path().join("src").join("main.rs");
        std::fs::create_dir_all(file_path.parent().unwrap()).expect("mkdir");
        std::fs::write(&file_path, "fn main() {}\n").expect("write");

        let tag = CommentTag::new_simple(
            file_path.clone(),
            1,
            "// @claude#refactor".to_string(),
            "claude".to_string(),
            "refactor".to_string(),
        );

        let mut manager = JobManager::new(tmp.path());
        let job_id = manager.create_job(&tag, "claude").expect("create_job");
        manager.set_status(job_id, JobStatus::Running);

        let job_manager = Arc::new(Mutex::new(manager));
        let mut logs = Vec::new();

        kill_job(&job_manager, job_id, &mut logs);

        let guard = job_manager.lock().expect("lock");
        let job = guard.get(job_id).expect("job exists");
        assert_eq!(job.status, JobStatus::Running);
        assert!(job.cancel_requested);
        assert!(!job.cancel_sent);
        assert!(logs.iter().any(|l| l.summary.contains("Stop requested")));
    }
}
