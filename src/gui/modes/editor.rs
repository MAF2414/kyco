//! Mode editor form rendering

use eframe::egui::{self, RichText, ScrollArea};

use super::persistence::{delete_mode_from_config, save_mode_to_config};
use super::state::ModeEditorState;
use crate::gui::animations::animated_button;
use crate::gui::app::{ACCENT_GREEN, ACCENT_RED, TEXT_MUTED, TEXT_PRIMARY};

/// Render the mode editor form
pub fn render_mode_editor(ui: &mut egui::Ui, state: &mut ModeEditorState<'_>, mode_name: &str) {
    let is_new = mode_name == "__new__";
    let title = if is_new {
        "Create New Mode".to_string()
    } else {
        format!("Edit Mode: {}", mode_name)
    };

    ui.label(RichText::new(&title).monospace().color(TEXT_PRIMARY));
    ui.add_space(16.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Name:").color(TEXT_MUTED));
                if is_new {
                    ui.add(
                        egui::TextEdit::singleline(state.mode_edit_name)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .desired_width(200.0),
                    );
                } else {
                    ui.label(
                        RichText::new(&*state.mode_edit_name)
                            .monospace()
                            .color(ACCENT_GREEN),
                    );
                }
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Aliases:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.mode_edit_aliases)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("r, ref")
                        .desired_width(200.0),
                );
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Default Agent:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.mode_edit_agent)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("claude (optional)")
                        .desired_width(200.0),
                );
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Session Mode:").color(TEXT_MUTED));
                egui::ComboBox::from_id_salt("mode_session_mode")
                    .selected_text(&**state.mode_edit_session_mode)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            state.mode_edit_session_mode,
                            "oneshot".to_string(),
                            "oneshot",
                        );
                        ui.selectable_value(
                            state.mode_edit_session_mode,
                            "session".to_string(),
                            "session",
                        );
                    });
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Max Turns:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.mode_edit_max_turns)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("0 (unlimited)")
                        .desired_width(120.0),
                );
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Model:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.mode_edit_model)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("optional")
                        .desired_width(200.0),
                );
            });
            ui.add_space(16.0);

            ui.separator();
            ui.add_space(8.0);
            ui.label(RichText::new("SDK Options").monospace().color(TEXT_PRIMARY));
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Claude Permissions:").color(TEXT_MUTED));
                egui::ComboBox::from_id_salt("mode_claude_permission")
                    .selected_text(&**state.mode_edit_claude_permission)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            state.mode_edit_claude_permission,
                            "auto".to_string(),
                            "auto",
                        );
                        ui.selectable_value(
                            state.mode_edit_claude_permission,
                            "default".to_string(),
                            "default",
                        );
                        ui.selectable_value(
                            state.mode_edit_claude_permission,
                            "acceptEdits".to_string(),
                            "acceptEdits",
                        );
                        ui.selectable_value(
                            state.mode_edit_claude_permission,
                            "bypassPermissions".to_string(),
                            "bypassPermissions",
                        );
                        ui.selectable_value(
                            state.mode_edit_claude_permission,
                            "plan".to_string(),
                            "plan",
                        );
                    });
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Codex Sandbox:").color(TEXT_MUTED));
                egui::ComboBox::from_id_salt("mode_codex_sandbox")
                    .selected_text(&**state.mode_edit_codex_sandbox)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            state.mode_edit_codex_sandbox,
                            "auto".to_string(),
                            "auto",
                        );
                        ui.selectable_value(
                            state.mode_edit_codex_sandbox,
                            "read-only".to_string(),
                            "read-only",
                        );
                        ui.selectable_value(
                            state.mode_edit_codex_sandbox,
                            "workspace-write".to_string(),
                            "workspace-write",
                        );
                        ui.selectable_value(
                            state.mode_edit_codex_sandbox,
                            "danger-full-access".to_string(),
                            "danger-full-access",
                        );
                    });
            });
            ui.add_space(16.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Allowed Tools:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.mode_edit_allowed_tools)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("Read, Grep, Glob (empty = all)")
                        .desired_width(300.0),
                );
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Disallowed Tools:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.mode_edit_disallowed_tools)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("Write, Edit")
                        .desired_width(300.0),
                );
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.checkbox(state.mode_edit_readonly, "");
                ui.label(
                    RichText::new("Read-only (auto-sets disallowed: Write, Edit)")
                        .color(TEXT_MUTED),
                );
            });
            ui.add_space(16.0);

            ui.label(RichText::new("Prompt Template:").color(TEXT_MUTED));
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Placeholders: {file}, {line}, {target}, {mode}, {description}, {scope_type}",
                )
                .small()
                .color(TEXT_MUTED),
            );
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::multiline(state.mode_edit_prompt)
                    .font(egui::TextStyle::Monospace)
                    .text_color(TEXT_PRIMARY)
                    .desired_width(f32::INFINITY)
                    .desired_rows(8),
            );
            ui.add_space(16.0);

            ui.label(RichText::new("System Prompt:").color(TEXT_MUTED));
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::multiline(state.mode_edit_system_prompt)
                    .font(egui::TextStyle::Monospace)
                    .text_color(TEXT_PRIMARY)
                    .desired_width(f32::INFINITY)
                    .desired_rows(12),
            );
            ui.add_space(16.0);

            ui.separator();
            ui.add_space(8.0);
            ui.label(RichText::new("Chain Integration (for workflows)").color(TEXT_PRIMARY));
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Output States:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.mode_edit_output_states)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("issues_found, no_issues")
                        .desired_width(300.0),
                );
            });
            ui.label(
                RichText::new("Comma-separated states this mode can output (for chain triggers)")
                    .small()
                    .color(TEXT_MUTED),
            );
            ui.add_space(8.0);

            ui.label(RichText::new("State Prompt:").color(TEXT_MUTED));
            ui.add_space(4.0);
            ui.label(
                RichText::new("Custom instruction for outputting state (auto-generated if empty)")
                    .small()
                    .color(TEXT_MUTED),
            );
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::multiline(state.mode_edit_state_prompt)
                    .font(egui::TextStyle::Monospace)
                    .text_color(TEXT_PRIMARY)
                    .hint_text("Output: state: issues_found if problems, state: no_issues if good")
                    .desired_width(f32::INFINITY)
                    .desired_rows(2),
            );
            ui.add_space(16.0);

            if let Some((msg, is_error)) = &state.mode_edit_status {
                let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
                ui.label(RichText::new(msg.as_str()).color(color));
                ui.add_space(8.0);
            }

            ui.horizontal(|ui| {
                if animated_button(ui, "Save to Config", ACCENT_GREEN, "mode_save_btn").clicked() {
                    save_mode_to_config(state, is_new);
                }
                if !is_new {
                    ui.add_space(16.0);
                    if animated_button(ui, "Delete", ACCENT_RED, "mode_delete_btn").clicked() {
                        delete_mode_from_config(state);
                    }
                }
            });
        });
}
