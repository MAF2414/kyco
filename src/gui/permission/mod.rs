//! Permission popup for tool approval requests
//!
//! When Claude needs permission to use a tool (in `default` or `acceptEdits` mode),
//! this popup is shown to the user to approve or deny the request.

use eframe::egui::{self, Color32, Id, RichText, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::gui::app::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_RED, BG_SECONDARY, TEXT_DIM, TEXT_MUTED,
    TEXT_PRIMARY,
};

/// A pending permission request from the Bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    /// Unique request ID (must be sent back with response)
    pub request_id: String,
    /// Session/job ID this request belongs to
    pub session_id: String,
    /// Tool name (e.g., "Bash", "Write", "Edit")
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: HashMap<String, serde_json::Value>,
    /// Timestamp when request was received
    pub received_at: u64,
}

/// User's decision on a permission request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Deny,
}

/// Actions that can be triggered from the permission popup
#[derive(Debug, Clone)]
pub enum PermissionAction {
    /// User approved the request
    Approve(String), // request_id
    /// User approved this and all pending requests
    ApproveAll(Vec<String>), // request_ids
    /// User denied the request
    Deny(String, String), // request_id, reason
    /// User dismissed the popup (treat as deny)
    Dismiss(String), // request_id
}

/// State for the permission popup
pub struct PermissionPopupState {
    /// Currently displayed permission request
    pub current_request: Option<PermissionRequest>,
    /// Queue of pending requests (if multiple)
    pub pending_requests: Vec<PermissionRequest>,
    /// Whether the popup is visible
    pub visible: bool,
    /// Whether we should bring app to foreground
    pub should_focus: bool,
}

impl Default for PermissionPopupState {
    fn default() -> Self {
        Self {
            current_request: None,
            pending_requests: Vec::new(),
            visible: false,
            should_focus: false,
        }
    }
}

impl PermissionPopupState {
    /// Add a new permission request
    pub fn add_request(&mut self, request: PermissionRequest) {
        if self.current_request.is_none() {
            self.current_request = Some(request);
            self.visible = true;
            self.should_focus = true;
        } else {
            self.pending_requests.push(request);
        }
    }

    /// Move to the next request (after handling current one)
    pub fn next_request(&mut self) {
        self.current_request = self.pending_requests.pop();
        self.visible = self.current_request.is_some();
        self.should_focus = self.visible;
    }

    /// Get total pending count (including current)
    pub fn pending_count(&self) -> usize {
        let current = if self.current_request.is_some() { 1 } else { 0 };
        current + self.pending_requests.len()
    }
}

/// Render the permission popup and return any action triggered by the user
pub fn render_permission_popup(
    ctx: &egui::Context,
    state: &mut PermissionPopupState,
) -> Option<PermissionAction> {
    if !state.visible {
        return None;
    }

    let request = match &state.current_request {
        Some(r) => r,
        None => return None,
    };

    let mut action: Option<PermissionAction> = None;

    // Animate popup fade-in
    let fade_alpha = ctx.animate_bool_with_time(Id::new("permission_popup_fade"), true, 0.15);

    // Orange/warning themed frame
    let frame = egui::Frame::window(&ctx.style())
        .fill(Color32::from_rgba_unmultiplied(
            42,
            36,
            30,
            (fade_alpha * 250.0) as u8,
        ))
        .stroke(Stroke::new(
            2.0,
            Color32::from_rgba_unmultiplied(255, 160, 0, (fade_alpha * 200.0) as u8),
        ));

    egui::Window::new("🔐 Permission Required")
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(500.0, 350.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_opacity(fade_alpha);
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 12.0);

            // Header
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("⚠️").size(24.0));
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Tool Permission Request")
                            .size(18.0)
                            .strong()
                            .color(Color32::from_rgb(255, 200, 100)),
                    );
                    ui.label(
                        RichText::new(format!("Session: {}", truncate_id(&request.session_id)))
                            .size(12.0)
                            .color(TEXT_DIM),
                    );
                });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Tool info
            egui::Frame::new()
                .fill(BG_SECONDARY)
                .inner_margin(12.0)
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Tool:").color(TEXT_MUTED));
                        ui.label(
                            RichText::new(&request.tool_name)
                                .size(16.0)
                                .strong()
                                .color(get_tool_color(&request.tool_name)),
                        );
                    });

                    ui.add_space(8.0);

                    // Show relevant tool input
                    ui.label(RichText::new("Parameters:").color(TEXT_MUTED));

                    egui::ScrollArea::vertical()
                        .max_height(120.0)
                        .show(ui, |ui| {
                            render_tool_input(ui, &request.tool_name, &request.tool_input);
                        });
                });

            ui.add_space(12.0);

            // Pending count indicator
            if state.pending_requests.len() > 0 {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "📋 {} more request(s) pending",
                            state.pending_requests.len()
                        ))
                        .size(12.0)
                        .color(TEXT_DIM),
                    );
                });
            }

            ui.add_space(8.0);

            // Action buttons
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Deny button (red)
                    if ui
                        .add(
                            egui::Button::new(RichText::new("✗ Deny").color(Color32::WHITE))
                                .fill(ACCENT_RED)
                                .min_size(Vec2::new(100.0, 36.0)),
                        )
                        .clicked()
                    {
                        action = Some(PermissionAction::Deny(
                            request.request_id.clone(),
                            "User denied permission".to_string(),
                        ));
                    }

                    ui.add_space(12.0);

                    // Allow button (green)
                    if ui
                        .add(
                            egui::Button::new(RichText::new("✓ Allow").color(Color32::WHITE))
                                .fill(ACCENT_GREEN)
                                .min_size(Vec2::new(100.0, 36.0)),
                        )
                        .clicked()
                    {
                        action = Some(PermissionAction::Approve(request.request_id.clone()));
                    }

                    ui.add_space(12.0);

                    // Allow All button (for remaining requests)
                    if state.pending_requests.len() > 0 {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("✓✓ Allow All").color(Color32::WHITE),
                                )
                                .fill(ACCENT_CYAN)
                                .min_size(Vec2::new(100.0, 36.0)),
                            )
                            .on_hover_text("Allow this and all pending requests")
                            .clicked()
                        {
                            let mut request_ids =
                                Vec::with_capacity(1 + state.pending_requests.len());
                            request_ids.push(request.request_id.clone());
                            request_ids.extend(
                                state.pending_requests.iter().map(|r| r.request_id.clone()),
                            );
                            action = Some(PermissionAction::ApproveAll(request_ids));
                        }
                    }
                });
            });
        });

    // Handle close on Escape
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        if let Some(req) = &state.current_request {
            action = Some(PermissionAction::Dismiss(req.request_id.clone()));
        }
    }

    action
}

