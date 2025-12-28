//! Miscellaneous control handlers: log, config reload.

use super::super::types::{ControlApiState, ControlLogRequest};
use super::super::respond_json;
use super::ExecutorEvent;
use crate::config::Config;
use crate::LogEvent;

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
