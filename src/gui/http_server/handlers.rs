//! HTTP request handlers for IDE and control API endpoints.

use std::sync::mpsc::Sender;
use tiny_http::Response;
use tracing::{error, info};

use super::types::{
    BatchRequest, ControlApiState, ControlJobContinueRequest, ControlJobContinueResponse,
    ControlJobCreateRequest, ControlJobCreateResponse, ControlJobDeleteRequest,
    ControlJobDeleteResponse, ControlLogRequest, SelectionRequest,
};
use super::{json_content_type, respond_json};
use crate::agent::bridge::BridgeClient;
use crate::config::Config;
use crate::git::GitManager;
use crate::gui::jobs;
use crate::gui::selection::SelectionContext;
use crate::{CommentTag, Job, JobId, JobStatus, LogEvent, Target};

use super::super::executor::ExecutorEvent;

/// Handle POST /selection request
pub fn handle_selection_request(
    tx: &Sender<SelectionRequest>,
    body: &str,
    request: tiny_http::Request,
) {
    match serde_json::from_str::<SelectionRequest>(body) {
        Ok(selection) => {
            info!(
                "[kyco:http] Received selection: file={:?}, lines={:?}-{:?}, text_len={:?}, workspace={:?}",
                selection.file_path,
                selection.line_start,
                selection.line_end,
                selection.selected_text.as_ref().map(|s| s.len()),
                selection.workspace
            );

            // Send to GUI
            if let Err(e) = tx.send(selection) {
                error!("[kyco:http] Failed to send to GUI: {}", e);
            }

            let response =
                Response::from_string("{\"status\":\"ok\"}").with_header(json_content_type());
            let _ = request.respond(response);
        }
        Err(e) => {
            error!("[kyco:http] Invalid JSON: {}", e);
            let response = Response::from_string(format!("{{\"error\":\"{}\"}}", e))
                .with_status_code(400)
                .with_header(json_content_type());
            let _ = request.respond(response);
        }
    }
}

/// Handle POST /batch request
pub fn handle_batch_request(tx: &Sender<BatchRequest>, body: &str, request: tiny_http::Request) {
    match serde_json::from_str::<BatchRequest>(body) {
        Ok(batch) => {
            info!("[kyco:http] Batch: {} files", batch.files.len());

            // Send to GUI (will open batch popup for mode/agent/prompt selection)
            if let Err(e) = tx.send(batch) {
                error!("[kyco:http] Failed to send batch to GUI: {}", e);
            }

            let response =
                Response::from_string("{\"status\":\"ok\"}").with_header(json_content_type());
            let _ = request.respond(response);
        }
        Err(e) => {
            error!("[kyco:http] Invalid batch JSON: {}", e);
            let response = Response::from_string(format!("{{\"error\":\"{}\"}}", e))
                .with_status_code(400)
                .with_header(json_content_type());
            let _ = request.respond(response);
        }
    }
}

pub fn handle_control_jobs_list(control: &ControlApiState, request: tiny_http::Request) {
    let jobs: Vec<Job> = match control.job_manager.lock() {
        Ok(manager) => manager.jobs().into_iter().cloned().collect(),
        Err(_) => {
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "job_manager_lock" }),
            );
            return;
        }
    };

    respond_json(request, 200, serde_json::json!({ "jobs": jobs }));
}

pub fn handle_control_job_get(control: &ControlApiState, path: &str, request: tiny_http::Request) {
    let job_id = match parse_job_id_from_path(path, None) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

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

    respond_json(request, 200, serde_json::json!({ "job": job }));
}