/// Get color for tool name
fn get_tool_color(tool_name: &str) -> Color32 {
    match tool_name {
        "Bash" => ACCENT_RED,     // Dangerous
        "Write" => ACCENT_PURPLE, // File modification
        "Edit" => ACCENT_PURPLE,  // File modification
        "Read" => ACCENT_GREEN,   // Safe
        "Glob" => ACCENT_GREEN,   // Safe
        "Grep" => ACCENT_GREEN,   // Safe
        _ => ACCENT_CYAN,         // Unknown
    }
}

/// Render tool-specific input display
fn render_tool_input(
    ui: &mut egui::Ui,
    tool_name: &str,
    input: &HashMap<String, serde_json::Value>,
) {
    match tool_name {
        "Bash" => {
            // Show the command being executed
            if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                ui.add_space(4.0);
                egui::Frame::new()
                    .fill(Color32::from_rgb(20, 20, 30))
                    .inner_margin(8.0)
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("$ ")
                                .color(ACCENT_GREEN)
                                .family(egui::FontFamily::Monospace),
                        );
                        ui.label(
                            RichText::new(cmd)
                                .color(TEXT_PRIMARY)
                                .family(egui::FontFamily::Monospace),
                        );
                    });
            }
        }
        "Write" | "Edit" => {
            // Show file path and preview
            if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📄 File:").color(TEXT_MUTED));
                    ui.label(
                        RichText::new(path)
                            .color(ACCENT_CYAN)
                            .family(egui::FontFamily::Monospace),
                    );
                });
            }
            if let Some(content) = input.get("content").and_then(|v| v.as_str()) {
                ui.add_space(4.0);
                let preview = if content.len() > 200 {
                    format!("{}...", &content[..200])
                } else {
                    content.to_string()
                };
                egui::Frame::new()
                    .fill(Color32::from_rgb(20, 20, 30))
                    .inner_margin(8.0)
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(preview)
                                .color(TEXT_DIM)
                                .family(egui::FontFamily::Monospace)
                                .size(11.0),
                        );
                    });
            }
        }
        _ => {
            // Generic JSON display
            for (key, value) in input {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}:", key)).color(TEXT_MUTED));
                    let value_str = match value {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    let display = if value_str.len() > 100 {
                        format!("{}...", &value_str[..100])
                    } else {
                        value_str
                    };
                    ui.label(
                        RichText::new(display)
                            .color(TEXT_PRIMARY)
                            .family(egui::FontFamily::Monospace),
                    );
                });
            }
        }
    }
}

/// Truncate a session ID for display
fn truncate_id(id: &str) -> String {
    if id.len() > 12 {
        format!("{}...", &id[..12])
    } else {
        id.to_string()
    }
}
