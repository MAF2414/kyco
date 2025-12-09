//! HTTP server for receiving selection data from IDE extensions
//!
//! Listens on localhost:9876 and accepts:
//! - POST /selection - Single file selection from IDE
//! - POST /batch - Batch processing of multiple files

use serde::Deserialize;
use std::sync::mpsc::Sender;
use std::thread;
use tiny_http::{Response, Server};
use tracing::{error, info};

/// Dependency location from IDE
#[derive(Debug, Clone, Deserialize)]
pub struct Dependency {
    pub file_path: String,
    pub line: usize,
}

/// Selection data received from IDE extensions
#[derive(Debug, Clone, Deserialize)]
pub struct SelectionRequest {
    pub file_path: Option<String>,
    pub selected_text: Option<String>,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub workspace: Option<String>,
    pub dependencies: Option<Vec<Dependency>>,
    pub dependency_count: Option<usize>,
    pub additional_dependency_count: Option<usize>,
    pub related_tests: Option<Vec<String>>,
}

/// A single file in a batch request
#[derive(Debug, Clone, Deserialize)]
pub struct BatchFile {
    /// Path to the file
    pub path: String,
    /// Workspace root directory
    pub workspace: String,
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

        ctx
    }
}

/// Start the HTTP server in a background thread
/// Returns immediately, server runs until program exits
pub fn start_http_server(
    selection_tx: Sender<SelectionRequest>,
    batch_tx: Sender<BatchRequest>,
) {
    thread::spawn(move || {
        let server = match Server::http("127.0.0.1:9876") {
            Ok(s) => {
                info!("[kyco:http] Server listening on http://127.0.0.1:9876");
                s
            }
            Err(e) => {
                error!("[kyco:http] Failed to start server: {}", e);
                return;
            }
        };

        for mut request in server.incoming_requests() {
            let method = request.method().to_string();
            let url = request.url().to_string();

            // Read body for POST requests
            if method == "POST" {
                let mut body = String::new();
                if let Err(e) = request.as_reader().read_to_string(&mut body) {
                    error!("[kyco:http] Failed to read body: {}", e);
                    let response = Response::from_string("Bad Request").with_status_code(400);
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

/// Handle POST /selection request
fn handle_selection_request(
    tx: &Sender<SelectionRequest>,
    body: &str,
    request: tiny_http::Request,
) {
    // Log raw body for debugging
    info!("[kyco:http] Raw body: {}", body);
    eprintln!("[kyco:http] === RECEIVED FROM EXTENSION ===");
    eprintln!("{}", body);
    eprintln!("[kyco:http] ================================");

    // Parse JSON
    match serde_json::from_str::<SelectionRequest>(body) {
        Ok(selection) => {
            eprintln!("[kyco:http] Parsed selection:");
            eprintln!("  file_path: {:?}", selection.file_path);
            eprintln!("  line_start: {:?}", selection.line_start);
            eprintln!("  line_end: {:?}", selection.line_end);
            eprintln!(
                "  selected_text length: {:?}",
                selection.selected_text.as_ref().map(|s| s.len())
            );
            if let Some(ref text) = selection.selected_text {
                let preview: String = text.chars().take(200).collect();
                eprintln!("  selected_text preview: {:?}", preview);
            }
            eprintln!("  workspace: {:?}", selection.workspace);

            info!(
                "[kyco:http] Received selection: file={:?}, lines={:?}-{:?}, text_len={:?}",
                selection.file_path,
                selection.line_start,
                selection.line_end,
                selection.selected_text.as_ref().map(|s| s.len())
            );

            // Send to GUI
            if let Err(e) = tx.send(selection) {
                error!("[kyco:http] Failed to send to GUI: {}", e);
            }

            let response = Response::from_string("{\"status\":\"ok\"}").with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap(),
            );
            let _ = request.respond(response);
        }
        Err(e) => {
            error!("[kyco:http] Invalid JSON: {}", e);
            eprintln!("[kyco:http] JSON parse error: {}", e);
            let response =
                Response::from_string(format!("{{\"error\":\"{}\"}}", e)).with_status_code(400);
            let _ = request.respond(response);
        }
    }
}

/// Handle POST /batch request
fn handle_batch_request(tx: &Sender<BatchRequest>, body: &str, request: tiny_http::Request) {
    info!("[kyco:http] Batch request received");
    eprintln!("[kyco:http] === BATCH REQUEST ===");
    eprintln!("{}", body);
    eprintln!("[kyco:http] ====================");

    match serde_json::from_str::<BatchRequest>(body) {
        Ok(batch) => {
            info!("[kyco:http] Batch: {} files", batch.files.len());
            eprintln!("[kyco:http] Parsed batch:");
            eprintln!("  files: {} total", batch.files.len());
            for (i, f) in batch.files.iter().take(5).enumerate() {
                eprintln!("    [{}] {}", i, f.path);
            }
            if batch.files.len() > 5 {
                eprintln!("    ... and {} more", batch.files.len() - 5);
            }

            // Send to GUI (will open batch popup for mode/agent/prompt selection)
            if let Err(e) = tx.send(batch) {
                error!("[kyco:http] Failed to send batch to GUI: {}", e);
            }

            let response = Response::from_string("{\"status\":\"ok\"}").with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap(),
            );
            let _ = request.respond(response);
        }
        Err(e) => {
            error!("[kyco:http] Invalid batch JSON: {}", e);
            eprintln!("[kyco:http] Batch JSON parse error: {}", e);
            let response =
                Response::from_string(format!("{{\"error\":\"{}\"}}", e)).with_status_code(400);
            let _ = request.respond(response);
        }
    }
}
