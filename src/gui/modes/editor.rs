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
            // Name (only editable for new modes)
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

            // Aliases
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

            // Default agent
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

            // Allowed tools
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

            // Disallowed tools
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

            // Read-only toggle (convenience)
            ui.horizontal(|ui| {
                ui.checkbox(state.mode_edit_readonly, "");
                ui.label(
                    RichText::new("Read-only (auto-sets disallowed: Write, Edit)").color(TEXT_MUTED),
                );
            });
            ui.add_space(16.0);

            // Prompt template
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

            // System prompt
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

            // Status message
            if let Some((msg, is_error)) = &state.mode_edit_status {
                let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
                ui.label(RichText::new(msg.as_str()).color(color));
                ui.add_space(8.0);
            }

            // Save button
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
