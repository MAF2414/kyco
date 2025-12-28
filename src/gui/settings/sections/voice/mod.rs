//! Voice input settings section

mod actions;
mod dependencies;
mod mode;
mod testing;

use eframe::egui::{self, RichText};

use crate::gui::settings::helpers::{render_section_frame, render_status_message};
use crate::gui::settings::save::save_settings_to_config;
use crate::gui::settings::state::SettingsState;
use crate::gui::theme::{ACCENT_GREEN, TEXT_DIM, TEXT_PRIMARY};

use actions::{render_vad_settings, render_voice_actions};
use dependencies::render_voice_dependencies_section;
use mode::{render_advanced_settings, render_voice_mode_settings, render_voice_model_settings};

/// Render Voice Input Settings section
pub fn render_settings_voice(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(RichText::new("Voice Input").monospace().color(TEXT_PRIMARY));
    ui.add_space(8.0);
    ui.label(RichText::new("Configure voice input for hands-free operation.").color(TEXT_DIM));
    ui.add_space(12.0);

    render_section_frame(ui, |ui| {
        render_voice_mode_settings(ui, state);
        render_voice_model_settings(ui, state);
        render_advanced_settings(ui, state);
    });

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new("Save Voice Settings").color(ACCENT_GREEN))
            .clicked()
        {
            save_settings_to_config(state);
        }
    });

    render_status_message(ui, state.settings_status);

    // Voice dependency installation section
    ui.add_space(12.0);
    render_voice_dependencies_section(ui, state);

    ui.add_space(12.0);
    render_voice_actions(ui, state);

    ui.add_space(12.0);
    render_vad_settings(ui, state);

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
}
