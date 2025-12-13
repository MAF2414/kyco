//! HTTP server for receiving selection data from IDE extensions
//!
//! Listens on localhost:9876 and accepts:
//! - POST /selection - Single file selection from IDE
//! - POST /batch - Batch processing of multiple files

use serde::Deserialize;
use std::io::Read;
use std::sync::mpsc::Sender;
use std::thread;
use tiny_http::{Response, Server};
use tracing::{error, info};

const AUTH_HEADER: &str = "X-KYCO-Token";
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024; // 2 MiB

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
                let errors: Vec<_> = diagnostics.iter().filter(|d| d.severity == "Error").collect();
                let warnings: Vec<_> = diagnostics.iter().filter(|d| d.severity == "Warning").collect();

                ctx.push_str("\n### Diagnostics:\n");

                if !errors.is_empty() {
                    ctx.push_str(&format!("**Errors ({}):**\n", errors.len()));
                    for diag in errors {
                        ctx.push_str(&format!(
                            "- Line {}: {}{}\n",
                            diag.line,
                            diag.message,
                            diag.code.as_ref().map(|c| format!(" [{}]", c)).unwrap_or_default()
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
                            diag.code.as_ref().map(|c| format!(" [{}]", c)).unwrap_or_default()
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
) {
    thread::spawn(move || {
        let bind_addr = format!("127.0.0.1:{}", port);
        let server = match Server::http(&bind_addr) {
            Ok(s) => {
                let auth_enabled = auth_token.as_deref().map_or(false, |t| !t.trim().is_empty());
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

            // Read body for POST requests
            if method == "POST" {
                if !is_authorized(&request, auth_token.as_deref()) {
                    let response = Response::from_string("{\"error\":\"unauthorized\"}")
                        .with_status_code(401)
                        .with_header(json_content_type());
                    let _ = request.respond(response);
                    continue;
                }

                let mut body = String::new();
                let mut reader = request.as_reader().take((MAX_BODY_BYTES + 1) as u64);
                if let Err(e) = reader.read_to_string(&mut body) {
                    error!("[kyco:http] Failed to read body: {}", e);
                    let response = Response::from_string("{\"error\":\"bad_request\"}")
                        .with_status_code(400)
                        .with_header(json_content_type());
                    let _ = request.respond(response);
                    continue;
                }

                if body.len() > MAX_BODY_BYTES {
                    let response = Response::from_string("{\"error\":\"payload_too_large\"}")
                        .with_status_code(413)
                        .with_header(json_content_type());
                    let _ = request.respond(response);
                    continue;
                }

                match url.as_str() {
                    "/selection" => {
                        handle_selection_request(&selection_tx, &body, request);
                    }
                    "/batch" => {
                        handle_batch_request(&batch_tx, &body, request);
                    }
                    _ => {
                        let response = Response::from_string("Not Found").with_status_code(404);
                        let _ = request.respond(response);
                    }
                }
            } else {
                let response = Response::from_string("Method Not Allowed").with_status_code(405);
                let _ = request.respond(response);
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

            let response = Response::from_string("{\"status\":\"ok\"}").with_header(json_content_type());
            let _ = request.respond(response);
        }
        Err(e) => {
            error!("[kyco:http] Invalid JSON: {}", e);
            let response =
                Response::from_string(format!("{{\"error\":\"{}\"}}", e)).with_status_code(400).with_header(json_content_type());
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

            let response = Response::from_string("{\"status\":\"ok\"}").with_header(json_content_type());
            let _ = request.respond(response);
        }
        Err(e) => {
            error!("[kyco:http] Invalid batch JSON: {}", e);
            let response =
                Response::from_string(format!("{{\"error\":\"{}\"}}", e)).with_status_code(400).with_header(json_content_type());
            let _ = request.respond(response);
        }
    }
}
