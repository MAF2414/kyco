//! Agent editor form rendering

use eframe::egui::{self, RichText, ScrollArea};

use super::persistence::{delete_agent_from_config, save_agent_to_config};
use super::state::AgentEditorState;
use crate::gui::animations::animated_button;
use crate::gui::theme::{ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, TEXT_MUTED, TEXT_PRIMARY};

/// Render the agent editor form
pub fn render_agent_editor(ui: &mut egui::Ui, state: &mut AgentEditorState<'_>, agent_name: &str) {
    let is_new = agent_name == "__new__";
    let title = if is_new {
        "Create New Agent".to_string()
    } else {
        format!("Edit Agent: {}", agent_name)
    };

    ui.label(RichText::new(&title).monospace().color(TEXT_PRIMARY));
    ui.add_space(16.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Name (only editable for new agents)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Name:").color(TEXT_MUTED));
                if is_new {
                    ui.add(
                        egui::TextEdit::singleline(state.agent_edit_name)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .desired_width(200.0),
                    );
                } else {
                    ui.label(
                        RichText::new(&*state.agent_edit_name)
                            .monospace()
                            .color(ACCENT_CYAN),
                    );
                }
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Aliases:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_aliases)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("c, cl")
                        .desired_width(200.0),
                );
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("SDK:").color(TEXT_MUTED));
                egui::ComboBox::from_id_salt("cli_type")
                    .selected_text(&*state.agent_edit_cli_type)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            state.agent_edit_cli_type,
                            "claude".to_string(),
                            "claude",
                        );
                        ui.selectable_value(
                            state.agent_edit_cli_type,
                            "codex".to_string(),
                            "codex",
                        );
                    });
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Session Mode:").color(TEXT_MUTED));
                egui::ComboBox::from_id_salt("agent_mode")
                    .selected_text(&*state.agent_edit_mode)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            state.agent_edit_mode,
                            "oneshot".to_string(),
                            "oneshot",
                        );
                        ui.selectable_value(
                            state.agent_edit_mode,
                            "session".to_string(),
                            "session",
                        );
                    });
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("System Prompt Mode:").color(TEXT_MUTED));
                egui::ComboBox::from_id_salt("system_prompt_mode")
                    .selected_text(&*state.agent_edit_system_prompt_mode)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            state.agent_edit_system_prompt_mode,
                            "append".to_string(),
                            "append",
                        );
                        ui.selectable_value(
                            state.agent_edit_system_prompt_mode,
                            "replace".to_string(),
                            "replace",
                        );
                        ui.selectable_value(
                            state.agent_edit_system_prompt_mode,
                            "configoverride".to_string(),
                            "configoverride",
                        );
                    });
            });
            ui.add_space(16.0);

            ui.separator();
            ui.add_space(8.0);
            ui.label(
                RichText::new("Tool Restrictions")
                    .monospace()
                    .color(TEXT_PRIMARY),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Allowed Tools:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_allowed_tools)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("Read, Grep (empty = all)")
                        .desired_width(300.0),
                );
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Disallowed Tools:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_disallowed_tools)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("Write, Edit")
                        .desired_width(300.0),
                );
            });
            ui.add_space(16.0);

            if let Some((msg, is_error)) = &state.agent_edit_status {
                let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
                ui.label(RichText::new(msg.as_str()).color(color));
                ui.add_space(8.0);
            }

            ui.horizontal(|ui| {
                if animated_button(ui, "Save to Config", ACCENT_GREEN, "agent_save_btn").clicked() {
                    save_agent_to_config(state, is_new);
                }
                if !is_new {
                    ui.add_space(16.0);
                    if animated_button(ui, "Delete", ACCENT_RED, "agent_delete_btn").clicked() {
                        delete_agent_from_config(state);
                    }
                }
            });
        });
}
