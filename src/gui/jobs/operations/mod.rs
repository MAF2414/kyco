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
/// For print-mode jobs, this signals the executor to cancel the job.
/// The actual process termination happens in the executor via cancellation tokens.
pub fn kill_job(job_manager: &Arc<Mutex<JobManager>>, job_id: JobId, logs: &mut Vec<LogEvent>) {
    if let Ok(mut manager) = job_manager.lock() {
        if let Some(job) = manager.get_mut(job_id) {
            if job.status == JobStatus::Running {
                job.fail("Job stopped by user".to_string());
                logs.push(LogEvent::system(format!("Stopped job #{}", job_id)));
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
