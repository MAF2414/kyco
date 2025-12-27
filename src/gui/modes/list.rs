//! Mode list rendering

use eframe::egui::{self, RichText, ScrollArea};

use super::persistence::load_mode_for_editing;
use super::state::ModeEditorState;
use crate::gui::app::{
    ACCENT_CYAN, ACCENT_GREEN, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};

/// Render the list of available modes
pub fn render_modes_list(ui: &mut egui::Ui, state: &mut ModeEditorState<'_>) {
    ui.label(
        RichText::new("Available Modes")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new("Modes define prompt templates for different task types. Click to edit.")
            .color(TEXT_DIM),
    );
    ui.add_space(12.0);

    let modes: Vec<(String, String)> = state
        .config
        .mode
        .iter()
        .map(|(name, mode)| {
            let aliases = mode.aliases.join(", ");
            (name.clone(), aliases)
        })
        .collect();

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (name, aliases) in &modes {
                egui::Frame::NONE
                    .fill(BG_SECONDARY)
                    .corner_radius(4.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        let response = ui.horizontal(|ui| {
                            ui.label(RichText::new(name).monospace().color(ACCENT_GREEN));
                            if !aliases.is_empty() {
                                ui.label(
                                    RichText::new(format!("({})", aliases))
                                        .small()
                                        .color(TEXT_MUTED),
                                );
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(RichText::new("→").color(TEXT_DIM));
                                },
                            );
                        });
                        if response.response.interact(egui::Sense::click()).clicked() {
                            *state.selected_mode = Some(name.clone());
                            load_mode_for_editing(state, name);
                        }
                    });
                ui.add_space(4.0);
            }

            ui.add_space(12.0);
            if ui
                .button(RichText::new("+ Add New Mode").color(ACCENT_CYAN))
                .clicked()
            {
                *state.selected_mode = Some("__new__".to_string());
                state.mode_edit_name.clear();
                state.mode_edit_aliases.clear();
                state.mode_edit_prompt.clear();
                state.mode_edit_system_prompt.clear();
                state.mode_edit_agent.clear();
                state.mode_edit_allowed_tools.clear();
                state.mode_edit_disallowed_tools.clear();
                *state.mode_edit_session_mode = "oneshot".to_string();
                *state.mode_edit_max_turns = "0".to_string();
                state.mode_edit_model.clear();
                *state.mode_edit_claude_permission = "auto".to_string();
                *state.mode_edit_codex_sandbox = "auto".to_string();
                *state.mode_edit_readonly = false;
                state.mode_edit_output_states.clear();
                state.mode_edit_state_prompt.clear();
                *state.mode_edit_status = None;
            }
        });
}
