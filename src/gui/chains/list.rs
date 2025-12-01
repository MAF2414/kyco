//! Chain list rendering

use eframe::egui::{self, RichText, ScrollArea};

use super::persistence::load_chain_for_editing;
use super::state::ChainEditorState;
use crate::gui::app::{ACCENT_CYAN, ACCENT_YELLOW, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

/// Render the list of available chains
pub fn render_chains_list(ui: &mut egui::Ui, state: &mut ChainEditorState<'_>) {
    ui.label(RichText::new("Available Chains").monospace().color(TEXT_PRIMARY));
    ui.add_space(8.0);
    ui.label(
        RichText::new("Chains execute multiple modes in sequence. Click to edit.")
            .color(TEXT_DIM),
    );
    ui.add_space(12.0);

    // Get chains from config
    let chains: Vec<(String, String, usize)> = state
        .config
        .chain
        .iter()
        .map(|(name, chain)| {
            let desc = chain.description.clone().unwrap_or_default();
            let steps = chain.steps.len();
            (name.clone(), desc, steps)
        })
        .collect();

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (name, description, step_count) in &chains {
                egui::Frame::none()
                    .fill(BG_SECONDARY)
                    .corner_radius(4.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        let response = ui.horizontal(|ui| {
                            ui.label(RichText::new(name).monospace().color(ACCENT_YELLOW));
                            ui.label(
                                RichText::new(format!("({} steps)", step_count))
                                    .small()
                                    .color(TEXT_MUTED),
                            );
                            if !description.is_empty() {
                                ui.label(
                                    RichText::new(format!("- {}", description))
                                        .small()
                                        .color(TEXT_DIM),
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
                            *state.selected_chain = Some(name.clone());
                            load_chain_for_editing(state, name);
                        }
                    });
                ui.add_space(4.0);
            }

            // Add new chain button
            ui.add_space(12.0);
            if ui
                .button(RichText::new("+ Add New Chain").color(ACCENT_CYAN))
                .clicked()
            {
                *state.selected_chain = Some("__new__".to_string());
                state.chain_edit_name.clear();
                state.chain_edit_description.clear();
                state.chain_edit_steps.clear();
                *state.chain_edit_stop_on_failure = true;
                *state.chain_edit_status = None;
            }
        });
}
