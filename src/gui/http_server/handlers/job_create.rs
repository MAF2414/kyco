//! Job creation handler.

use std::path::{Path, PathBuf};

use super::super::types::{ControlApiState, ControlJobCreateRequest, ControlJobCreateResponse};
use super::super::respond_json;
use super::ExecutorEvent;
use crate::gui::jobs;
use crate::gui::selection::SelectionContext;
use crate::LogEvent;

fn expand_tilde(path: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    if path == "~" {
        return Some(home);
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return Some(home.join(rest));
    }
    #[cfg(windows)]
    if let Some(rest) = path.strip_prefix("~\\") {
        return Some(home.join(rest));
    }
    None
}

/// Find the workspace root for a file by searching for `.kyco` or `.git` directories
/// in parent directories. Falls back to the provided default if none found.
fn find_workspace_root(file_path: &Path, default: &Path) -> PathBuf {
    let mut current = if file_path.is_file() {
        file_path.parent()
    } else {
        Some(file_path)
    };

    while let Some(dir) = current {
        // Check for .kyco directory (kyco project root)
        if dir.join(".kyco").is_dir() {
            return dir.to_path_buf();
        }
        // Check for .git directory (git repo root)
        if dir.join(".git").exists() {
            return dir.to_path_buf();
        }
        current = dir.parent();
    }

    // Fallback to default
    default.to_path_buf()
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

    // Validate: need either file_path or prompt (or both)
    let file_path_raw = req.file_path.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let prompt_provided = req.prompt.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_some();

    if file_path_raw.is_none() && !prompt_provided {
        respond_json(
            request,
            400,
            serde_json::json!({ "error": "missing_file_or_prompt", "message": "Either file_path or prompt (or both) must be provided" }),
        );
        return;
    }

    // Validate mode exists (mode or chain), including alias resolution.
    let resolved_mode = match control.config.read() {
        Ok(config) => {
            let resolved = config
                .alias
                .mode
                .get(mode)
                .cloned()
                .unwrap_or_else(|| mode.to_string());

            if config.get_mode_or_chain(&resolved).is_none() {
                respond_json(
                    request,
                    400,
                    serde_json::json!({
                        "error": "unknown_mode",
                        "message": format!("Unknown mode or chain: {}", resolved),
                        "mode": resolved,
                    }),
                );
                return;
            }

            resolved
        }
        Err(_) => {
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "config_lock" }),
            );
            return;
        }
    };

    // Basic line range validation.
    if let Some(start) = req.line_start {
        if start == 0 {
            respond_json(
                request,
                400,
                serde_json::json!({
                    "error": "invalid_line_start",
                    "message": "line_start must be >= 1",
                    "line_start": start,
                }),
            );
            return;
        }
    }
    if let Some(end) = req.line_end {
        if end == 0 {
            respond_json(
                request,
                400,
                serde_json::json!({
                    "error": "invalid_line_end",
                    "message": "line_end must be >= 1",
                    "line_end": end,
                }),
            );
            return;
        }
    }
    if let (Some(start), Some(end)) = (req.line_start, req.line_end) {
        if end < start {
            respond_json(
                request,
                400,
                serde_json::json!({
                    "error": "invalid_line_range",
                    "message": "line_end must be >= line_start",
                    "line_start": start,
                    "line_end": end,
                }),
            );
            return;
        }
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

    // Normalize and validate file path if provided
    let (abs_path_str, workspace) = if let Some(file_path_raw) = file_path_raw {
        let path = expand_tilde(file_path_raw)
            .unwrap_or_else(|| std::path::PathBuf::from(file_path_raw));
        let abs_path = if path.is_absolute() {
            path
        } else {
            control.work_dir.join(path)
        };
        if !abs_path.exists() {
            respond_json(
                request,
                400,
                serde_json::json!({
                    "error": "file_not_found",
                    "message": format!("File not found: {}", abs_path.display()),
                    "file_path": file_path_raw,
                    "resolved_path": abs_path.display().to_string(),
                }),
            );
            return;
        }
        if !abs_path.is_file() {
            respond_json(
                request,
                400,
                serde_json::json!({
                    "error": "path_not_file",
                    "message": format!("Path is not a file: {}", abs_path.display()),
                    "file_path": file_path_raw,
                    "resolved_path": abs_path.display().to_string(),
                }),
            );
            return;
        }
        // Find the correct workspace root based on the file location
        let ws = find_workspace_root(&abs_path, &control.work_dir);
        (Some(abs_path.display().to_string()), ws)
    } else {
        // No file provided - use current work_dir as workspace
        (None, control.work_dir.clone())
    };

    let selection = SelectionContext {
        app_name: Some("CLI".to_string()),
        file_path: abs_path_str,
        selected_text: req.selected_text,
        line_number: req.line_start,
        line_end: req.line_end,
        workspace_path: Some(workspace),
        ..Default::default()
    };

    let prompt = req.prompt.unwrap_or_default();
    let mut logs: Vec<LogEvent> = Vec::new();

    let created = jobs::create_jobs_from_selection_multi(
        &control.job_manager,
        &control.group_manager,
        &selection,
        &agents,
        &resolved_mode,
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