pub fn handle_control_job_queue(control: &ControlApiState, path: &str, request: tiny_http::Request) {
    let job_id = match parse_job_id_from_path(path, Some("queue")) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

    let status = match control.job_manager.lock() {
        Ok(mut manager) => match manager.get(job_id).is_some() {
            true => {
                manager.set_status(job_id, JobStatus::Queued);
                Some(JobStatus::Queued)
            }
            false => None,
        },
        Err(_) => {
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "job_manager_lock" }),
            );
            return;
        }
    };

    let Some(status) = status else {
        respond_json(request, 404, serde_json::json!({ "error": "not_found" }));
        return;
    };

    let _ = control
        .executor_tx
        .send(ExecutorEvent::Log(LogEvent::system(format!(
            "Queued job #{}",
            job_id
        ))));

    respond_json(
        request,
        200,
        serde_json::json!({ "status": "ok", "job_id": job_id, "job_status": status }),
    );
}

pub fn handle_control_job_abort(control: &ControlApiState, path: &str, request: tiny_http::Request) {
    let job_id = match parse_job_id_from_path(path, Some("abort")) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

    let (agent_id, session_id, status) = match control.job_manager.lock() {
        Ok(mut manager) => match manager.get_mut(job_id) {
            Some(job) => (
                job.agent_id.clone(),
                job.bridge_session_id.clone(),
                job.status,
            ),
            None => {
                respond_json(request, 404, serde_json::json!({ "error": "not_found" }));
                return;
            }
        },
        Err(_) => {
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "job_manager_lock" }),
            );
            return;
        }
    };

    if matches!(
        status,
        JobStatus::Running | JobStatus::Queued | JobStatus::Pending | JobStatus::Blocked
    ) {
        if let Some(session_id) = session_id.as_deref() {
            let client = BridgeClient::new();
            let agent_id_lower = agent_id.to_ascii_lowercase();
            let likely_codex = agent_id_lower == "codex" || agent_id_lower.contains("codex");

            let interrupt_attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if likely_codex {
                    client
                        .interrupt_codex(session_id)
                        .or_else(|_| client.interrupt_claude(session_id))
                } else {
                    client
                        .interrupt_claude(session_id)
                        .or_else(|_| client.interrupt_codex(session_id))
                }
            }));

            match interrupt_attempt {
                Ok(Ok(true)) => {
                    let _ = control
                        .executor_tx
                        .send(ExecutorEvent::Log(LogEvent::system(format!(
                            "Sent interrupt for job #{} (agent: {})",
                            job_id, agent_id
                        ))));
                }
                Ok(Ok(false)) => {
                    let _ = control
                        .executor_tx
                        .send(ExecutorEvent::Log(LogEvent::error(format!(
                            "Interrupt rejected for job #{} (agent: {})",
                            job_id, agent_id
                        ))));
                }
                Ok(Err(e)) => {
                    let _ = control
                        .executor_tx
                        .send(ExecutorEvent::Log(LogEvent::error(format!(
                            "Failed to interrupt job #{} (agent: {}): {}",
                            job_id, agent_id, e
                        ))));
                }
                Err(_) => {
                    let _ = control
                        .executor_tx
                        .send(ExecutorEvent::Log(LogEvent::error(format!(
                            "Bridge interrupt panicked (job #{})",
                            job_id
                        ))));
                }
            }
        }

        // Mark job as failed ("aborted")
        match control.job_manager.lock() {
            Ok(mut manager) => {
                if let Some(job) = manager.get_mut(job_id) {
                    job.fail("Job aborted by user".to_string());
                } else {
                    // Job was deleted by another request between our first lock and now
                    let _ = control
                        .executor_tx
                        .send(ExecutorEvent::Log(LogEvent::error(format!(
                            "Job #{} no longer exists during abort",
                            job_id
                        ))));
                }
            }
            Err(e) => {
                // Lock poisoned - log the error but continue to respond
                let _ = control
                    .executor_tx
                    .send(ExecutorEvent::Log(LogEvent::error(format!(
                        "Lock poisoned during job #{} abort: {}",
                        job_id, e
                    ))));
            }
        }

        let _ = control
            .executor_tx
            .send(ExecutorEvent::Log(LogEvent::system(format!(
                "Aborted job #{}",
                job_id
            ))));

        respond_json(
            request,
            200,
            serde_json::json!({ "status": "ok", "job_id": job_id }),
        );
        return;
    }

    respond_json(
        request,
        400,
        serde_json::json!({ "error": "not_abortable", "job_id": job_id, "status": status }),
    );
}

