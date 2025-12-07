//! Group operations: merge and cleanup
//!
//! This module handles the workflow of merging the selected result and
//! cleaning up all worktrees from the group.

use crate::git::GitManager;
use crate::job::{GroupManager, JobManager};
use crate::{AgentGroupId, JobStatus};

/// Result of a group operation
pub struct GroupOperationResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Message describing the result
    pub message: String,
}

impl GroupOperationResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
        }
    }
}

/// Merge the selected job and cleanup all worktrees in the group
///
/// This function:
/// 1. Gets the selected job from the group
/// 2. Merges its worktree branch into the main branch
/// 3. Removes all worktrees for the group (including the selected one)
/// 4. Marks the group as merged
pub fn merge_and_cleanup(
    group_id: AgentGroupId,
    group_manager: &mut GroupManager,
    job_manager: &mut JobManager,
    git_manager: &GitManager,
) -> GroupOperationResult {
    // Get the group
    let group = match group_manager.get(group_id) {
        Some(g) => g.clone(),
        None => return GroupOperationResult::error("Group not found"),
    };

    // Check that a job is selected
    let selected_job_id = match group.selected_job {
        Some(id) => id,
        None => return GroupOperationResult::error("No job selected for merge"),
    };

    // Get the selected job
    let selected_job = match job_manager.get(selected_job_id) {
        Some(j) => j.clone(),
        None => return GroupOperationResult::error("Selected job not found"),
    };

    // Check that the job has a worktree
    let worktree_path = match &selected_job.git_worktree_path {
        Some(p) => p.clone(),
        None => return GroupOperationResult::error("Selected job has no worktree"),
    };

    // Merge the selected job's changes
    if let Err(e) = git_manager.apply_changes(&worktree_path) {
        return GroupOperationResult::error(format!("Failed to merge changes: {}", e));
    }

    // Mark the selected job as merged
    if let Some(job) = job_manager.get_mut(selected_job_id) {
        job.set_status(JobStatus::Merged);
    }

    // Mark other jobs as rejected
    for &job_id in &group.job_ids {
        if job_id != selected_job_id {
            if let Some(job) = job_manager.get_mut(job_id) {
                job.set_status(JobStatus::Rejected);
            }
        }
    }

    // Remove all worktrees in the group
    let mut cleanup_errors = Vec::new();
    for &job_id in &group.job_ids {
        if let Some(job) = job_manager.get(job_id) {
            if let Some(worktree) = &job.git_worktree_path {
                if let Err(e) = git_manager.remove_worktree_by_path(worktree) {
                    cleanup_errors.push(format!("Job #{}: {}", job_id, e));
                }
            }
        }
    }

    // Mark the group as merged
    group_manager.mark_merged(group_id);

    if cleanup_errors.is_empty() {
        GroupOperationResult::success(format!(
            "Merged changes from {} and cleaned up {} worktrees",
            selected_job.agent_id,
            group.job_ids.len()
        ))
    } else {
        GroupOperationResult::success(format!(
            "Merged changes from {} (cleanup warnings: {})",
            selected_job.agent_id,
            cleanup_errors.join(", ")
        ))
    }
}

/// Cancel a group and cleanup all worktrees
pub fn cancel_and_cleanup(
    group_id: AgentGroupId,
    group_manager: &mut GroupManager,
    job_manager: &mut JobManager,
    git_manager: &GitManager,
) -> GroupOperationResult {
    // Get the group
    let group = match group_manager.get(group_id) {
        Some(g) => g.clone(),
        None => return GroupOperationResult::error("Group not found"),
    };

    // Mark all jobs as rejected
    for &job_id in &group.job_ids {
        if let Some(job) = job_manager.get_mut(job_id) {
            if !job.is_finished() {
                job.set_status(JobStatus::Rejected);
            }
        }
    }

    // Remove all worktrees
    let mut cleanup_errors = Vec::new();
    for &job_id in &group.job_ids {
        if let Some(job) = job_manager.get(job_id) {
            if let Some(worktree) = &job.git_worktree_path {
                if let Err(e) = git_manager.remove_worktree_by_path(worktree) {
                    cleanup_errors.push(format!("Job #{}: {}", job_id, e));
                }
            }
        }
    }

    // Cancel the group
    group_manager.cancel_group(group_id);

    if cleanup_errors.is_empty() {
        GroupOperationResult::success(format!(
            "Cancelled group and cleaned up {} worktrees",
            group.job_ids.len()
        ))
    } else {
        GroupOperationResult::success(format!(
            "Cancelled group (cleanup warnings: {})",
            cleanup_errors.join(", ")
        ))
    }
}
