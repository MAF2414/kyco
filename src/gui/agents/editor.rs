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

            // Model override (optional - empty uses user's default)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Model:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_model)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("(uses default)")
                        .desired_width(200.0),
                );
            });
            ui.add_space(8.0);

            // SDK-specific permission / sandbox settings
            match state.agent_edit_cli_type.as_str() {
                "claude" => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Permission Mode:").color(TEXT_MUTED));
                        let selected_text = if state.agent_edit_permission_mode.trim().is_empty()
                        {
                            "Auto (skill-derived)".to_string()
                        } else {
                            state.agent_edit_permission_mode.to_string()
                        };
                        egui::ComboBox::from_id_salt("agent_permission_mode")
                            .selected_text(selected_text)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    state.agent_edit_permission_mode,
                                    "".to_string(),
                                    "Auto (skill-derived)",
                                );
                                ui.selectable_value(
                                    state.agent_edit_permission_mode,
                                    "default".to_string(),
                                    "default (ask)",
                                );
                                ui.selectable_value(
                                    state.agent_edit_permission_mode,
                                    "acceptEdits".to_string(),
                                    "acceptEdits",
                                );
                                ui.selectable_value(
                                    state.agent_edit_permission_mode,
                                    "plan".to_string(),
                                    "plan",
                                );
                                ui.selectable_value(
                                    state.agent_edit_permission_mode,
                                    "bypassPermissions".to_string(),
                                    "bypassPermissions",
                                );
                            });
                    });
                    ui.add_space(8.0);
                }
                "codex" => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Sandbox:").color(TEXT_MUTED));
                        let selected_text = if state.agent_edit_sandbox.trim().is_empty() {
                            "Auto (skill-derived)".to_string()
                        } else {
                            state.agent_edit_sandbox.to_string()
                        };
                        egui::ComboBox::from_id_salt("agent_codex_sandbox")
                            .selected_text(selected_text)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    state.agent_edit_sandbox,
                                    "".to_string(),
                                    "Auto (skill-derived)",
                                );
                                ui.selectable_value(
                                    state.agent_edit_sandbox,
                                    "read-only".to_string(),
                                    "read-only",
                                );
                                ui.selectable_value(
                                    state.agent_edit_sandbox,
                                    "workspace-write".to_string(),
                                    "workspace-write",
                                );
                                ui.selectable_value(
                                    state.agent_edit_sandbox,
                                    "danger-full-access".to_string(),
                                    "danger-full-access",
                                );
                            });
                    });
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Ask For Approval:").color(TEXT_MUTED));
                        let selected_text = if state.agent_edit_ask_for_approval.trim().is_empty()
                        {
                            "Auto (never)".to_string()
                        } else {
                            state.agent_edit_ask_for_approval.to_string()
                        };
                        egui::ComboBox::from_id_salt("agent_codex_ask_for_approval")
                            .selected_text(selected_text)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    state.agent_edit_ask_for_approval,
                                    "".to_string(),
                                    "Auto (never)",
                                );
                                ui.selectable_value(
                                    state.agent_edit_ask_for_approval,
                                    "untrusted".to_string(),
                                    "untrusted (ask always)",
                                );
                                ui.selectable_value(
                                    state.agent_edit_ask_for_approval,
                                    "on-request".to_string(),
                                    "on-request",
                                );
                                ui.selectable_value(
                                    state.agent_edit_ask_for_approval,
                                    "on-failure".to_string(),
                                    "on-failure",
                                );
                                ui.selectable_value(
                                    state.agent_edit_ask_for_approval,
                                    "never".to_string(),
                                    "never (no prompts)",
                                );
                            });
                    });
                    ui.add_space(8.0);
                }
                _ => {}
            }

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

            ui.separator();
            ui.add_space(8.0);
            ui.label(
                RichText::new("Token Pricing (per 1M tokens, USD)")
                    .monospace()
                    .color(TEXT_PRIMARY),
            );
            ui.label(
                RichText::new("Used for cost estimation when API doesn't return cost")
                    .small()
                    .color(TEXT_MUTED),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Input:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_price_input)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("e.g., 3.00")
                        .desired_width(80.0),
                );
                ui.label(RichText::new("Cached:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_price_cached_input)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("e.g., 0.30")
                        .desired_width(80.0),
                );
                ui.label(RichText::new("Output:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_price_output)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("e.g., 15.00")
                        .desired_width(80.0),
                );
            });
            ui.add_space(16.0);

            ui.separator();
            ui.add_space(8.0);
            ui.label(
                RichText::new("⚠ Safety Settings")
                    .monospace()
                    .color(ACCENT_RED),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.checkbox(
                    state.agent_edit_allow_dangerous_bypass,
                    RichText::new("Allow dangerous bypass").color(TEXT_PRIMARY),
                );
            });
            ui.label(
                RichText::new("Enables --dangerously-skip-permissions (Claude) or --yolo (Codex)")
                    .small()
                    .color(TEXT_MUTED),
            );
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
