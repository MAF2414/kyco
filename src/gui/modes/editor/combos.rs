//! Combo box helpers for mode editor

use eframe::egui;

/// Render the session mode combo box (oneshot/session)
pub fn render_session_mode_combo(ui: &mut egui::Ui, value: &mut String) {
    egui::ComboBox::from_id_salt("mode_session_mode")
        .selected_text(&**value)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, "oneshot".to_string(), "oneshot");
            ui.selectable_value(value, "session".to_string(), "session");
        });
}

/// Render the use worktree combo box (global/always/never)
pub fn render_use_worktree_combo(ui: &mut egui::Ui, value: &mut Option<bool>) {
    let display_text = match value {
        None => "global",
        Some(true) => "always",
        Some(false) => "never",
    };
    egui::ComboBox::from_id_salt("mode_use_worktree")
        .selected_text(display_text)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, "global")
                .on_hover_text("Use the global worktree setting");
            ui.selectable_value(value, Some(true), "always")
                .on_hover_text("Always run in a git worktree");
            ui.selectable_value(value, Some(false), "never")
                .on_hover_text("Never run in a worktree (even if global is enabled)");
        });
}

/// Render the Claude permissions combo box
pub fn render_claude_permission_combo(ui: &mut egui::Ui, value: &mut String) {
    egui::ComboBox::from_id_salt("mode_claude_permission")
        .selected_text(&**value)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, "auto".to_string(), "auto");
            ui.selectable_value(value, "default".to_string(), "default");
            ui.selectable_value(value, "acceptEdits".to_string(), "acceptEdits");
            ui.selectable_value(value, "bypassPermissions".to_string(), "bypassPermissions");
            ui.selectable_value(value, "plan".to_string(), "plan");
        });
}

/// Render the Codex sandbox combo box
pub fn render_codex_sandbox_combo(ui: &mut egui::Ui, value: &mut String) {
    egui::ComboBox::from_id_salt("mode_codex_sandbox")
        .selected_text(&**value)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, "auto".to_string(), "auto");
            ui.selectable_value(value, "read-only".to_string(), "read-only");
            ui.selectable_value(value, "workspace-write".to_string(), "workspace-write");
            ui.selectable_value(value, "danger-full-access".to_string(), "danger-full-access");
        });
}
