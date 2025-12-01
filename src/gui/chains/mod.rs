//! Chain settings component for the GUI
//!
//! Renders the chains configuration view where users can:
//! - List all available chains
//! - Create new chains
//! - Edit existing chains (steps, triggers, etc.)
//! - Delete chains

mod editor;
mod list;
mod persistence;
pub mod state;

pub use state::{ChainEditorState, ChainStepEdit};

use eframe::egui::{self, RichText};

use super::app::{ViewMode, BG_PRIMARY, TEXT_DIM, TEXT_PRIMARY};

/// Render the chains configuration view
pub fn render_chains(ctx: &egui::Context, state: &mut ChainEditorState<'_>) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(BG_PRIMARY).inner_margin(16.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("🔗 CHAINS")
                            .monospace()
                            .size(18.0)
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("✕ Close").color(TEXT_DIM))
                            .clicked()
                        {
                            *state.view_mode = ViewMode::JobList;
                        }
                        if state.selected_chain.is_some() {
                            ui.add_space(8.0);
                            if ui
                                .button(RichText::new("← Back").color(TEXT_DIM))
                                .clicked()
                            {
                                *state.selected_chain = None;
                                *state.chain_edit_status = None;
                            }
                        }
                    });
                });
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(16.0);

                if let Some(chain_name) = state.selected_chain.clone() {
                    editor::render_chain_editor(ui, state, &chain_name);
                } else {
                    list::render_chains_list(ui, state);
                }
            });
        });
}
