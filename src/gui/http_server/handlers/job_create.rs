//! Job creation handler.

use super::super::types::{ControlApiState, ControlJobCreateRequest, ControlJobCreateResponse};
use super::super::respond_json;
use super::ExecutorEvent;
use crate::gui::jobs;
use crate::gui::selection::SelectionContext;
use crate::LogEvent;

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
