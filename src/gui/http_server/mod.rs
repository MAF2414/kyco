//! HTTP server for receiving selection data from IDE extensions
//!
//! Listens on localhost:9876 and accepts POST /selection with JSON body

use serde::Deserialize;
use std::io::Read;
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
pub fn start_http_server(tx: Sender<SelectionRequest>) {
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

            // Only handle POST /selection
            if method != "POST" || url != "/selection" {
                let response = Response::from_string("Not Found")
                    .with_status_code(404);
                let _ = request.respond(response);
                continue;
            }

            // Read body
            let mut body = String::new();
            if let Err(e) = request.as_reader().read_to_string(&mut body) {
                error!("[kyco:http] Failed to read body: {}", e);
                let response = Response::from_string("Bad Request")
                    .with_status_code(400);
                let _ = request.respond(response);
                continue;
            }

            // Log raw body for debugging
            info!("[kyco:http] Raw body: {}", body);
            eprintln!("[kyco:http] === RECEIVED FROM EXTENSION ===");
            eprintln!("{}", body);
            eprintln!("[kyco:http] ================================");

            // Parse JSON
            match serde_json::from_str::<SelectionRequest>(&body) {
                Ok(selection) => {
                    eprintln!("[kyco:http] Parsed selection:");
                    eprintln!("  file_path: {:?}", selection.file_path);
                    eprintln!("  line_start: {:?}", selection.line_start);
                    eprintln!("  line_end: {:?}", selection.line_end);
                    eprintln!("  selected_text length: {:?}", selection.selected_text.as_ref().map(|s| s.len()));
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

                    let response = Response::from_string("{\"status\":\"ok\"}")
                        .with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"application/json"[..],
                            )
                            .unwrap(),
                        );
                    let _ = request.respond(response);
                }
                Err(e) => {
                    error!("[kyco:http] Invalid JSON: {}", e);
                    eprintln!("[kyco:http] JSON parse error: {}", e);
                    let response = Response::from_string(format!("{{\"error\":\"{}\"}}", e))
                        .with_status_code(400);
                    let _ = request.respond(response);
                }
            }
        }
    });
}
