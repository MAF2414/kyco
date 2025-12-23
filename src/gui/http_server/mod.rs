//! HTTP server for receiving selection data from IDE extensions
//!
//! Listens on localhost:9876 and accepts:
//! - POST /selection - Single file selection from IDE
//! - POST /batch - Batch processing of multiple files
//! - Control endpoints under /ctl/* (for orchestrators / CLI)

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use tiny_http::{Response, Server};
use tracing::{error, info};

use super::executor::ExecutorEvent;
use super::jobs;
use super::selection::SelectionContext;
use crate::job::{GroupManager, JobManager};
use crate::{Job, JobId, JobStatus, LogEvent};

const AUTH_HEADER: &str = "X-KYCO-Token";
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024; // 2 MiB

/// Shared state for the local control API (used by `kyco job ...` and orchestrators).
#[derive(Clone)]
pub struct ControlApiState {
    pub work_dir: std::path::PathBuf,
    pub job_manager: Arc<Mutex<JobManager>>,
    pub group_manager: Arc<Mutex<GroupManager>>,
    pub executor_tx: Sender<ExecutorEvent>,
}

/// Dependency location from IDE
#[derive(Debug, Clone, Deserialize)]
pub struct Dependency {
    pub file_path: String,
    pub line: usize,
}

/// Diagnostic (error, warning, etc.) from IDE
#[derive(Debug, Clone, Deserialize)]
pub struct Diagnostic {
    /// Severity level: Error, Warning, Information, or Hint
    pub severity: String,
    /// The diagnostic message
    pub message: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Optional error/warning code from the language server
    pub code: Option<String>,
}

/// Selection data received from IDE extensions
#[derive(Debug, Clone, Deserialize)]
pub struct SelectionRequest {
    pub file_path: Option<String>,
    pub selected_text: Option<String>,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub workspace: Option<String>,
    /// Git repository root if file is in a git repo, None otherwise
    pub git_root: Option<String>,
    /// Project root: git_root > workspace_folder > file's parent dir
    /// This is the path that should be used as cwd for the agent
    pub project_root: Option<String>,
    pub dependencies: Option<Vec<Dependency>>,
    pub dependency_count: Option<usize>,
    pub additional_dependency_count: Option<usize>,
    pub related_tests: Option<Vec<String>>,
    /// Diagnostics (errors, warnings) from the IDE for this file
    pub diagnostics: Option<Vec<Diagnostic>>,
}

/// A single file in a batch request
#[derive(Debug, Clone, Deserialize)]
pub struct BatchFile {
    /// Path to the file
    pub path: String,
    /// Workspace root directory
    pub workspace: String,
    /// Git repository root if file is in a git repo
    pub git_root: Option<String>,
    /// Project root: git_root > workspace > file's parent dir
    pub project_root: Option<String>,
    /// Optional: start line for selection
    pub line_start: Option<usize>,
    /// Optional: end line for selection
    pub line_end: Option<usize>,
}

/// Batch processing request from IDE extensions
///
/// Note: Only contains file list. Mode, agents, and prompt are selected
/// in the KYCo GUI popup (same UX as single file selection).
#[derive(Debug, Clone, Deserialize)]
pub struct BatchRequest {
    /// Files to process
    pub files: Vec<BatchFile>,
}

