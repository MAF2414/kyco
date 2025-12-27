//! Voice settings component for the GUI
//!
//! Renders the voice input settings section in the settings view where users can:
//! - Configure voice input mode (disabled, manual, hotkey hold, continuous)
//! - Set wake keywords for continuous mode
//! - Select Whisper model and language
//! - Configure advanced settings (silence threshold, duration, max duration)
//! - Install voice dependencies

use eframe::egui::{self, Color32, RichText};

use crate::gui::app::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};

use super::VoiceActionRegistry;

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
        render_voice_mode_selector(ui, state);
        ui.add_space(8.0);

        render_mode_description(ui, state.voice_settings_mode);
        ui.add_space(12.0);

        render_whisper_model_selector(ui, state.voice_settings_model);
        ui.add_space(8.0);

        render_language_selector(ui, state.voice_settings_language);
        ui.add_space(12.0);

        render_advanced_settings(ui, state);
    });

    ui.add_space(12.0);
    render_voice_actions_section(ui, state.action_registry);

    ui.add_space(12.0);
    render_vad_settings(ui, state);

    render_save_button(ui, state);
    render_status_message(ui, state.settings_status);
    render_dependencies_section(ui, state);

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

fn render_voice_mode_selector(ui: &mut egui::Ui, state: &mut VoiceSettingsState<'_>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Mode:").color(TEXT_MUTED));
        egui::ComboBox::from_id_salt("voice_mode")
            .selected_text(state.voice_settings_mode.as_str())
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
}

fn render_mode_description(ui: &mut egui::Ui, mode: &str) {
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

fn render_whisper_model_selector(ui: &mut egui::Ui, model: &mut String) {
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

fn render_language_selector(ui: &mut egui::Ui, language: &mut String) {
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

fn render_advanced_settings(ui: &mut egui::Ui, state: &mut VoiceSettingsState<'_>) {
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
    });
}

fn render_voice_actions_section(ui: &mut egui::Ui, registry: &VoiceActionRegistry) {
    ui.label(
        RichText::new("Voice Actions (Wakewords → Modes)")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);

    egui::Frame::NONE
        .fill(Color32::from_rgb(30, 30, 35))
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new("Speak a wakeword to trigger the corresponding mode:")
                    .color(TEXT_MUTED),
            );

            if let Some(ref prefix) = registry.global_prefix {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("Global prefix: \"{}\"", prefix))
                        .small()
                        .color(ACCENT_CYAN),
                );
            }

            ui.add_space(8.0);

            // Display actions from registry (dynamically loaded from modes/chains)
            if registry.actions.is_empty() {
                ui.label(
                    RichText::new("No modes or chains configured. Add modes in your config to enable voice actions.")
                        .small()
                        .color(TEXT_DIM),
                );
            } else {
                egui::Grid::new("voice_actions_grid")
                    .num_columns(3)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Wakeword").color(TEXT_MUTED).small());
                        ui.label(RichText::new("Mode").color(TEXT_MUTED).small());
                        ui.label(RichText::new("Aliases").color(TEXT_MUTED).small());
                        ui.end_row();

                        for action in &registry.actions {
                            let primary = action.wakewords.first().map(|s| s.as_str()).unwrap_or(&action.mode);
                            ui.label(RichText::new(primary).monospace().color(ACCENT_CYAN));
                            ui.label(RichText::new(format!("→ {}", action.mode)).color(TEXT_PRIMARY));

                            // Aliases (skip first wakeword, add action aliases)
                            let aliases: Vec<&str> = action.wakewords.iter().skip(1).map(|s| s.as_str())
                                .chain(action.aliases.iter().map(|s| s.as_str()))
                                .collect();
                            let aliases_str = if aliases.is_empty() {
                                "-".to_string()
                            } else {
                                aliases.join(", ")
                            };
                            ui.label(RichText::new(aliases_str).small().color(TEXT_DIM));
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);
                let example_mode = registry.actions.first().map(|a| a.mode.as_str()).unwrap_or("refactor");
                ui.label(
                    RichText::new(format!("Example: Say \"{} this function\" to trigger {} mode", example_mode, example_mode))
                        .small()
                        .italics()
                        .color(TEXT_DIM),
                );
            }
        });
}

