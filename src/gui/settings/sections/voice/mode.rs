//! Voice mode and model settings

use eframe::egui::{self, RichText};

use crate::gui::settings::helpers::{render_text_field, render_text_field_with_desc};
use crate::gui::settings::state::SettingsState;
use crate::gui::theme::{TEXT_MUTED, TEXT_PRIMARY};

/// Render voice mode dropdown and description
pub fn render_voice_mode_settings(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Mode:").color(TEXT_MUTED));
        egui::ComboBox::from_id_salt("voice_mode")
            .selected_text(&*state.voice_settings_mode)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    state.voice_settings_mode,
                    "disabled".to_string(),
                    "Disabled",
                );
                ui.selectable_value(
                    state.voice_settings_mode,
                    "manual".to_string(),
                    "Manual (click mic or Shift+V)",
                );
                ui.selectable_value(
                    state.voice_settings_mode,
                    "hotkey_hold".to_string(),
                    "Hold Shift+V to record",
                );
                ui.selectable_value(
                    state.voice_settings_mode,
                    "continuous".to_string(),
                    "Always listening for keywords",
                );
            });
    });
    ui.add_space(8.0);

    // Mode descriptions
    let mode_desc = match state.voice_settings_mode.as_str() {
        "manual" => "Click the microphone button or press Shift+V to record.",
        "hotkey_hold" => "Hold Shift+V while speaking, release to transcribe.",
        "continuous" => {
            "Listens for mode keywords (e.g., 'refactor', 'fix') and triggers automatically."
        }
        _ => "Voice input is disabled.",
    };
    ui.label(RichText::new(mode_desc).small().color(TEXT_MUTED));
    ui.add_space(12.0);

    // Keywords (for continuous mode)
    render_text_field(
        ui,
        "Keywords:",
        state.voice_settings_keywords,
        300.0,
        Some("refactor, fix, tests, docs"),
    );
    ui.label(
        RichText::new("(comma-separated, used for continuous mode)")
            .small()
            .color(TEXT_MUTED),
    );
    ui.add_space(8.0);
}

/// Render whisper model and language dropdowns
pub fn render_voice_model_settings(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Whisper Model:").color(TEXT_MUTED));
        egui::ComboBox::from_id_salt("whisper_model")
            .selected_text(&*state.voice_settings_model)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    state.voice_settings_model,
                    "tiny".to_string(),
                    "tiny (fast, less accurate)",
                );
                ui.selectable_value(
                    state.voice_settings_model,
                    "base".to_string(),
                    "base (balanced)",
                );
                ui.selectable_value(
                    state.voice_settings_model,
                    "small".to_string(),
                    "small (better accuracy)",
                );
                ui.selectable_value(
                    state.voice_settings_model,
                    "medium".to_string(),
                    "medium (high accuracy)",
                );
                ui.selectable_value(
                    state.voice_settings_model,
                    "large".to_string(),
                    "large (best accuracy)",
                );
            });
    });
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label(RichText::new("Language:").color(TEXT_MUTED));
        egui::ComboBox::from_id_salt("voice_language")
            .selected_text(&*state.voice_settings_language)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    state.voice_settings_language,
                    "auto".to_string(),
                    "Auto-detect",
                );
                ui.selectable_value(state.voice_settings_language, "en".to_string(), "English");
                ui.selectable_value(state.voice_settings_language, "de".to_string(), "German");
                ui.selectable_value(state.voice_settings_language, "fr".to_string(), "French");
                ui.selectable_value(state.voice_settings_language, "es".to_string(), "Spanish");
                ui.selectable_value(state.voice_settings_language, "it".to_string(), "Italian");
                ui.selectable_value(
                    state.voice_settings_language,
                    "pt".to_string(),
                    "Portuguese",
                );
                ui.selectable_value(state.voice_settings_language, "nl".to_string(), "Dutch");
                ui.selectable_value(state.voice_settings_language, "pl".to_string(), "Polish");
                ui.selectable_value(
                    state.voice_settings_language,
                    "ja".to_string(),
                    "Japanese",
                );
                ui.selectable_value(state.voice_settings_language, "zh".to_string(), "Chinese");
            });
    });
    ui.add_space(12.0);
}

/// Render advanced voice settings (collapsible)
pub fn render_advanced_settings(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.collapsing("Advanced Settings", |ui| {
        ui.add_space(4.0);
        render_text_field_with_desc(
            ui,
            "Silence Threshold:",
            state.voice_settings_silence_threshold,
            60.0,
            "(0.0-1.0)",
        );
        ui.add_space(4.0);
        render_text_field_with_desc(
            ui,
            "Silence Duration:",
            state.voice_settings_silence_duration,
            60.0,
            "seconds",
        );
        ui.add_space(4.0);
        render_text_field_with_desc(
            ui,
            "Max Duration:",
            state.voice_settings_max_duration,
            60.0,
            "seconds",
        );
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label(RichText::new("Hotkeys").color(TEXT_PRIMARY).strong());
        ui.add_space(4.0);
        render_text_field_with_desc(
            ui,
            "Global Dictation:",
            state.voice_settings_global_hotkey,
            120.0,
            "e.g. cmd+shift+v",
        );
        ui.add_space(4.0);
        render_text_field_with_desc(
            ui,
            "Popup Recording:",
            state.voice_settings_popup_hotkey,
            120.0,
            "e.g. cmd+d",
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("Note: Hotkey changes require app restart")
                .color(TEXT_MUTED)
                .small(),
        );
    });
}