pub fn handle_control_job_create(
    control: &ControlApiState,
    body: &str,
    request: tiny_http::Request,
) {
    let req: ControlJobCreateRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(e) => {
            respond_json(
                request,
                400,
                serde_json::json!({ "error": "invalid_json", "details": e.to_string() }),
            );
            return;
        }
    };

    let mode = req.mode.trim();
    if mode.is_empty() {
        respond_json(request, 400, serde_json::json!({ "error": "missing_mode" }));
        return;
    }

    let file_path_raw = req.file_path.trim();
    if file_path_raw.is_empty() {
        respond_json(
            request,
            400,
            serde_json::json!({ "error": "missing_file_path" }),
        );
        return;
    }

    let mut agents: Vec<String> = req
        .agents
        .unwrap_or_default()
        .into_iter()
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .collect();
    if agents.is_empty() {
        let agent = req.agent.as_deref().unwrap_or("claude").trim().to_string();
        agents.push(agent);
    }

    // Normalize file path to absolute within work_dir.
    let path = std::path::PathBuf::from(file_path_raw);
    let abs_path = if path.is_absolute() {
        path
    } else {
        control.work_dir.join(path)
    };
    let abs_path_str = abs_path.display().to_string();

    let selection = SelectionContext {
        app_name: Some("CLI".to_string()),
        file_path: Some(abs_path_str),
        selected_text: req.selected_text,
        line_number: req.line_start,
        line_end: req.line_end,
        workspace_path: Some(control.work_dir.clone()),
        ..Default::default()
    };

    let prompt = req.prompt.unwrap_or_default();
    let mut logs: Vec<LogEvent> = Vec::new();

    let created = jobs::create_jobs_from_selection_multi(
        &control.job_manager,
        &control.group_manager,
        &selection,
        &agents,
        mode,
        &prompt,
        &mut logs,
        req.force_worktree,
    );

    let Some(created) = created else {
        respond_json(
            request,
            500,
            serde_json::json!({ "error": "create_failed" }),
        );
        return;
    };

    if req.queue {
        for job_id in &created.job_ids {
            jobs::queue_job(&control.job_manager, *job_id, &mut logs);
        }
    }

    for log in &logs {
        let _ = control.executor_tx.send(ExecutorEvent::Log(log.clone()));
    }

    respond_json(
        request,
        200,
        serde_json::to_value(ControlJobCreateResponse {
            job_ids: created.job_ids,
            group_id: created.group_id,
        })
        .unwrap_or_else(|_| serde_json::json!({ "error": "serialize" })),
    );
}

