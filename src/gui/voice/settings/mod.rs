//! Voice settings component for the GUI
//!
//! Renders the voice input settings section in the settings view where users can:
//! - Configure voice input mode (disabled, manual, hotkey hold, continuous)
//! - Set wake keywords for continuous mode
//! - Select Whisper model and language
//! - Configure advanced settings (silence threshold, duration, max duration)
//! - Install voice dependencies

mod advanced;
mod dependencies;
mod selectors;
mod vad_settings;
mod voice_actions;

use eframe::egui::{self, RichText};

use crate::gui::theme::{ACCENT_GREEN, ACCENT_RED, BG_SECONDARY, TEXT_DIM, TEXT_PRIMARY};

use super::VoiceActionRegistry;

use advanced::render_advanced_settings;
use dependencies::render_dependencies_section;
use selectors::{
    render_language_selector, render_mode_description, render_voice_mode_selector,
    render_whisper_model_selector,
};
use vad_settings::render_vad_settings;
use voice_actions::render_voice_actions_section;

/// State for voice settings UI
pub struct VoiceSettingsState<'a> {
    pub voice_settings_mode: &'a mut String,
    pub voice_settings_keywords: &'a mut String,
    pub voice_settings_model: &'a mut String,
    pub voice_settings_language: &'a mut String,
    pub voice_settings_silence_threshold: &'a mut String,
    pub voice_settings_silence_duration: &'a mut String,
    pub voice_settings_max_duration: &'a mut String,
    pub voice_install_status: &'a mut Option<(String, bool)>,
    pub voice_install_in_progress: &'a mut bool,
    pub settings_status: &'a Option<(String, bool)>,
    /// Voice action registry (for displaying available wakewords)
    pub action_registry: &'a VoiceActionRegistry,
    /// VAD settings
    pub vad_enabled: &'a mut bool,
    pub vad_speech_threshold: &'a mut String,
    pub vad_silence_duration_ms: &'a mut String,
    /// Callback to save settings
    pub on_save: &'a mut dyn FnMut(),
    /// Callback to install voice dependencies
    pub on_install_dependencies: &'a mut dyn FnMut(),
}

/// Render the voice input settings section
pub fn render_voice_settings(ui: &mut egui::Ui, state: &mut VoiceSettingsState<'_>) {
    ui.label(RichText::new("Voice Input").monospace().color(TEXT_PRIMARY));
    ui.add_space(8.0);
    ui.label(RichText::new("Configure voice input for hands-free operation.").color(TEXT_DIM));
    ui.add_space(12.0);

    render_section_frame(ui, |ui| {
        render_voice_mode_selector(ui, state.voice_settings_mode);
        ui.add_space(8.0);

        render_mode_description(ui, state.voice_settings_mode);
        ui.add_space(12.0);

        render_whisper_model_selector(ui, state.voice_settings_model);
        ui.add_space(8.0);

        render_language_selector(ui, state.voice_settings_language);
        ui.add_space(12.0);

        render_advanced_settings(
            ui,
            state.voice_settings_silence_threshold,
            state.voice_settings_silence_duration,
            state.voice_settings_max_duration,
        );
    });

    ui.add_space(12.0);
    render_voice_actions_section(ui, state.action_registry);

    ui.add_space(12.0);
    render_vad_settings(
        ui,
        state.vad_enabled,
        state.vad_speech_threshold,
        state.vad_silence_duration_ms,
    );

    render_save_button(ui, state);
    render_status_message(ui, state.settings_status);
    render_dependencies_section(
        ui,
        state.voice_install_status,
        state.voice_install_in_progress,
        state.on_install_dependencies,
    );

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
}

fn render_section_frame<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, add_contents)
        .inner
}

fn render_save_button(ui: &mut egui::Ui, state: &mut VoiceSettingsState<'_>) {
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new("Save Voice Settings").color(ACCENT_GREEN))
            .clicked()
        {
            (state.on_save)();
        }
    });
}

fn render_status_message(ui: &mut egui::Ui, status: &Option<(String, bool)>) {
    if let Some((msg, is_error)) = status {
        let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
        ui.label(RichText::new(msg).color(color));
    }
}
