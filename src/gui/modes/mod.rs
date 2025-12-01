//! Mode settings component for the GUI
//!
//! Renders the modes configuration view where users can:
//! - List all available modes
//! - Create new modes
//! - Edit existing modes (aliases, prompt template, system prompt, etc.)
//! - Delete modes

mod editor;
mod list;
mod persistence;
mod state;

pub use persistence::load_mode_for_editing;
pub use state::ModeEditorState;

use eframe::egui::{self, RichText};

use super::app::{ViewMode, BG_PRIMARY, TEXT_DIM, TEXT_PRIMARY};

/// Render the modes configuration view
pub fn render_modes(ctx: &egui::Context, state: &mut ModeEditorState<'_>) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(BG_PRIMARY).inner_margin(16.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("📋 MODES")
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
                        if state.selected_mode.is_some() {
                            ui.add_space(8.0);
                            if ui
                                .button(RichText::new("← Back").color(TEXT_DIM))
                                .clicked()
                            {
                                *state.selected_mode = None;
                                *state.mode_edit_status = None;
                            }
                        }
                    });
                });
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(16.0);

                if let Some(mode_name) = state.selected_mode.clone() {
                    editor::render_mode_editor(ui, state, &mode_name);
                } else {
                    list::render_modes_list(ui, state);
                }
            });
        });
}
