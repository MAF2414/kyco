//! Job worktree handlers: merge, reject, diff.
//!
//! These handlers manage the git worktree lifecycle for completed jobs.

use super::super::respond_json;
use super::super::types::ControlApiState;
use super::{parse_job_id_from_path, ExecutorEvent};
use crate::git::{CommitMessage, GitManager};
use crate::{JobStatus, LogEvent};

/// Handle POST /ctl/jobs/{id}/merge
///
/// Merges the job's worktree changes into the base branch and cleans up the worktree.
pub fn handle_control_job_merge(
    control: &ControlApiState,
    path: &str,
    body: &str,
    request: tiny_http::Request,
) {
    let job_id = match parse_job_id_from_path(path, Some("merge")) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

    // Parse optional commit message from body
    let custom_message: Option<String> = if !body.trim().is_empty() {
        serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(String::from))
    } else {
        None
    };

    // Get job info
    let job = match control.job_manager.lock() {
        Ok(manager) => manager.get(job_id).cloned(),
        Err(_) => {
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "job_manager_lock" }),
            );
            return;
        }
    };

    let Some(job) = job else {
        respond_json(request, 404, serde_json::json!({ "error": "not_found" }));
        return;
    };

    // Check if job is in a mergeable state
    if !matches!(job.status, JobStatus::Done) {
        respond_json(
            request,
            400,
            serde_json::json!({
                "error": "not_mergeable",
                "message": format!("Job must be in 'done' status to merge, current: {}", job.status),
                "job_id": job_id,
                "status": job.status
            }),
        );
        return;
    }

    // Check if job has a worktree
    let Some(worktree_path) = job.git_worktree_path.clone() else {
        // No worktree - just mark as merged (changes were made in-place)
        if let Ok(mut manager) = control.job_manager.lock() {
            if let Some(j) = manager.get_mut(job_id) {
                j.set_status(JobStatus::Merged);
            }
        }
        let _ = control
            .executor_tx
            .send(ExecutorEvent::Log(LogEvent::system(format!(
                "Merged job #{} (no worktree)",
                job_id
            ))));
        respond_json(
            request,
            200,
            serde_json::json!({
                "status": "ok",
                "job_id": job_id,
                "message": "Job marked as merged (no worktree to apply)"
            }),
        );
        return;
    };

    let Some(base_branch) = job.base_branch.clone() else {
        respond_json(
            request,
            400,
            serde_json::json!({
                "error": "no_base_branch",
                "message": "Job has no base branch recorded"
            }),
        );
        return;
    };

    // Get workspace root
    let workspace_root = job
        .workspace_path
        .clone()
        .unwrap_or_else(|| control.work_dir.clone());

    // Initialize git manager and perform merge
    let git = match GitManager::new(&workspace_root) {
        Ok(g) => g,
        Err(e) => {
            respond_json(
                request,
                500,
                serde_json::json!({
                    "error": "git_init_failed",
                    "message": e.to_string()
                }),
            );
            return;
        }
    };

    // Create commit message
    let commit_message = custom_message
        .map(|msg| CommitMessage::new(msg, None))
        .unwrap_or_else(|| CommitMessage::from_job(&job));

    // Apply changes (merge worktree into base branch)
    if let Err(e) = git.apply_changes(&worktree_path, &base_branch, Some(&commit_message)) {
        respond_json(
            request,
            500,
            serde_json::json!({
                "error": "merge_failed",
                "message": e.to_string()
            }),
        );
        return;
    }

    // Cleanup worktree
    let cleanup_warning = match git.remove_worktree_by_path(&worktree_path) {
        Ok(()) => None,
        Err(e) => Some(e.to_string()),
    };

    // Update job status
    if let Ok(mut manager) = control.job_manager.lock() {
        if let Some(j) = manager.get_mut(job_id) {
            j.set_status(JobStatus::Merged);
            j.git_worktree_path = None;
            j.branch_name = None;
        }
    }

    let message = match cleanup_warning {
        Some(warn) => format!("Merged job #{} (cleanup warning: {})", job_id, warn),
        None => format!("Merged job #{}", job_id),
    };

    let _ = control
        .executor_tx
        .send(ExecutorEvent::Log(LogEvent::system(message.clone())));

    respond_json(
        request,
        200,
        serde_json::json!({
            "status": "ok",
            "job_id": job_id,
            "message": message
        }),
    );
}