/// Render VAD (Voice Activity Detection) settings
fn render_vad_settings(ui: &mut egui::Ui, state: &mut VoiceSettingsState<'_>) {
    ui.label(
        RichText::new("VAD Settings (Voice Activity Detection)")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);

    egui::Frame::NONE
        .fill(Color32::from_rgb(30, 30, 35))
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new(
                    "VAD detects when you start/stop speaking for efficient continuous listening.",
                )
                .small()
                .color(TEXT_MUTED),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.checkbox(state.vad_enabled, "");
                ui.label(RichText::new("Enable VAD for continuous mode").color(TEXT_PRIMARY));
            });

            if *state.vad_enabled {
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Speech threshold:").color(TEXT_MUTED));
                    ui.add(
                        egui::TextEdit::singleline(state.vad_speech_threshold)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .desired_width(60.0),
                    );
                    ui.label(RichText::new("(0.0-1.0)").small().color(TEXT_DIM));
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Silence to stop:").color(TEXT_MUTED));
                    ui.add(
                        egui::TextEdit::singleline(state.vad_silence_duration_ms)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .desired_width(60.0),
                    );
                    ui.label(RichText::new("ms").small().color(TEXT_DIM));
                });
            }
        });
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

fn render_dependencies_section(ui: &mut egui::Ui, state: &mut VoiceSettingsState<'_>) {
    ui.add_space(12.0);
    ui.label(
        RichText::new("Voice Dependencies")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);

    egui::Frame::NONE
        .fill(Color32::from_rgb(30, 30, 35))
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new("Voice input requires the following tools to be installed:")
                    .color(TEXT_MUTED),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("-").color(TEXT_DIM));
                ui.label(RichText::new("sox").monospace().color(ACCENT_CYAN));
                ui.label(RichText::new("- audio recording").color(TEXT_DIM));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("-").color(TEXT_DIM));
                ui.label(RichText::new("whisper-cpp").monospace().color(ACCENT_CYAN));
                ui.label(
                    RichText::new("- speech-to-text transcription").color(TEXT_DIM),
                );
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                let button_text = if *state.voice_install_in_progress {
                    "Installing..."
                } else {
                    "Install Voice Dependencies"
                };

                let text_color = if *state.voice_install_in_progress {
                    TEXT_MUTED
                } else {
                    TEXT_PRIMARY
                };

                let button = ui.add_enabled(
                    !*state.voice_install_in_progress,
                    egui::Button::new(RichText::new(button_text).color(text_color)),
                );

                if button.clicked() && !*state.voice_install_in_progress {
                    (state.on_install_dependencies)();
                }

                ui.add_space(8.0);
                ui.label(
                    RichText::new("(requires Homebrew on macOS)")
                        .small()
                        .color(TEXT_DIM),
                );
            });

            if let Some((msg, is_error)) = state.voice_install_status.as_ref() {
                ui.add_space(8.0);
                let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
                ui.label(RichText::new(msg.as_str()).small().color(color));
            }

            // Microphone permission button (macOS only)
            #[cfg(target_os = "macos")]
            {
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Microphone access must be granted in System Settings.")
                        .small()
                        .color(TEXT_MUTED),
                );
                ui.add_space(4.0);
                if ui
                    .button(RichText::new("Open Microphone Settings").color(ACCENT_CYAN))
                    .clicked()
                {
                    let _ = std::process::Command::new("open")
                        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
                        .spawn();
                }
            }
        });
}

fn render_status_message(ui: &mut egui::Ui, status: &Option<(String, bool)>) {
    if let Some((msg, is_error)) = status {
        let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
        ui.label(RichText::new(msg).color(color));
    }
}

/// Render a labeled text input field with a description on the same line
fn render_text_field_with_desc(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    width: f32,
    description: &str,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT_MUTED));
        ui.add(
            egui::TextEdit::singleline(value)
                .font(egui::TextStyle::Monospace)
                .text_color(TEXT_PRIMARY)
                .desired_width(width),
        );
        ui.label(RichText::new(description).small().color(TEXT_MUTED));
    });
}
