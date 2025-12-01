//! Main settings panel rendering
//!
//! Contains the top-level render function for the settings view.

use eframe::egui::{self, RichText, ScrollArea};

use crate::gui::app::{ViewMode, BG_PRIMARY, TEXT_DIM, TEXT_PRIMARY};

use super::sections::{
    render_settings_general, render_settings_http_server, render_settings_ide_extensions,
    render_settings_output_schema, render_settings_voice,
};
use super::state::SettingsState;

/// Render the settings configuration view
pub fn render_settings(ctx: &egui::Context, state: &mut SettingsState<'_>) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(BG_PRIMARY).inner_margin(16.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("⚙ SETTINGS")
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
                    });
                });
                ui.add_space(16.0);

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        render_settings_general(ui, state);
                        render_settings_output_schema(ui, state);
                        render_settings_ide_extensions(ui, state);
                        render_settings_voice(ui, state);
                        render_settings_http_server(ui);
                    });
            });
        });
}
