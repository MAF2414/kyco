//! Permission popup handling for KycoApp update loop

use crate::LogEvent;
use crate::agent::bridge::{ToolApprovalResponse, ToolDecision};
use crate::gui::app::KycoApp;
use crate::gui::permission::{PermissionAction, render_permission_popup};
use eframe::egui;
use std::time::Duration;

impl KycoApp {
    /// Poll the Bridge for pending tool approvals (fallback in case the streaming event was missed).
    pub(crate) fn poll_pending_tool_approvals(&mut self) {
        // Avoid spamming localhost with requests every frame.
        if self.last_permission_poll.elapsed() < Duration::from_millis(500) {
            return;
        }
        self.last_permission_poll = std::time::Instant::now();

        let pending = match self.bridge_client.get_pending_tool_approvals() {
            Ok(pending) => pending,
            Err(_) => return,
        };

        for approval in pending {
            if self.permission_state.contains_request_id(&approval.request_id) {
                continue;
            }

            let tool_input = approval
                .tool_input
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<std::collections::HashMap<String, serde_json::Value>>()
                })
                .unwrap_or_default();

            let request = crate::gui::permission::PermissionRequest {
                request_id: approval.request_id,
                session_id: approval.session_id,
                tool_name: approval.tool_name,
                tool_input,
                received_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            };

            self.permission_state.add_request(request);
        }
    }

    /// Render the permission popup modal (on top of everything)
    pub(crate) fn render_permission_popup_modal(&mut self, ctx: &egui::Context) {
        if let Some(action) = render_permission_popup(ctx, &mut self.permission_state) {
            match action {
                PermissionAction::Approve(request_id) => {
                    // Send approval to Bridge via HTTP POST /claude/tool-approval
                    let response = ToolApprovalResponse {
                        request_id: request_id.clone(),
                        decision: ToolDecision::Allow,
                        reason: None,
                        modified_input: None,
                    };
                    match self.bridge_client.send_tool_approval(&response) {
                        Ok(true) => {
                            self.logs.push(LogEvent::system(format!(
                                "✓ Approved tool request: {}",
                                &request_id[..12.min(request_id.len())]
                            )));
                        }
                        Ok(false) => {
                            self.logs.push(LogEvent::error(format!(
                                "Tool approval rejected by bridge: {}",
                                &request_id[..12.min(request_id.len())]
                            )));
                        }
                        Err(e) => {
                            self.logs.push(LogEvent::error(format!(
                                "Failed to send tool approval: {}",
                                e
                            )));
                        }
                    }
                    self.permission_state.next_request();
                }
                PermissionAction::ApproveAll(request_ids) => {
                    let mut approved = 0usize;
                    for request_id in &request_ids {
                        let response = ToolApprovalResponse {
                            request_id: request_id.clone(),
                            decision: ToolDecision::Allow,
                            reason: None,
                            modified_input: None,
                        };
                        match self.bridge_client.send_tool_approval(&response) {
                            Ok(true) => {
                                approved += 1;
                            }
                            Ok(false) => {
                                self.logs.push(LogEvent::error(format!(
                                    "Tool approval rejected by bridge: {}",
                                    &request_id[..12.min(request_id.len())]
                                )));
                            }
                            Err(e) => {
                                self.logs.push(LogEvent::error(format!(
                                    "Failed to send tool approval: {}",
                                    e
                                )));
                            }
                        }
                    }

                    self.logs.push(LogEvent::system(format!(
                        "✓ Approved {} tool request(s)",
                        approved
                    )));

                    // Clear popup state
                    self.permission_state.current_request = None;
                    self.permission_state.pending_requests.clear();
                    self.permission_state.visible = false;
                    self.permission_state.should_focus = false;
                }
                PermissionAction::Deny(request_id, reason) => {
                    // Send denial to Bridge via HTTP POST /claude/tool-approval
                    let response = ToolApprovalResponse {
                        request_id: request_id.clone(),
                        decision: ToolDecision::Deny,
                        reason: Some(reason.clone()),
                        modified_input: None,
                    };
                    match self.bridge_client.send_tool_approval(&response) {
                        Ok(_) => {
                            self.logs.push(LogEvent::system(format!(
                                "✗ Denied tool request: {} ({})",
                                &request_id[..12.min(request_id.len())],
                                reason
                            )));
                        }
                        Err(e) => {
                            self.logs.push(LogEvent::error(format!(
                                "Failed to send tool denial: {}",
                                e
                            )));
                        }
                    }
                    self.permission_state.next_request();
                }
                PermissionAction::Dismiss(request_id) => {
                    // Treat dismiss as deny
                    let response = ToolApprovalResponse {
                        request_id: request_id.clone(),
                        decision: ToolDecision::Deny,
                        reason: Some("User dismissed".to_string()),
                        modified_input: None,
                    };
                    let _ = self.bridge_client.send_tool_approval(&response);
                    self.logs.push(LogEvent::system(format!(
                        "Dismissed tool request: {}",
                        &request_id[..12.min(request_id.len())]
                    )));
                    self.permission_state.next_request();
                }
            }
        }

        // Bring app to foreground if needed
        if self.permission_state.should_focus {
            self.permission_state.should_focus = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
    }
}
