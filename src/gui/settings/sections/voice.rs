//! Voice input settings section

use eframe::egui::{self, Color32, RichText};

use crate::gui::theme::{ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

use super::super::helpers::{
    render_section_frame, render_status_message, render_text_field, render_text_field_with_desc,
};
use super::super::save::save_settings_to_config;
use super::super::state::{SettingsState, VoiceTestStatus};

/// Render Voice Input Settings section
pub fn render_settings_voice(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(RichText::new("Voice Input").monospace().color(TEXT_PRIMARY));
    ui.add_space(8.0);
    ui.label(RichText::new("Configure voice input for hands-free operation.").color(TEXT_DIM));
    ui.add_space(12.0);

    render_section_frame(ui, |ui| {
        render_voice_mode_settings(ui, state);
        render_voice_model_settings(ui, state);

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

/// Render voice mode dropdown and description
fn render_voice_mode_settings(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
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
fn render_voice_model_settings(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
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

/// Render voice dependencies installation section
fn render_voice_dependencies_section(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
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
                ui.label(RichText::new("•").color(TEXT_DIM));
                ui.label(RichText::new("sox").monospace().color(ACCENT_CYAN));
                ui.label(RichText::new("- audio recording").color(TEXT_DIM));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("•").color(TEXT_DIM));
                ui.label(
                    RichText::new("whisper-cpp")
                        .monospace()
                        .color(ACCENT_CYAN),
                );
                ui.label(RichText::new("- speech-to-text transcription").color(TEXT_DIM));
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                let button_text = if *state.voice_install_in_progress {
                    "Installing..."
                } else {
                    "Install Voice Dependencies"
                };

                let button = ui.add_enabled(
                    !*state.voice_install_in_progress,
                    egui::Button::new(RichText::new(button_text).color(
                        if *state.voice_install_in_progress {
                            TEXT_MUTED
                        } else {
                            TEXT_PRIMARY
                        },
                    )),
                );

                if button.clicked() && !*state.voice_install_in_progress {
                    install_voice_dependencies(state);
                }

                ui.add_space(8.0);
                ui.label(
                    RichText::new("(requires Homebrew on macOS)")
                        .small()
                        .color(TEXT_DIM),
                );
            });

            if let Some((msg, is_error)) = &state.voice_install_status {
                ui.add_space(8.0);
                let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
                ui.label(RichText::new(msg).small().color(color));
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

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            render_voice_test_section(ui, state);
        });
}

/// Render Voice Actions (Wakewords → Modes) section
fn render_voice_actions(ui: &mut egui::Ui, state: &SettingsState<'_>) {
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
                RichText::new(
                    "Speak a wakeword to trigger the corresponding mode (loaded from config):",
                )
                .color(TEXT_MUTED),
            );

            // Show global prefix if set
            if let Some(ref prefix) = state.voice_action_registry.global_prefix {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("Global prefix: \"{}\"", prefix))
                        .small()
                        .color(ACCENT_CYAN),
                );
            }

            ui.add_space(8.0);

            // Display actions from registry
            if state.voice_action_registry.actions.is_empty() {
                ui.label(
                    RichText::new("No modes configured. Add modes in .kyco/config.toml")
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

                        for action in &state.voice_action_registry.actions {
                            let primary = action
                                .wakewords
                                .first()
                                .map(|s| s.as_str())
                                .unwrap_or(&action.mode);
                            ui.label(RichText::new(primary).monospace().color(ACCENT_CYAN));
                            ui.label(
                                RichText::new(format!("→ {}", action.mode)).color(TEXT_PRIMARY),
                            );

                            let aliases: Vec<&str> = action
                                .wakewords
                                .iter()
                                .skip(1)
                                .map(|s| s.as_str())
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
                if let Some(first) = state.voice_action_registry.actions.first() {
                    ui.label(
                        RichText::new(format!("Example: \"{}\" this function", first.mode))
                            .small()
                            .italics()
                            .color(TEXT_DIM),
                    );
                }
            }
        });
}

/// Render VAD (Voice Activity Detection) settings - Coming Soon
fn render_vad_settings(ui: &mut egui::Ui, _state: &mut SettingsState<'_>) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("VAD (Voice Activity Detection)")
                .monospace()
                .color(TEXT_MUTED),
        );
        ui.label(
            RichText::new("Coming Soon")
                .small()
                .color(Color32::from_rgb(255, 180, 0)),
        );
    });
    ui.add_space(8.0);

    egui::Frame::NONE
        .fill(Color32::from_rgb(30, 30, 35))
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new("VAD will detect speech to efficiently trigger transcription in continuous mode. This feature is currently under development.")
                    .small()
                    .color(TEXT_DIM),
            );
        });
}