/// Handle POST /ctl/jobs/{id}/reject
///
/// Rejects the job's changes and removes the worktree.
pub fn handle_control_job_reject(
    control: &ControlApiState,
    path: &str,
    request: tiny_http::Request,
) {
    let job_id = match parse_job_id_from_path(path, Some("reject")) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

    // Get job info
    let job = match control.job_manager.lock() {
        Ok(manager) => manager.get(job_id).cloned(),
        Err(_) => {
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "job_manager_lock" }),
            );
            return;
        }
    };

    let Some(job) = job else {
        respond_json(request, 404, serde_json::json!({ "error": "not_found" }));
        return;
    };

    // Check if job is in a rejectable state
    if !matches!(job.status, JobStatus::Done | JobStatus::Failed) {
        respond_json(
            request,
            400,
            serde_json::json!({
                "error": "not_rejectable",
                "message": format!("Job must be in 'done' or 'failed' status to reject, current: {}", job.status),
                "job_id": job_id,
                "status": job.status
            }),
        );
        return;
    }

    // Cleanup worktree if present
    let mut cleanup_warning: Option<String> = None;
    if let Some(worktree_path) = job.git_worktree_path.clone() {
        let workspace_root = job
            .workspace_path
            .clone()
            .unwrap_or_else(|| control.work_dir.clone());

        if let Ok(git) = GitManager::new(&workspace_root) {
            if let Err(e) = git.remove_worktree_by_path(&worktree_path) {
                cleanup_warning = Some(e.to_string());
            }
        }
    }

    // Update job status
    if let Ok(mut manager) = control.job_manager.lock() {
        if let Some(j) = manager.get_mut(job_id) {
            j.set_status(JobStatus::Rejected);
            j.git_worktree_path = None;
            j.branch_name = None;
        }
    }

    let message = match cleanup_warning {
        Some(warn) => format!("Rejected job #{} (cleanup warning: {})", job_id, warn),
        None => format!("Rejected job #{}", job_id),
    };

    let _ = control
        .executor_tx
        .send(ExecutorEvent::Log(LogEvent::system(message.clone())));

    respond_json(
        request,
        200,
        serde_json::json!({
            "status": "ok",
            "job_id": job_id,
            "message": message
        }),
    );
}

/// Handle GET /ctl/jobs/{id}/diff
///
/// Returns the diff of changes made by the job.
pub fn handle_control_job_diff(control: &ControlApiState, path: &str, request: tiny_http::Request) {
    let job_id = match parse_job_id_from_path(path, Some("diff")) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

    // Get job info
    let job = match control.job_manager.lock() {
        Ok(manager) => manager.get(job_id).cloned(),
        Err(_) => {
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "job_manager_lock" }),
            );
            return;
        }
    };

    let Some(job) = job else {
        respond_json(request, 404, serde_json::json!({ "error": "not_found" }));
        return;
    };

    // Check if job has a worktree
    let Some(worktree_path) = job.git_worktree_path.clone() else {
        respond_json(
            request,
            400,
            serde_json::json!({
                "error": "no_worktree",
                "message": "Job has no worktree (changes may have been made in-place or already merged)"
            }),
        );
        return;
    };

    let workspace_root = job
        .workspace_path
        .clone()
        .unwrap_or_else(|| control.work_dir.clone());

    // Initialize git manager
    let git = match GitManager::new(&workspace_root) {
        Ok(g) => g,
        Err(e) => {
            respond_json(
                request,
                500,
                serde_json::json!({
                    "error": "git_init_failed",
                    "message": e.to_string()
                }),
            );
            return;
        }
    };

    // Get the diff
    let diff = match git.diff(&worktree_path, job.base_branch.as_deref()) {
        Ok(d) => d,
        Err(e) => {
            respond_json(
                request,
                500,
                serde_json::json!({
                    "error": "diff_failed",
                    "message": e.to_string()
                }),
            );
            return;
        }
    };

    // Get changed files list
    let changed_files = git.changed_files(&worktree_path).unwrap_or_default();
    let changed_files: Vec<String> = changed_files
        .into_iter()
        .filter_map(|p| p.to_str().map(String::from))
        .collect();

    respond_json(
        request,
        200,
        serde_json::json!({
            "job_id": job_id,
            "diff": diff,
            "changed_files": changed_files,
            "worktree_path": worktree_path.to_string_lossy(),
            "base_branch": job.base_branch
        }),
    );
}
