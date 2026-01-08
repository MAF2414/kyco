//! Chain list rendering

use eframe::egui::{self, RichText, ScrollArea};

use super::persistence::load_chain_for_editing;
use super::state::{ChainEditorState, StateDefinitionEdit};
use crate::gui::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, ACCENT_YELLOW, BG_SECONDARY, TEXT_DIM, TEXT_MUTED,
    TEXT_PRIMARY,
};

/// Color for state definitions
const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(200, 150, 255);

/// Render the list of available chains
pub fn render_chains_list(ui: &mut egui::Ui, state: &mut ChainEditorState<'_>) {
    ui.label(
        RichText::new("Available Chains")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "Chains execute multiple skills in sequence. Select code, then type the chain name.",
        )
        .color(TEXT_DIM),
    );
    ui.add_space(12.0);

    let mut chains: Vec<(String, String, usize, usize, Vec<String>)> = state
        .config
        .chain
        .iter()
        .map(|(name, chain)| {
            let desc = chain.description.clone().unwrap_or_default();
            let steps = chain.steps.len();
            let states = chain.states.len();
            let step_modes: Vec<String> = chain.steps.iter().map(|s| s.skill.clone()).collect();
            (name.clone(), desc, steps, states, step_modes)
        })
        .collect();
    chains.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    let mut chain_to_duplicate: Option<String> = None;

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (name, description, step_count, state_count, step_modes) in &chains {
                // Track if duplicate button was clicked (to prevent card click)
                let mut duplicate_clicked = false;

                let card_response = egui::Frame::NONE
                    .fill(BG_SECONDARY)
                    .corner_radius(4.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(name).monospace().color(ACCENT_YELLOW));

                            ui.label(
                                RichText::new(format!("{} steps", step_count))
                                    .small()
                                    .color(TEXT_MUTED),
                            );
                            if *state_count > 0 {
                                ui.label(
                                    RichText::new(format!("• {} states", state_count))
                                        .small()
                                        .color(ACCENT_PURPLE),
                                );
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .button(RichText::new("⧉").color(TEXT_DIM))
                                        .on_hover_text("Duplicate chain")
                                        .clicked()
                                    {
                                        duplicate_clicked = true;
                                        chain_to_duplicate = Some(name.clone());
                                    }
                                },
                            );
                        });

                        if !description.is_empty() {
                            ui.label(RichText::new(description).small().color(TEXT_DIM));
                        }

                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            for (i, skill) in step_modes.iter().enumerate() {
                                // Check if skill exists
                                let skill_exists = state.config.skill.contains_key(skill);
                                let color = if skill_exists { ACCENT_CYAN } else { ACCENT_RED };
                                let text = if skill_exists {
                                    RichText::new(skill).monospace().small().color(color)
                                } else {
                                    RichText::new(format!("{}⚠", skill))
                                        .monospace()
                                        .small()
                                        .color(color)
                                };
                                ui.label(text).on_hover_text(if skill_exists {
                                    String::new()
                                } else {
                                    format!("Skill '{}' not found - create it in Skills tab", skill)
                                });
                                if i < step_modes.len() - 1 {
                                    ui.label(RichText::new("→").small().color(TEXT_DIM));
                                }
                            }
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Use:").small().color(TEXT_DIM));
                            ui.label(RichText::new(name).monospace().small().color(ACCENT_GREEN));
                            ui.label(RichText::new("or").small().color(TEXT_DIM));
                            ui.label(
                                RichText::new(format!("claude:{}", name))
                                    .monospace()
                                    .small()
                                    .color(ACCENT_GREEN),
                            );
                        });
                    });

                // Make the whole card clickable (unless duplicate was clicked)
                let card_rect = card_response.response.rect;
                let card_interact =
                    ui.interact(card_rect, ui.id().with(name), egui::Sense::click());

                if card_interact.clicked() && !duplicate_clicked {
                    *state.selected_chain = Some(name.clone());
                    load_chain_for_editing(state, name);
                }

                if card_interact.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                ui.add_space(4.0);
            }

            ui.add_space(12.0);
            if ui
                .button(RichText::new("+ Add New Chain").color(ACCENT_CYAN))
                .clicked()
            {
                *state.selected_chain = Some("__new__".to_string());
                state.chain_edit_name.clear();
                state.chain_edit_description.clear();
                state.chain_edit_states.clear();
                state.chain_edit_steps.clear();
                *state.chain_edit_stop_on_failure = true;
                *state.chain_edit_pass_full_response = true;
                *state.chain_edit_max_loops = 1;
                *state.chain_edit_status = None;
            }
        });

    // Handle chain duplication outside the UI loop
    if let Some(source_name) = chain_to_duplicate {
        if let Some(source_chain) = state.config.chain.get(&source_name) {
            let mut new_name = format!("{}_copy", source_name);
            let mut counter = 1;
            while state.config.chain.contains_key(&new_name) {
                counter += 1;
                new_name = format!("{}_{}", source_name, counter);
            }

            *state.selected_chain = Some("__new__".to_string());
            *state.chain_edit_name = new_name;
            *state.chain_edit_description = source_chain.description.clone().unwrap_or_default();
            *state.chain_edit_states = source_chain
                .states
                .iter()
                .map(StateDefinitionEdit::from)
                .collect();
            *state.chain_edit_steps = source_chain
                .steps
                .iter()
                .map(super::state::ChainStepEdit::from)
                .collect();
            *state.chain_edit_stop_on_failure = source_chain.stop_on_failure;
            *state.chain_edit_pass_full_response = source_chain.pass_full_response;
            *state.chain_edit_max_loops = source_chain.max_loops;
            *state.chain_edit_status =
                Some(("Duplicated chain - edit name and save".to_string(), false));
        }
    }
}