/// Install voice dependencies (sox, whisper-cpp) - async, non-blocking
fn install_voice_dependencies(state: &mut SettingsState<'_>) {
    *state.voice_install_in_progress = true;
    *state.voice_install_status = Some(("Installing voice dependencies...".to_string(), false));

    // Use the selected model from settings (defaults to "base")
    let model_name = if state.voice_settings_model.is_empty() {
        "base"
    } else {
        state.voice_settings_model.as_str()
    };

    // Use async installation to avoid blocking the UI thread
    let handle =
        crate::gui::voice::install::install_voice_dependencies_async(state.work_dir, model_name);
    *state.voice_install_handle = Some(handle);
}

/// Render voice test section with microphone test button and status
fn render_voice_test_section(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(RichText::new("Test Microphone").color(TEXT_PRIMARY));
    ui.add_space(4.0);
    ui.label(
        RichText::new("Test recording and transcription. This will also request microphone permission if needed.")
            .small()
            .color(TEXT_MUTED),
    );
    ui.add_space(8.0);

    // Check dependencies status
    let sox_available = std::process::Command::new("which")
        .arg("rec")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let whisper_available = std::process::Command::new("which")
        .arg("whisper-cli")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let model_name = if state.voice_settings_model.is_empty() {
        "base"
    } else {
        state.voice_settings_model.as_str()
    };
    let model_path = state
        .work_dir
        .join(".kyco")
        .join("whisper-models")
        .join(format!("ggml-{}.bin", model_name));
    let model_available = model_path.exists();

    render_dependency_status(ui, sox_available, whisper_available, model_available, model_name);
    ui.add_space(8.0);

    let all_deps_available = sox_available && whisper_available && model_available;
    render_test_button(ui, state, all_deps_available);
    render_test_result(ui, state);
}

/// Render dependency status icons
fn render_dependency_status(
    ui: &mut egui::Ui,
    sox_available: bool,
    whisper_available: bool,
    model_available: bool,
    model_name: &str,
) {
    ui.horizontal(|ui| {
        let (sox_icon, sox_color) = if sox_available {
            ("✓", ACCENT_GREEN)
        } else {
            ("✗", ACCENT_RED)
        };
        ui.label(RichText::new(sox_icon).color(sox_color));
        ui.label(RichText::new("sox").monospace().color(TEXT_MUTED));
        ui.add_space(12.0);

        let (whisper_icon, whisper_color) = if whisper_available {
            ("✓", ACCENT_GREEN)
        } else {
            ("✗", ACCENT_RED)
        };
        ui.label(RichText::new(whisper_icon).color(whisper_color));
        ui.label(RichText::new("whisper-cli").monospace().color(TEXT_MUTED));
        ui.add_space(12.0);

        let (model_icon, model_color) = if model_available {
            ("✓", ACCENT_GREEN)
        } else {
            ("✗", ACCENT_RED)
        };
        ui.label(RichText::new(model_icon).color(model_color));
        ui.label(
            RichText::new(format!("{} model", model_name))
                .monospace()
                .color(TEXT_MUTED),
        );
    });
}

/// Render test microphone button
fn render_test_button(ui: &mut egui::Ui, state: &mut SettingsState<'_>, all_deps_available: bool) {
    let is_testing = matches!(
        state.voice_test_status,
        VoiceTestStatus::Recording | VoiceTestStatus::Transcribing
    );

    let button_text = match &*state.voice_test_status {
        VoiceTestStatus::Idle => "Test Microphone (3 sec)",
        VoiceTestStatus::Recording => "● Recording...",
        VoiceTestStatus::Transcribing => "◌ Transcribing...",
        VoiceTestStatus::Success => "Test Again",
        VoiceTestStatus::Error(_) => "Try Again",
    };

    let button_enabled = all_deps_available && !is_testing;

    ui.horizontal(|ui| {
        let button = ui.add_enabled(
            button_enabled,
            egui::Button::new(RichText::new(button_text).color(if button_enabled {
                ACCENT_CYAN
            } else {
                TEXT_MUTED
            })),
        );

        if button.clicked() {
            start_voice_test(state);
        }

        if !all_deps_available {
            ui.label(
                RichText::new("(install dependencies first)")
                    .small()
                    .color(TEXT_DIM),
            );
        }
    });
}

