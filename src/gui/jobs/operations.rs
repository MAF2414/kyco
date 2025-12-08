//! Job operations: CRUD and state management
//!
//! This module contains job lifecycle operations:
//! - Refresh jobs from manager
//! - Create jobs from selection
//! - Queue, apply, and reject jobs
//! - Multi-agent job creation

use super::super::selection::SelectionContext;
use crate::job::{GroupManager, JobManager};
use crate::{AgentGroupId, CommentTag, Job, JobId, JobStatus, LogEvent, Target};
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
        agents: vec![agent.to_string()],
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
                // Set IDE context if available
                if let Some(job) = manager.get_mut(job_id) {
                    let ide_context = selection.format_ide_context();
                    if !ide_context.trim().is_empty() && ide_context.lines().count() > 1 {
                        job.ide_context = Some(ide_context);
                    }
                }
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

/// Kill/stop a running job
///
/// For print-mode jobs, this signals the executor to cancel the job.
/// The actual process termination happens in the executor via cancellation tokens.
pub fn kill_job(
    job_manager: &Arc<Mutex<JobManager>>,
    job_id: JobId,
    logs: &mut Vec<LogEvent>,
) {
    if let Ok(mut manager) = job_manager.lock() {
        if let Some(job) = manager.get_mut(job_id) {
            if job.status == JobStatus::Running {
                job.fail("Job stopped by user".to_string());
                logs.push(LogEvent::system(format!("Stopped job #{}", job_id)));
            }
        }
    }
}

/// Mark a REPL job as complete
///
/// REPL jobs run in a separate Terminal.app window. The user clicks this
/// when they've finished their work in the terminal.
pub fn mark_job_complete(
    job_manager: &Arc<Mutex<JobManager>>,
    job_id: JobId,
    logs: &mut Vec<LogEvent>,
) {
    // Mark the terminal session as completed (if it exists)
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

/// Result of creating jobs (may be single or multi-agent)
pub struct CreateJobsResult {
    /// All created job IDs
    pub job_ids: Vec<JobId>,
    /// Group ID if multiple agents were used
    pub group_id: Option<AgentGroupId>,
}

/// Create jobs from selection - supports multi-agent parallel execution
///
/// If `agents` contains multiple agents, a group is created and multiple
/// jobs are spawned in parallel.
pub fn create_jobs_from_selection_multi(
    job_manager: &Arc<Mutex<JobManager>>,
    group_manager: &Arc<Mutex<GroupManager>>,
    selection: &SelectionContext,
    agents: &[String],
    mode: &str,
    prompt: &str,
    logs: &mut Vec<LogEvent>,
    force_worktree: bool,
) -> Option<CreateJobsResult> {
    let file_path = selection.file_path.clone()?;
    let line_number = selection.line_number.unwrap_or(1);
    let line_end = selection.line_end;
    let target = format!("{}:{}", file_path, line_number);

    if agents.is_empty() {
        logs.push(LogEvent::error("No agents specified".to_string()));
        return None;
    }

    // Single agent - create normally without group
    if agents.len() == 1 {
        let agent = &agents[0];
        let tag = CommentTag {
            file_path: PathBuf::from(&file_path),
            line_number,
            raw_line: format!("// @{}:{} {}", agent, mode, prompt),
            agent: agent.to_string(),
            agents: vec![agent.to_string()],
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
                    // Set IDE context and force_worktree if available
                    if let Some(job) = manager.get_mut(job_id) {
                        let ide_context = selection.format_ide_context();
                        if !ide_context.trim().is_empty() && ide_context.lines().count() > 1 {
                            job.ide_context = Some(ide_context);
                        }
                        job.force_worktree = force_worktree;
                    }
                    logs.push(LogEvent::system(format!("Created job #{}", job_id)));
                    return Some(CreateJobsResult {
                        job_ids: vec![job_id],
                        group_id: None,
                    });
                }
                Err(e) => {
                    logs.push(LogEvent::error(format!("Failed to create job: {}", e)));
                }
            }
        }
        return None;
    }

    // Multi-agent - create a group
    let group_id = if let Ok(mut gm) = group_manager.lock() {
        gm.create_group(prompt.to_string(), mode.to_string(), target.clone())
    } else {
        logs.push(LogEvent::error("Failed to acquire group manager lock".to_string()));
        return None;
    };

    let mut job_ids = Vec::new();

    for agent in agents {
        let tag = CommentTag {
            file_path: PathBuf::from(&file_path),
            line_number,
            raw_line: format!("// @{}:{} {}", agent, mode, prompt),
            agent: agent.to_string(),
            agents: agents.iter().cloned().collect(),
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
                    // Set the group_id, IDE context, and force_worktree on the job
                    if let Some(job) = manager.get_mut(job_id) {
                        job.group_id = Some(group_id);
                        let ide_context = selection.format_ide_context();
                        if !ide_context.trim().is_empty() && ide_context.lines().count() > 1 {
                            job.ide_context = Some(ide_context);
                        }
                        job.force_worktree = force_worktree;
                    }

                    // Add to group
                    if let Ok(mut gm) = group_manager.lock() {
                        gm.add_job_to_group(group_id, job_id, agent.clone());
                    }

                    job_ids.push(job_id);
                    logs.push(LogEvent::system(format!(
                        "Created job #{} for agent {}",
                        job_id, agent
                    )));
                }
                Err(e) => {
                    logs.push(LogEvent::error(format!(
                        "Failed to create job for {}: {}",
                        agent, e
                    )));
                }
            }
        }
    }

    if job_ids.is_empty() {
        // No jobs were created, cancel the group
        if let Ok(mut gm) = group_manager.lock() {
            gm.cancel_group(group_id);
        }
        return None;
    }

    logs.push(LogEvent::system(format!(
        "Created group #{} with {} parallel jobs",
        group_id,
        job_ids.len()
    )));

    Some(CreateJobsResult {
        job_ids,
        group_id: Some(group_id),
    })
}
