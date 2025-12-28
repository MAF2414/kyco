//! HTTP server for receiving selection data from IDE extensions
//!
//! Listens on localhost:9876 and accepts:
//! - POST /selection - Single file selection from IDE
//! - POST /batch - Batch processing of multiple files
//! - Control endpoints under /ctl/* (for orchestrators / CLI)

mod handlers;
mod types;

use std::io::Read;
use std::sync::mpsc::Sender;
use std::thread;
use tiny_http::{Response, Server};
use tracing::{error, info};

// Re-export public types for external use
pub use types::{
    BatchFile, BatchRequest, ControlApiState, ControlJobContinueRequest,
    ControlJobContinueResponse, ControlJobCreateRequest, ControlJobCreateResponse,
    ControlJobDeleteRequest, ControlJobDeleteResponse, ControlLogRequest, Dependency, Diagnostic,
    SelectionRequest,
};

use handlers::{
    handle_batch_request, handle_control_config_reload, handle_control_job_abort,
    handle_control_job_continue, handle_control_job_create, handle_control_job_delete,
    handle_control_job_get, handle_control_job_queue, handle_control_jobs_list, handle_control_log,
    handle_selection_request,
};

const AUTH_HEADER: &str = "X-KYCO-Token";
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024; // 2 MiB

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
                ("POST", p) if p.starts_with("/ctl/jobs/") && p.ends_with("/abort") => {
                    handle_control_job_abort(&control, p, request);
                }
                ("POST", p) if p.starts_with("/ctl/jobs/") && p.ends_with("/delete") => {
                    let body = match read_request_body(&mut request) {
                        Ok(body) => body,
                        Err(response) => {
                            let _ = request.respond(response);
                            continue;
                        }
                    };
                    handle_control_job_delete(&control, p, &body, request);
                }
                ("POST", p) if p.starts_with("/ctl/jobs/") && p.ends_with("/continue") => {
                    let body = match read_request_body(&mut request) {
                        Ok(body) => body,
                        Err(response) => {
                            let _ = request.respond(response);
                            continue;
                        }
                    };
                    handle_control_job_continue(&control, p, &body, request);
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
                ("POST", "/ctl/config/reload") => {
                    handle_control_config_reload(&control, request);
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

pub(crate) fn json_content_type() -> tiny_http::Header {
    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()
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

pub(crate) fn respond_json(request: tiny_http::Request, status_code: u16, value: serde_json::Value) {
    let body =
        serde_json::to_string(&value).unwrap_or_else(|_| "{\"error\":\"serialize\"}".to_string());
    let response = Response::from_string(body)
        .with_status_code(status_code)
        .with_header(json_content_type());
    let _ = request.respond(response);
}
