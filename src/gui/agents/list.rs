//! Agent list rendering

use eframe::egui::{self, RichText, ScrollArea};

use super::persistence::load_agent_for_editing;
use super::state::AgentEditorState;
use crate::gui::theme::{ACCENT_CYAN, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

/// Render the list of available agents
pub fn render_agents_list(ui: &mut egui::Ui, state: &mut AgentEditorState<'_>) {
    ui.label(
        RichText::new("Available Agents")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new("Agents select which SDK backend to use. Click to edit.").color(TEXT_DIM),
    );
    ui.add_space(12.0);

    let agents: Vec<(String, String, String)> = state
        .config
        .agent
        .iter()
        .map(|(name, agent)| {
            let aliases = agent.aliases.join(", ");
            let backend = agent.sdk.default_name().to_string();
            (name.clone(), aliases, backend)
        })
        .collect();

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (name, aliases, binary) in &agents {
                egui::Frame::NONE
                    .fill(BG_SECONDARY)
                    .corner_radius(4.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        let response = ui.horizontal(|ui| {
                            ui.label(RichText::new(name).monospace().color(ACCENT_CYAN));
                            if !aliases.is_empty() {
                                ui.label(
                                    RichText::new(format!("({})", aliases))
                                        .small()
                                        .color(TEXT_MUTED),
                                );
                            }
                            ui.label(
                                RichText::new(format!("-> {}", binary))
                                    .small()
                                    .color(TEXT_DIM),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(RichText::new("->").color(TEXT_DIM));
                                },
                            );
                        });
                        if response.response.interact(egui::Sense::click()).clicked() {
                            *state.selected_agent = Some(name.clone());
                            load_agent_for_editing(state, name);
                        }
                    });
                ui.add_space(4.0);
            }

            ui.add_space(12.0);
            if ui
                .button(RichText::new("+ Add New Agent").color(ACCENT_CYAN))
                .clicked()
            {
                *state.selected_agent = Some("__new__".to_string());
                state.agent_edit_name.clear();
                state.agent_edit_aliases.clear();
                *state.agent_edit_cli_type = "claude".to_string();
                state.agent_edit_model.clear();
                state.agent_edit_permission_mode.clear();
                state.agent_edit_sandbox.clear();
                state.agent_edit_ask_for_approval.clear();
                *state.agent_edit_mode = "oneshot".to_string();
                *state.agent_edit_system_prompt_mode = "append".to_string();
                state.agent_edit_allowed_tools.clear();
                state.agent_edit_disallowed_tools.clear();
                *state.agent_edit_allow_dangerous_bypass = false;
                *state.agent_edit_status = None;
            }
        });
}