/// Control API: create one or more jobs from a file selection.
#[derive(Debug, Clone, Deserialize)]
pub struct ControlJobCreateRequest {
    /// File path (relative to KYCo work_dir, or absolute).
    pub file_path: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub selected_text: Option<String>,
    /// Mode or chain name.
    pub mode: String,
    /// Optional freeform prompt/description.
    pub prompt: Option<String>,
    /// Primary agent id (e.g. "claude"). Ignored if `agents` is provided.
    pub agent: Option<String>,
    /// Optional list of agents for parallel execution (multi-agent group).
    pub agents: Option<Vec<String>>,
    /// If true, set status to queued immediately.
    #[serde(default = "default_true")]
    pub queue: bool,
    /// If true, force running in a git worktree (like Shift+Enter in UI).
    #[serde(default)]
    pub force_worktree: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ControlJobCreateResponse {
    pub job_ids: Vec<JobId>,
    pub group_id: Option<crate::AgentGroupId>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlLogRequest {
    pub message: String,
}

impl SelectionRequest {
    /// Format IDE context as markdown for prompt injection
    pub fn format_ide_context(&self) -> String {
        let mut ctx = String::new();

        ctx.push_str("## IDE Selection Context\n");

        if let Some(ref path) = self.file_path {
            ctx.push_str(&format!("- **File:** `{}`\n", path));
        }

        if let (Some(start), Some(end)) = (self.line_start, self.line_end) {
            ctx.push_str(&format!("- **Lines:** {}-{}\n", start, end));
        }

        // Dependencies
        if let Some(count) = self.dependency_count {
            if count > 0 {
                ctx.push_str(&format!("\n### Dependencies ({} total", count));
                if let Some(additional) = self.additional_dependency_count {
                    if additional > 0 {
                        ctx.push_str(&format!(", showing {}", count - additional));
                    }
                }
                ctx.push_str("):\n");

                if let Some(ref deps) = self.dependencies {
                    for dep in deps {
                        ctx.push_str(&format!("- `{}:{}`\n", dep.file_path, dep.line));
                    }
                }

                if let Some(additional) = self.additional_dependency_count {
                    if additional > 0 {
                        ctx.push_str(&format!("- *...and {} more*\n", additional));
                    }
                }
            }
        }

        // Related Tests
        if let Some(ref tests) = self.related_tests {
            if !tests.is_empty() {
                ctx.push_str("\n### Related Tests:\n");
                for test in tests {
                    ctx.push_str(&format!("- `{}`\n", test));
                }
            }
        }

        // Diagnostics (Errors/Warnings)
        if let Some(ref diagnostics) = self.diagnostics {
            if !diagnostics.is_empty() {
                let errors: Vec<_> = diagnostics
                    .iter()
                    .filter(|d| d.severity == "Error")
                    .collect();
                let warnings: Vec<_> = diagnostics
                    .iter()
                    .filter(|d| d.severity == "Warning")
                    .collect();

                ctx.push_str("\n### Diagnostics:\n");

                if !errors.is_empty() {
                    ctx.push_str(&format!("**Errors ({}):**\n", errors.len()));
                    for diag in errors {
                        ctx.push_str(&format!(
                            "- Line {}: {}{}\n",
                            diag.line,
                            diag.message,
                            diag.code
                                .as_ref()
                                .map(|c| format!(" [{}]", c))
                                .unwrap_or_default()
                        ));
                    }
                }

                if !warnings.is_empty() {
                    ctx.push_str(&format!("**Warnings ({}):**\n", warnings.len()));
                    for diag in warnings {
                        ctx.push_str(&format!(
                            "- Line {}: {}{}\n",
                            diag.line,
                            diag.message,
                            diag.code
                                .as_ref()
                                .map(|c| format!(" [{}]", c))
                                .unwrap_or_default()
                        ));
                    }
                }
            }
        }

        ctx
    }
}

/// Start the HTTP server in a background thread
/// Returns immediately, server runs until program exits
pub fn start_http_server(
    selection_tx: Sender<SelectionRequest>,
    batch_tx: Sender<BatchRequest>,
    port: u16,
    auth_token: Option<String>,
    control: ControlApiState,
) {
    thread::spawn(move || {
        let bind_addr = format!("127.0.0.1:{}", port);
        let server = match Server::http(&bind_addr) {
            Ok(s) => {
                let auth_enabled = auth_token
                    .as_deref()
                    .map_or(false, |t| !t.trim().is_empty());
                info!(
                    "[kyco:http] Server listening on http://{} (auth: {})",
                    bind_addr,
                    if auth_enabled { "enabled" } else { "disabled" }
                );
                s
            }
            Err(e) => {
                error!("[kyco:http] Failed to start server on {}: {}", bind_addr, e);
                return;
            }
        };

        for mut request in server.incoming_requests() {
            let method = request.method().to_string();
            let url = request.url().to_string();
            let path = url.split('?').next().unwrap_or(url.as_str());

            if !is_authorized(&request, auth_token.as_deref()) {
                let response = Response::from_string("{\"error\":\"unauthorized\"}")
                    .with_status_code(401)
                    .with_header(json_content_type());
                let _ = request.respond(response);
                continue;
            }

            match (method.as_str(), path) {
                // IDE extension endpoints
                ("POST", "/selection") => {
                    let body = match read_request_body(&mut request) {
                        Ok(body) => body,
                        Err(response) => {
                            let _ = request.respond(response);
                            continue;
                        }
                    };
                    handle_selection_request(&selection_tx, &body, request);
                }
                ("POST", "/batch") => {
                    let body = match read_request_body(&mut request) {
                        Ok(body) => body,
                        Err(response) => {
                            let _ = request.respond(response);
                            continue;
                        }
                    };
                    handle_batch_request(&batch_tx, &body, request);
                }

                // Control API endpoints (orchestrators / CLI)
                ("GET", "/ctl/ping") => {
                    respond_json(
                        request,
                        200,
                        serde_json::json!({
                            "status": "ok",
                            "version": env!("CARGO_PKG_VERSION"),
                        }),
                    );
                }
                ("GET", "/ctl/jobs") => {
                    handle_control_jobs_list(&control, request);
                }
                ("GET", p) if p.starts_with("/ctl/jobs/") => {
                    handle_control_job_get(&control, p, request);
                }
                ("POST", "/ctl/jobs") => {
                    let body = match read_request_body(&mut request) {
                        Ok(body) => body,
                        Err(response) => {
                            let _ = request.respond(response);
                            continue;
                        }
                    };
                    handle_control_job_create(&control, &body, request);
                }
                ("POST", p) if p.starts_with("/ctl/jobs/") && p.ends_with("/queue") => {
                    handle_control_job_queue(&control, p, request);
                }
                ("POST", "/ctl/log") => {
                    let body = match read_request_body(&mut request) {
                        Ok(body) => body,
                        Err(response) => {
                            let _ = request.respond(response);
                            continue;
                        }
                    };
                    handle_control_log(&control, &body, request);
                }

                _ => {
                    let response = Response::from_string("{\"error\":\"not_found\"}")
                        .with_status_code(404)
                        .with_header(json_content_type());
                    let _ = request.respond(response);
                }
            }
        }
    });
}

fn is_authorized(request: &tiny_http::Request, expected: Option<&str>) -> bool {
    let Some(expected) = expected.filter(|t| !t.trim().is_empty()) else {
        return true;
    };

    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(AUTH_HEADER))
        .map(|h| h.value.as_str() == expected)
        .unwrap_or(false)
}

fn json_content_type() -> tiny_http::Header {
    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()
}

fn default_true() -> bool {
    true
}

fn read_request_body(
    request: &mut tiny_http::Request,
) -> Result<String, Response<std::io::Cursor<Vec<u8>>>> {
    let mut body = String::new();
    let mut reader = request.as_reader().take((MAX_BODY_BYTES + 1) as u64);
    if let Err(e) = reader.read_to_string(&mut body) {
        error!("[kyco:http] Failed to read body: {}", e);
        let response = Response::from_string("{\"error\":\"bad_request\"}")
            .with_status_code(400)
            .with_header(json_content_type());
        return Err(response);
    }

    if body.len() > MAX_BODY_BYTES {
        let response = Response::from_string("{\"error\":\"payload_too_large\"}")
            .with_status_code(413)
            .with_header(json_content_type());
        return Err(response);
    }

    Ok(body)
}

fn respond_json(request: tiny_http::Request, status_code: u16, value: serde_json::Value) {
    let body = serde_json::to_string(&value).unwrap_or_else(|_| "{\"error\":\"serialize\"}".to_string());
    let response = Response::from_string(body)
        .with_status_code(status_code)
        .with_header(json_content_type());
    let _ = request.respond(response);
}

/// Handle POST /selection request
fn handle_selection_request(
    tx: &Sender<SelectionRequest>,
    body: &str,
    request: tiny_http::Request,
) {
    // Parse JSON
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
fn handle_batch_request(tx: &Sender<BatchRequest>, body: &str, request: tiny_http::Request) {
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

fn handle_control_jobs_list(control: &ControlApiState, request: tiny_http::Request) {
    let jobs: Vec<Job> = match control.job_manager.lock() {
        Ok(manager) => manager.jobs().into_iter().cloned().collect(),
        Err(_) => {
            respond_json(request, 500, serde_json::json!({ "error": "job_manager_lock" }));
            return;
        }
    };

    respond_json(request, 200, serde_json::json!({ "jobs": jobs }));
}

fn handle_control_job_get(control: &ControlApiState, path: &str, request: tiny_http::Request) {
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
            respond_json(request, 500, serde_json::json!({ "error": "job_manager_lock" }));
            return;
        }
    };

    let Some(job) = job else {
        respond_json(request, 404, serde_json::json!({ "error": "not_found" }));
        return;
    };

    respond_json(request, 200, serde_json::json!({ "job": job }));
}

fn handle_control_job_queue(control: &ControlApiState, path: &str, request: tiny_http::Request) {
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
            respond_json(request, 500, serde_json::json!({ "error": "job_manager_lock" }));
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

fn handle_control_job_create(control: &ControlApiState, body: &str, request: tiny_http::Request) {
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
        respond_json(request, 400, serde_json::json!({ "error": "missing_file_path" }));
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
        let agent = req
            .agent
            .as_deref()
            .unwrap_or("claude")
            .trim()
            .to_string();
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
        respond_json(request, 500, serde_json::json!({ "error": "create_failed" }));
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

fn handle_control_log(control: &ControlApiState, body: &str, request: tiny_http::Request) {
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
        respond_json(request, 400, serde_json::json!({ "error": "missing_message" }));
        return;
    }

    let _ = control
        .executor_tx
        .send(ExecutorEvent::Log(LogEvent::system(msg.to_string())));

    respond_json(request, 200, serde_json::json!({ "status": "ok" }));
}

fn parse_job_id_from_path(path: &str, suffix: Option<&str>) -> Result<JobId, &'static str> {
    let trimmed = path.trim_end_matches('/');
    let trimmed = match suffix {
        Some(suffix) => trimmed.strip_suffix(&format!("/{suffix}")).ok_or("bad_path")?,
        None => trimmed,
    };

    let id_str = trimmed.rsplit('/').next().ok_or("bad_path")?;
    id_str.parse::<JobId>().map_err(|_| "bad_job_id")
}
