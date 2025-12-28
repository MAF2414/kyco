//! IDE extension request handlers (selection, batch).

use std::sync::mpsc::Sender;
use tiny_http::Response;
use tracing::{error, info};

use super::super::types::{BatchRequest, SelectionRequest};
use super::super::json_content_type;

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
