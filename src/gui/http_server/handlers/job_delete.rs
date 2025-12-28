//! Job deletion handler.

use super::super::types::{ControlApiState, ControlJobDeleteRequest, ControlJobDeleteResponse};
use super::super::respond_json;
use super::{parse_job_id_from_path, ExecutorEvent};
use crate::git::GitManager;
use crate::{JobStatus, LogEvent};

pub fn handle_control_job_delete(
    control: &ControlApiState,
    path: &str,
    body: &str,
    request: tiny_http::Request,
) {
    let job_id = match parse_job_id_from_path(path, Some("delete")) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

    let req: ControlJobDeleteRequest = if body.trim().is_empty() {
        ControlJobDeleteRequest {
            cleanup_worktree: false,
        }
    } else {
        match serde_json::from_str(body) {
            Ok(req) => req,
            Err(e) => {
                respond_json(
                    request,
                    400,
                    serde_json::json!({ "error": "invalid_json", "details": e.to_string() }),
                );
                return;
            }
        }
    };

    let removed = match control.job_manager.lock() {
        Ok(mut manager) => {
            if let Some(job) = manager.get(job_id) {
                if matches!(job.status, JobStatus::Running) {
                    respond_json(
                        request,
                        400,
                        serde_json::json!({ "error": "job_running", "job_id": job_id }),
                    );
                    return;
                }
            }
            manager.remove_job(job_id)
        }
        Err(_) => {
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "job_manager_lock" }),
            );
            return;
        }
    };

    let Some(job) = removed else {
        respond_json(request, 404, serde_json::json!({ "error": "not_found" }));
        return;
    };

    // Remove from group tracking (best-effort).
    if let Some(group_id) = job.group_id {
        if let Ok(mut gm) = control.group_manager.lock() {
            let _ = gm.remove_job(job_id);
            let _ = control
                .executor_tx
                .send(ExecutorEvent::Log(LogEvent::system(format!(
                    "Removed job #{} from group #{}",
                    job_id, group_id
                ))));
        }
    }

    if req.cleanup_worktree {
        if let Some(worktree_path) = job.git_worktree_path.as_ref() {
            let workspace_root = job
                .workspace_path
                .clone()
                .unwrap_or_else(|| control.work_dir.clone());
            match GitManager::new(&workspace_root)
                .and_then(|gm| gm.remove_worktree_by_path(worktree_path))
            {
                Ok(()) => {
                    let _ = control
                        .executor_tx
                        .send(ExecutorEvent::Log(LogEvent::system(format!(
                            "Removed worktree for job #{}",
                            job_id
                        ))));
                }
                Err(e) => {
                    let _ = control
                        .executor_tx
                        .send(ExecutorEvent::Log(LogEvent::error(format!(
                            "Failed to remove worktree for job #{}: {}",
                            job_id, e
                        ))));
                }
            }
        }
    }

    let _ = control
        .executor_tx
        .send(ExecutorEvent::Log(LogEvent::system(format!(
            "Deleted job #{}",
            job_id
        ))));

    respond_json(
        request,
        200,
        serde_json::to_value(ControlJobDeleteResponse {
            status: "ok".to_string(),
            job_id,
            cleanup_worktree: req.cleanup_worktree,
        })
        .unwrap_or_else(|_| serde_json::json!({ "error": "serialize" })),
    );
}