pub fn handle_control_job_continue(
    control: &ControlApiState,
    path: &str,
    body: &str,
    request: tiny_http::Request,
) {
    let job_id = match parse_job_id_from_path(path, Some("continue")) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

    let req: ControlJobContinueRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(e) => {
            respond_json(
                request,
                400,
                serde_json::json!({ "error": "invalid_json", "details": e.to_string() }),
            );
            return;
        }
    };

    let prompt = req.prompt.trim();
    if prompt.is_empty() {
        respond_json(
            request,
            400,
            serde_json::json!({ "error": "missing_prompt" }),
        );
        return;
    }

    let mut logs: Vec<LogEvent> = Vec::new();

    let created_id = {
        let mut manager = match control.job_manager.lock() {
            Ok(m) => m,
            Err(_) => {
                respond_json(
                    request,
                    500,
                    serde_json::json!({ "error": "job_manager_lock" }),
                );
                return;
            }
        };

        let Some(original) = manager.get(job_id).cloned() else {
            respond_json(request, 404, serde_json::json!({ "error": "not_found" }));
            return;
        };

        let Some(session_id) = original.bridge_session_id.clone() else {
            respond_json(request, 400, serde_json::json!({ "error": "no_session" }));
            return;
        };

        let tag = CommentTag {
            file_path: original.source_file.clone(),
            line_number: original.source_line,
            raw_line: format!("// @{}:{} {}", &original.agent_id, &original.mode, prompt),
            agent: original.agent_id.clone(),
            agents: vec![original.agent_id.clone()],
            mode: original.mode.clone(),
            target: Target::Block,
            status_marker: None,
            description: Some(prompt.to_string()),
            job_id: None,
        };

        let continuation_id = match manager.create_job_with_range(&tag, &original.agent_id, None) {
            Ok(id) => id,
            Err(e) => {
                respond_json(
                    request,
                    500,
                    serde_json::json!({ "error": "create_failed", "details": e.to_string() }),
                );
                return;
            }
        };

        if let Some(job) = manager.get_mut(continuation_id) {
            job.raw_tag_line = None;
            job.bridge_session_id = Some(session_id);

            // Reuse the same worktree and job context
            job.git_worktree_path = original.git_worktree_path.clone();
            job.branch_name = original.branch_name.clone();
            job.base_branch = original.base_branch.clone();
            job.scope = original.scope.clone();
            job.target = original.target;
            job.ide_context = original.ide_context;
            job.force_worktree = original.force_worktree;
            job.workspace_id = original.workspace_id;
            job.workspace_path = original.workspace_path.clone();
        }

        logs.push(LogEvent::system(format!(
            "Created continuation job #{} (from job #{})",
            continuation_id, job_id
        )));

        continuation_id
    };

    if req.queue {
        jobs::queue_job(&control.job_manager, created_id, &mut logs);
    }

    for log in &logs {
        let _ = control.executor_tx.send(ExecutorEvent::Log(log.clone()));
    }

    respond_json(
        request,
        200,
        serde_json::to_value(ControlJobContinueResponse { job_id: created_id })
            .unwrap_or_else(|_| serde_json::json!({ "error": "serialize" })),
    );
}

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

    let removed: Option<Job> = match control.job_manager.lock() {
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

pub fn handle_control_log(control: &ControlApiState, body: &str, request: tiny_http::Request) {
    let req: ControlLogRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(e) => {
            respond_json(
                request,
                400,
                serde_json::json!({ "error": "invalid_json", "details": e.to_string() }),
            );
            return;
        }
    };

    let msg = req.message.trim();
    if msg.is_empty() {
        respond_json(
            request,
            400,
            serde_json::json!({ "error": "missing_message" }),
        );
        return;
    }

    let _ = control
        .executor_tx
        .send(ExecutorEvent::Log(LogEvent::system(msg.to_string())));

    respond_json(request, 200, serde_json::json!({ "status": "ok" }));
}

/// Handle config reload request from CLI or orchestrators.
/// Immediately reloads the config from disk, bypassing the 500ms polling interval.
pub fn handle_control_config_reload(control: &ControlApiState, request: tiny_http::Request) {
    match Config::from_file(&control.config_path) {
        Ok(new_config) => {
            if let Ok(mut guard) = control.config.write() {
                *guard = new_config;
            }
            let _ = control
                .executor_tx
                .send(ExecutorEvent::Log(LogEvent::system(format!(
                    "Config reloaded via API from {}",
                    control.config_path.display()
                ))));
            respond_json(request, 200, serde_json::json!({ "status": "ok" }));
        }
        Err(e) => {
            let _ = control
                .executor_tx
                .send(ExecutorEvent::Log(LogEvent::error(format!(
                    "Failed to reload config: {}",
                    e
                ))));
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "reload_failed", "details": e.to_string() }),
            );
        }
    }
}

fn parse_job_id_from_path(path: &str, suffix: Option<&str>) -> Result<JobId, &'static str> {
    let trimmed = path.trim_end_matches('/');
    let trimmed = match suffix {
        Some(suffix) => trimmed
            .strip_suffix(&format!("/{suffix}"))
            .ok_or("bad_path")?,
        None => trimmed,
    };

    let id_str = trimmed.rsplit('/').next().ok_or("bad_path")?;
    id_str.parse::<JobId>().map_err(|_| "bad_job_id")
}