/// Render test result (success or error)
fn render_test_result(ui: &mut egui::Ui, state: &SettingsState<'_>) {
    match &*state.voice_test_status {
        VoiceTestStatus::Success => {
            if let Some(result) = &state.voice_test_result {
                ui.add_space(8.0);
                egui::Frame::NONE
                    .fill(Color32::from_rgb(20, 40, 20))
                    .corner_radius(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("✓ Transcription:").color(ACCENT_GREEN));
                        ui.label(RichText::new(result).color(TEXT_PRIMARY));
                    });
            }
        }
        VoiceTestStatus::Error(msg) => {
            ui.add_space(8.0);
            egui::Frame::NONE
                .fill(Color32::from_rgb(40, 20, 20))
                .corner_radius(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.label(RichText::new(format!("✗ {}", msg)).color(ACCENT_RED));
                });
        }
        _ => {}
    }
}

/// Start voice test - records 3 seconds and transcribes
fn start_voice_test(state: &mut SettingsState<'_>) {
    *state.voice_test_status = VoiceTestStatus::Recording;
    *state.voice_test_result = None;

    let work_dir = state.work_dir.to_path_buf();
    let model_name = if state.voice_settings_model.is_empty() {
        "base".to_string()
    } else {
        state.voice_settings_model.clone()
    };
    let language = state.voice_settings_language.clone();

    // Run test synchronously (blocks UI briefly but avoids thread resource leak)
    // For async version, would need channels or Arc<Mutex> for state updates
    match run_voice_test_sync(&work_dir, &model_name, &language) {
        Ok(text) => {
            *state.voice_test_status = VoiceTestStatus::Success;
            *state.voice_test_result = Some(text);
        }
        Err(e) => {
            *state.voice_test_status = VoiceTestStatus::Error(e);
        }
    }
}

/// Run voice test synchronously (3 second recording + transcription)
fn run_voice_test_sync(
    work_dir: &std::path::Path,
    model_name: &str,
    language: &str,
) -> Result<String, String> {
    use std::process::Command;

    let kyco_dir = work_dir.join(".kyco");
    std::fs::create_dir_all(&kyco_dir).map_err(|e| format!("Failed to create .kyco dir: {}", e))?;

    let recording_path = kyco_dir.join("voice_test.wav");
    let model_path = kyco_dir
        .join("whisper-models")
        .join(format!("ggml-{}.bin", model_name));

    // Convert paths to strings, returning error if invalid UTF-8
    let recording_path_str = recording_path
        .to_str()
        .ok_or_else(|| "Recording path contains invalid UTF-8 characters".to_string())?;
    let model_path_str = model_path
        .to_str()
        .ok_or_else(|| "Model path contains invalid UTF-8 characters".to_string())?;

    // Record 3 seconds of audio
    let rec_result = Command::new("rec")
        .args([
            "-r",
            "16000", // 16kHz sample rate (whisper requirement)
            "-c",
            "1", // Mono
            "-b",
            "16", // 16-bit
            recording_path_str,
            "trim",
            "0",
            "3", // Record exactly 3 seconds
        ])
        .output();

    match rec_result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Recording failed: {}", stderr));
            }
        }
        Err(e) => {
            return Err(format!("Failed to start recording: {}", e));
        }
    }

    if !recording_path.exists() {
        return Err("Recording file not created".to_string());
    }

    let mut whisper_args = vec![
        "-m".to_string(),
        model_path_str.to_string(),
        "-f".to_string(),
        recording_path_str.to_string(),
        "--no-timestamps".to_string(),
    ];

    if language != "auto" {
        whisper_args.push("-l".to_string());
        whisper_args.push(language.to_string());
    }

    let whisper_result = Command::new("whisper-cli").args(&whisper_args).output();

    let _ = std::fs::remove_file(&recording_path);

    match whisper_result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Transcription failed: {}", stderr));
            }
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if text.is_empty() {
                Err("No speech detected".to_string())
            } else {
                Ok(text)
            }
        }
        Err(e) => Err(format!("Failed to run whisper: {}", e)),
    }
}
