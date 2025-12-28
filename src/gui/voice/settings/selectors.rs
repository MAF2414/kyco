//! Voice mode, model, and language selectors

use eframe::egui::{self, RichText};

use crate::gui::theme::TEXT_MUTED;

pub fn render_voice_mode_selector(ui: &mut egui::Ui, voice_settings_mode: &mut String) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Mode:").color(TEXT_MUTED));
        egui::ComboBox::from_id_salt("voice_mode")
            .selected_text(voice_settings_mode.as_str())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    voice_settings_mode,
                    "disabled".to_string(),
                    "Disabled",
                );
                ui.selectable_value(
                    voice_settings_mode,
                    "manual".to_string(),
                    "Manual (click mic or Shift+V)",
                );
                ui.selectable_value(
                    voice_settings_mode,
                    "hotkey_hold".to_string(),
                    "Hold Shift+V to record",
                );
                ui.selectable_value(
                    voice_settings_mode,
                    "continuous".to_string(),
                    "Always listening for keywords",
                );
            });
    });
}

pub fn render_mode_description(ui: &mut egui::Ui, mode: &str) {
    let mode_desc = match mode {
        "manual" => "Click the microphone button or press Shift+V to record.",
        "hotkey_hold" => "Hold Shift+V while speaking, release to transcribe.",
        "continuous" => {
            "Listens for mode keywords (e.g., 'refactor', 'fix') and triggers automatically."
        }
        _ => "Voice input is disabled.",
    };
    ui.label(RichText::new(mode_desc).small().color(TEXT_MUTED));
}

pub fn render_whisper_model_selector(ui: &mut egui::Ui, model: &mut String) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Whisper Model:").color(TEXT_MUTED));
        egui::ComboBox::from_id_salt("whisper_model")
            .selected_text(model.as_str())
            .show_ui(ui, |ui| {
                ui.selectable_value(model, "tiny".to_string(), "tiny (fast, less accurate)");
                ui.selectable_value(model, "base".to_string(), "base (balanced)");
                ui.selectable_value(model, "small".to_string(), "small (better accuracy)");
                ui.selectable_value(model, "medium".to_string(), "medium (high accuracy)");
                ui.selectable_value(model, "large".to_string(), "large (best accuracy)");
            });
    });
}

pub fn render_language_selector(ui: &mut egui::Ui, language: &mut String) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Language:").color(TEXT_MUTED));
        egui::ComboBox::from_id_salt("voice_language")
            .selected_text(language.as_str())
            .show_ui(ui, |ui| {
                ui.selectable_value(language, "auto".to_string(), "Auto-detect");
                ui.selectable_value(language, "en".to_string(), "English");
                ui.selectable_value(language, "de".to_string(), "German");
                ui.selectable_value(language, "fr".to_string(), "French");
                ui.selectable_value(language, "es".to_string(), "Spanish");
                ui.selectable_value(language, "it".to_string(), "Italian");
                ui.selectable_value(language, "pt".to_string(), "Portuguese");
                ui.selectable_value(language, "nl".to_string(), "Dutch");
                ui.selectable_value(language, "pl".to_string(), "Polish");
                ui.selectable_value(language, "ja".to_string(), "Japanese");
                ui.selectable_value(language, "zh".to_string(), "Chinese");
            });
    });
}
