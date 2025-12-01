//! Job operations: CRUD and state management
//!
//! This module contains job lifecycle operations:
//! - Refresh jobs from manager
//! - Create jobs from selection
//! - Queue, apply, and reject jobs

use super::super::selection::SelectionContext;
use crate::job::JobManager;
use crate::{CommentTag, Job, JobId, JobStatus, LogEvent, Target};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Refresh cached jobs from JobManager
pub fn refresh_jobs(job_manager: &Arc<Mutex<JobManager>>) -> Vec<Job> {
    if let Ok(manager) = job_manager.lock() {
        manager.jobs().into_iter().cloned().collect()
    } else {
        Vec::new()
    }
}

/// Create a job from the selection popup
pub fn create_job_from_selection(
    job_manager: &Arc<Mutex<JobManager>>,
    selection: &SelectionContext,
    agent: &str,
    mode: &str,
    prompt: &str,
    logs: &mut Vec<LogEvent>,
) -> Option<JobId> {
    let file_path = selection.file_path.clone()?;
    let line_number = selection.line_number.unwrap_or(1);
    let line_end = selection.line_end;

    let tag = CommentTag {
        file_path: PathBuf::from(&file_path),
        line_number,
        raw_line: format!("// @{}:{} {}", agent, mode, prompt),
        agent: agent.to_string(),
        mode: mode.to_string(),
        target: Target::Block,
        status_marker: None,
        description: if prompt.is_empty() {
            None
        } else {
            Some(prompt.to_string())
        },
        job_id: None,
    };

    if let Ok(mut manager) = job_manager.lock() {
        match manager.create_job_with_range(&tag, agent, line_end) {
            Ok(job_id) => {
                logs.push(LogEvent::system(format!("Created job #{}", job_id)));
                return Some(job_id);
            }
            Err(e) => {
                logs.push(LogEvent::error(format!("Failed to create job: {}", e)));
            }
        }
    }
    None
}

/// Queue a job for execution
pub fn queue_job(
    job_manager: &Arc<Mutex<JobManager>>,
    job_id: JobId,
    logs: &mut Vec<LogEvent>,
) {
    if let Ok(mut manager) = job_manager.lock() {
        manager.set_status(job_id, JobStatus::Queued);
        logs.push(LogEvent::system(format!("Queued job #{}", job_id)));
    }
}

/// Apply job changes (merge worktree to main)
pub fn apply_job(
    job_manager: &Arc<Mutex<JobManager>>,
    job_id: JobId,
    logs: &mut Vec<LogEvent>,
) {
    if let Ok(mut manager) = job_manager.lock() {
        manager.set_status(job_id, JobStatus::Merged);
        logs.push(LogEvent::system(format!("Applied job #{}", job_id)));
    }
}

/// Reject job changes
pub fn reject_job(
    job_manager: &Arc<Mutex<JobManager>>,
    job_id: JobId,
    logs: &mut Vec<LogEvent>,
) {
    if let Ok(mut manager) = job_manager.lock() {
        manager.set_status(job_id, JobStatus::Rejected);
        logs.push(LogEvent::system(format!("Rejected job #{}", job_id)));
    }
}
