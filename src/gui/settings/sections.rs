//! Settings section render functions
//!
//! Each function renders a distinct section of the settings panel.

use eframe::egui::{self, Color32, RichText};

use crate::gui::app::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};

use super::helpers::{
    render_checkbox_field, render_section_frame, render_status_message, render_text_field,
    render_text_field_with_desc,
};
use super::save::save_settings_to_config;
use super::state::{SettingsState, VoiceTestStatus};

/// Render General Settings section
pub fn render_settings_general(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(RichText::new("General Settings").monospace().color(TEXT_PRIMARY));
    ui.add_space(8.0);

    render_section_frame(ui, |ui| {
        render_text_field(
            ui,
            "Max Concurrent Jobs:",
            state.settings_max_concurrent,
            60.0,
            None,
        );
        ui.add_space(12.0);

        render_checkbox_field(
            ui,
            state.settings_auto_run,
            "Auto-Run",
            "(automatically start jobs when found)",
        );
        ui.add_space(4.0);

        render_checkbox_field(
            ui,
            state.settings_use_worktree,
            "Use Git Worktrees",
            "(isolate each job in separate worktree)",
        );
    });

    // Save button
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new("Save Settings").color(ACCENT_GREEN))
            .clicked()
        {
            save_settings_to_config(state);
        }
    });

    // Status message
    if let Some((msg, is_error)) = &state.settings_status {
        ui.add_space(8.0);
        let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
        ui.label(RichText::new(msg).color(color));
    }

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
}

/// Render Output Schema section
pub fn render_settings_output_schema(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(
        RichText::new("Agent Output Schema")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "YAML summary template is appended to system prompts. Optional JSON Schema enables true SDK structured output.",
        )
            .color(TEXT_DIM),
    );
    ui.add_space(12.0);

    render_section_frame(ui, |ui| {
        ui.label(RichText::new("YAML Summary Template").color(TEXT_MUTED));
        ui.add_space(4.0);
        ui.label(
            RichText::new("Placeholders: --- markers for YAML output")
                .small()
                .color(TEXT_MUTED),
        );
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::multiline(state.settings_output_schema)
                .font(egui::TextStyle::Monospace)
                .text_color(TEXT_PRIMARY)
                .desired_width(f32::INFINITY)
                .desired_rows(8),
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);

        ui.label(RichText::new("Structured Output (JSON Schema, optional)").color(TEXT_MUTED));
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "When set, Claude/Codex SDK will be asked to return JSON matching this schema.",
            )
            .small()
            .color(TEXT_MUTED),
        );
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::multiline(state.settings_structured_output_schema)
                .font(egui::TextStyle::Monospace)
                .text_color(TEXT_PRIMARY)
                .hint_text("{\n  \"type\": \"object\",\n  \"properties\": { ... }\n}")
                .desired_width(f32::INFINITY)
                .desired_rows(8),
        );
    });

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
}

/// Render IDE Extensions section
pub fn render_settings_ide_extensions(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(RichText::new("IDE Extensions").monospace().color(TEXT_PRIMARY));
    ui.add_space(8.0);
    ui.label(
        RichText::new("Install extensions to send code selections to kyco with a hotkey.")
            .color(TEXT_DIM),
    );
    ui.add_space(12.0);

    // VS Code
    render_section_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("VS Code").monospace().color(ACCENT_CYAN));
            ui.label(RichText::new("Cmd+Option+K").small().color(TEXT_MUTED));
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("Sends current selection + file path to kyco")
                .small()
                .color(TEXT_DIM),
        );
        ui.add_space(8.0);

        if ui
            .button(RichText::new("📦 Install VS Code Extension").color(ACCENT_GREEN))
            .clicked()
        {
            install_vscode_extension(state);
        }
    });

    ui.add_space(12.0);

    // JetBrains
    render_section_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("JetBrains IDEs").monospace().color(ACCENT_CYAN));
            ui.label(RichText::new("Ctrl+Alt+Y").small().color(TEXT_MUTED));
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("IntelliJ, WebStorm, PyCharm, etc.")
                .small()
                .color(TEXT_DIM),
        );
        ui.add_space(8.0);

        if ui
            .button(RichText::new("📦 Install JetBrains Plugin").color(ACCENT_GREEN))
            .clicked()
        {
            install_jetbrains_plugin(state);
        }
    });

    // Status message
    if let Some((msg, is_error)) = &state.extension_status {
        ui.add_space(16.0);
        let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
        egui::Frame::NONE
            .fill(if *is_error {
                Color32::from_rgb(40, 20, 20)
            } else {
                Color32::from_rgb(20, 40, 20)
            })
            .corner_radius(4.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(RichText::new(msg).color(color));
            });
    }

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
}

/// Render Voice Input Settings section
pub fn render_settings_voice(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(RichText::new("Voice Input").monospace().color(TEXT_PRIMARY));
    ui.add_space(8.0);
    ui.label(
        RichText::new("Configure voice input for hands-free operation.").color(TEXT_DIM),
    );
    ui.add_space(12.0);

    render_section_frame(ui, |ui| {
        // Voice mode selector
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

        // Whisper model
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

        // Language
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
                    ui.selectable_value(
                        state.voice_settings_language,
                        "en".to_string(),
                        "English",
                    );
                    ui.selectable_value(state.voice_settings_language, "de".to_string(), "German");
                    ui.selectable_value(state.voice_settings_language, "fr".to_string(), "French");
                    ui.selectable_value(
                        state.voice_settings_language,
                        "es".to_string(),
                        "Spanish",
                    );
                    ui.selectable_value(
                        state.voice_settings_language,
                        "it".to_string(),
                        "Italian",
                    );
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
                    ui.selectable_value(
                        state.voice_settings_language,
                        "zh".to_string(),
                        "Chinese",
                    );
                });
        });
        ui.add_space(12.0);

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
    });

    // Save voice settings button
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new("Save Voice Settings").color(ACCENT_GREEN))
            .clicked()
        {
            save_settings_to_config(state);
        }
    });

    // Status message (shared with general settings)
    render_status_message(ui, state.settings_status);

    // Voice dependency installation section
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

            // List required dependencies
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

            // Install button
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

            // Status message
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

            // Voice test section
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            render_voice_test_section(ui, state);
        });

    // Voice Actions section
    ui.add_space(12.0);
    render_voice_actions(ui, state);

    // VAD Settings section
    ui.add_space(12.0);
    render_vad_settings(ui, state);

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
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
                RichText::new("Speak a wakeword to trigger the corresponding mode (loaded from config):")
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
                        // Header
                        ui.label(RichText::new("Wakeword").color(TEXT_MUTED).small());
                        ui.label(RichText::new("Mode").color(TEXT_MUTED).small());
                        ui.label(RichText::new("Aliases").color(TEXT_MUTED).small());
                        ui.end_row();

                        for action in &state.voice_action_registry.actions {
                            let primary = action.wakewords.first().map(|s| s.as_str()).unwrap_or(&action.mode);
                            ui.label(RichText::new(primary).monospace().color(ACCENT_CYAN));
                            ui.label(RichText::new(format!("→ {}", action.mode)).color(TEXT_PRIMARY));

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

/// Render HTTP Server info section
pub fn render_settings_http_server(ui: &mut egui::Ui) {
    ui.label(RichText::new("HTTP Server").monospace().color(TEXT_PRIMARY));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Status:").color(TEXT_MUTED));
        ui.label(RichText::new("● Running").color(ACCENT_GREEN));
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Address:").color(TEXT_MUTED));
        ui.label(
            RichText::new("http://127.0.0.1:9876")
                .monospace()
                .color(ACCENT_CYAN),
        );
    });
    ui.add_space(8.0);
    ui.label(
        RichText::new("Extensions send selections to this endpoint.")
            .small()
            .color(TEXT_DIM),
    );
}

/// Install VS Code extension
fn install_vscode_extension(state: &mut SettingsState<'_>) {
    let result = crate::gui::install::install_vscode_extension(state.work_dir);
    *state.extension_status = Some((result.message, result.is_error));
}

/// Install JetBrains plugin
fn install_jetbrains_plugin(state: &mut SettingsState<'_>) {
    let result = crate::gui::install::install_jetbrains_plugin(state.work_dir);
    *state.extension_status = Some((result.message, result.is_error));
}

/// Install voice dependencies (sox, whisper-cpp)
fn install_voice_dependencies(state: &mut SettingsState<'_>) {
    *state.voice_install_in_progress = true;
    *state.voice_install_status = Some(("Installing voice dependencies...".to_string(), false));

    // Use the selected model from settings (defaults to "base")
    let model_name = if state.voice_settings_model.is_empty() {
        "base"
    } else {
        state.voice_settings_model.as_str()
    };

    let result =
        crate::gui::voice::install::install_voice_dependencies(state.work_dir, model_name);

    *state.voice_install_status = Some((result.message, result.is_error));
    *state.voice_install_in_progress = result.in_progress;
}

/// Render voice test section with microphone test button and status
fn render_voice_test_section(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(
        RichText::new("Test Microphone")
            .color(TEXT_PRIMARY),
    );
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

    // Status indicators
    ui.horizontal(|ui| {
        // sox status
        let (sox_icon, sox_color) = if sox_available {
            ("✓", ACCENT_GREEN)
        } else {
            ("✗", ACCENT_RED)
        };
        ui.label(RichText::new(sox_icon).color(sox_color));
        ui.label(RichText::new("sox").monospace().color(TEXT_MUTED));
        ui.add_space(12.0);

        // whisper-cli status
        let (whisper_icon, whisper_color) = if whisper_available {
            ("✓", ACCENT_GREEN)
        } else {
            ("✗", ACCENT_RED)
        };
        ui.label(RichText::new(whisper_icon).color(whisper_color));
        ui.label(RichText::new("whisper-cli").monospace().color(TEXT_MUTED));
        ui.add_space(12.0);

        // model status
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

    ui.add_space(8.0);

    let all_deps_available = sox_available && whisper_available && model_available;

    // Test button
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
            egui::Button::new(
                RichText::new(button_text).color(if button_enabled {
                    ACCENT_CYAN
                } else {
                    TEXT_MUTED
                }),
            ),
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

    // Show test result
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

    // Run test in background thread
    std::thread::spawn(move || {
        // This will be polled by the GUI - for now we do sync
        let result = run_voice_test(&work_dir, &model_name, &language);
        // Note: We can't easily update state from here, so we'll handle this differently
        // For a proper implementation, we'd use channels or Arc<Mutex>
        eprintln!("Voice test result: {:?}", result);
    });

    // For now, show recording status - the actual async handling would need
    // more infrastructure. Let's do a simpler sync version for immediate feedback.
    *state.voice_test_status = VoiceTestStatus::Recording;

    // Actually run the test synchronously for simplicity
    // (In production, this should be async with proper state updates)
    let work_dir = state.work_dir.to_path_buf();
    let model_name = if state.voice_settings_model.is_empty() {
        "base".to_string()
    } else {
        state.voice_settings_model.clone()
    };
    let language = state.voice_settings_language.clone();

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

    // Record 3 seconds of audio
    let rec_result = Command::new("rec")
        .args([
            "-r",
            "16000", // 16kHz sample rate (whisper requirement)
            "-c",
            "1", // Mono
            "-b",
            "16", // 16-bit
            recording_path.to_str().unwrap_or("test.wav"),
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

    // Check if recording was created
    if !recording_path.exists() {
        return Err("Recording file not created".to_string());
    }

    // Transcribe
    let mut whisper_args = vec![
        "-m".to_string(),
        model_path.to_str().unwrap_or("model.bin").to_string(),
        "-f".to_string(),
        recording_path.to_str().unwrap_or("test.wav").to_string(),
        "--no-timestamps".to_string(),
    ];

    if language != "auto" {
        whisper_args.push("-l".to_string());
        whisper_args.push(language.to_string());
    }

    let whisper_result = Command::new("whisper-cli").args(&whisper_args).output();

    // Clean up recording file
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

/// Run voice test (not used currently, placeholder for async version)
fn run_voice_test(
    work_dir: &std::path::Path,
    model_name: &str,
    language: &str,
) -> Result<String, String> {
    run_voice_test_sync(work_dir, model_name, language)
}

// ═══════════════════════════════════════════════════════════════════════════
// Workspace Config Section
// ═══════════════════════════════════════════════════════════════════════════

/// Render Workspace Config section with Reset/Export/Import buttons
pub fn render_settings_workspace_config(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(
        RichText::new("Workspace Configuration")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);

    let config_path = state.work_dir.join(".kyco").join("config.toml");
    let config_exists = config_path.exists();

    render_section_frame(ui, |ui| {
        // Current config path
        ui.horizontal(|ui| {
            ui.label(RichText::new("Config Path:").color(TEXT_MUTED));
            let path_str = config_path.display().to_string();
            let path_text = if path_str.len() > 50 {
                format!("...{}", &path_str[path_str.len() - 47..])
            } else {
                path_str.clone()
            };
            ui.label(RichText::new(&path_text).monospace().small().color(TEXT_DIM));

            // Copy path to clipboard
            if ui.small_button("📋").on_hover_text("Copy path to clipboard").clicked() {
                ui.ctx().copy_text(path_str);
            }
        });

        ui.add_space(8.0);

        // Config status
        if config_exists {
            if let Ok(metadata) = std::fs::metadata(&config_path) {
                if let Ok(modified) = metadata.modified() {
                    let duration = std::time::SystemTime::now()
                        .duration_since(modified)
                        .unwrap_or_default();
                    let ago = if duration.as_secs() < 60 {
                        format!("{} seconds ago", duration.as_secs())
                    } else if duration.as_secs() < 3600 {
                        format!("{} minutes ago", duration.as_secs() / 60)
                    } else if duration.as_secs() < 86400 {
                        format!("{} hours ago", duration.as_secs() / 3600)
                    } else {
                        format!("{} days ago", duration.as_secs() / 86400)
                    };
                    ui.label(RichText::new(format!("✓ Last modified: {}", ago)).small().color(ACCENT_GREEN));
                }
            }
        } else {
            ui.label(RichText::new("⚠ No config file found").small().color(ACCENT_RED));
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Action buttons
        ui.horizontal(|ui| {
            // Reset to Defaults button
            let reset_btn = ui.button(RichText::new("Reset to Defaults").color(ACCENT_CYAN));
            if reset_btn.clicked() {
                match reset_config_to_defaults(state.work_dir) {
                    Ok(_) => {
                        *state.settings_status = Some(("Config reset to defaults. Restart recommended.".to_string(), false));
                    }
                    Err(e) => {
                        *state.settings_status = Some((format!("Reset failed: {}", e), true));
                    }
                }
            }
            reset_btn.on_hover_text("Overwrite config with default values");

            ui.add_space(8.0);

            // Export to Clipboard button
            let export_btn = ui.button(RichText::new("Export to Clipboard").color(ACCENT_GREEN));
            if export_btn.clicked() {
                match export_config_to_string(state.work_dir) {
                    Ok(content) => {
                        ui.ctx().copy_text(content);
                        *state.settings_status = Some(("Config copied to clipboard".to_string(), false));
                    }
                    Err(e) => {
                        *state.settings_status = Some((format!("Export failed: {}", e), true));
                    }
                }
            }
            export_btn.on_hover_text("Copy current config.toml content to clipboard");

            ui.add_space(8.0);

            // Open in Editor button
            let open_btn = ui.button(RichText::new("Open in Editor").color(TEXT_PRIMARY));
            if open_btn.clicked() && config_exists {
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("open")
                        .arg(&config_path)
                        .spawn();
                }
                #[cfg(target_os = "linux")]
                {
                    let _ = std::process::Command::new("xdg-open")
                        .arg(&config_path)
                        .spawn();
                }
                #[cfg(target_os = "windows")]
                {
                    let _ = std::process::Command::new("cmd")
                        .args(["/C", "start", "", &config_path.display().to_string()])
                        .spawn();
                }
            }
            open_btn.on_hover_text("Open config.toml in system editor");
        });
    });

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
}

/// Reset config to defaults by calling Config::auto_init
fn reset_config_to_defaults(work_dir: &std::path::Path) -> Result<(), String> {
    use crate::config::Config;

    let config_dir = work_dir.join(".kyco");
    let config_path = config_dir.join("config.toml");

    // Create default config
    let mut default_config = Config::with_defaults();

    // Preserve the HTTP token if it exists
    if config_path.exists() {
        if let Ok(existing) = Config::from_file(&config_path) {
            if !existing.settings.gui.http_token.is_empty() {
                default_config.settings.gui.http_token = existing.settings.gui.http_token;
            }
        }
    }

    // Ensure directory exists
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create .kyco directory: {}", e))?;

    // Write config
    let content = toml::to_string_pretty(&default_config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(&config_path, content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

/// Export current config to string
fn export_config_to_string(work_dir: &std::path::Path) -> Result<String, String> {
    let config_path = work_dir.join(".kyco").join("config.toml");

    if !config_path.exists() {
        return Err("No config file found".to_string());
    }

    std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))
}

// ═══════════════════════════════════════════════════════════════════════════
// Config Import from Other Workspace
// ═══════════════════════════════════════════════════════════════════════════

use crate::workspace::Workspace;

/// Render the "Import from Workspace" section
pub fn render_settings_import_config(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    // Only show if we have a workspace registry with multiple workspaces
    let workspaces: Vec<Workspace> = if let Some(registry_arc) = state.workspace_registry {
        if let Ok(registry) = registry_arc.lock() {
            registry.list().iter().map(|w| (*w).clone()).collect()
        } else {
            return;
        }
    } else {
        return;
    };

    // Filter out current workspace
    let other_workspaces: Vec<&Workspace> = workspaces
        .iter()
        .filter(|w| w.path != state.work_dir)
        .collect();

    if other_workspaces.is_empty() {
        return; // No other workspaces to import from
    }

    ui.label(
        RichText::new("Import from Workspace")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new("Copy modes, agents, or chains from another workspace")
            .small()
            .color(TEXT_DIM),
    );
    ui.add_space(12.0);

    render_section_frame(ui, |ui| {
        // Workspace selector
        ui.horizontal(|ui| {
            ui.label(RichText::new("Source workspace:").color(TEXT_MUTED));

            // Clamp selection to valid range
            if *state.import_workspace_selected >= other_workspaces.len() {
                *state.import_workspace_selected = 0;
            }

            let selected_name = other_workspaces
                .get(*state.import_workspace_selected)
                .map(|w| w.name.as_str())
                .unwrap_or("Select...");

            egui::ComboBox::from_id_salt("import_workspace_selector")
                .selected_text(selected_name)
                .width(200.0)
                .show_ui(ui, |ui| {
                    for (idx, ws) in other_workspaces.iter().enumerate() {
                        ui.selectable_value(
                            state.import_workspace_selected,
                            idx,
                            &ws.name,
                        );
                    }
                });
        });

        ui.add_space(12.0);

        // What to import checkboxes
        ui.label(RichText::new("What to import:").color(TEXT_MUTED));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.checkbox(state.import_modes, "Modes");
            ui.checkbox(state.import_agents, "Agents");
            ui.checkbox(state.import_chains, "Chains");
            ui.checkbox(state.import_settings, "Settings");
        });

        ui.add_space(12.0);

        // Import button
        let can_import = *state.import_modes || *state.import_agents || *state.import_chains || *state.import_settings;

        ui.horizontal(|ui| {
            let import_btn = ui.add_enabled(
                can_import,
                egui::Button::new(RichText::new("Import Selected").color(if can_import { ACCENT_GREEN } else { TEXT_MUTED })),
            );

            if import_btn.clicked() {
                if let Some(source_ws) = other_workspaces.get(*state.import_workspace_selected) {
                    match import_config_from_workspace(
                        &source_ws.path,
                        state.work_dir,
                        *state.import_modes,
                        *state.import_agents,
                        *state.import_chains,
                        *state.import_settings,
                    ) {
                        Ok(msg) => {
                            *state.settings_status = Some((msg, false));
                        }
                        Err(e) => {
                            *state.settings_status = Some((format!("Import failed: {}", e), true));
                        }
                    }
                }
            }
            import_btn.on_hover_text("Import selected configuration from source workspace");
        });
    });

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
}

/// Import configuration sections from one workspace to another
fn import_config_from_workspace(
    source_dir: &std::path::Path,
    target_dir: &std::path::Path,
    import_modes: bool,
    import_agents: bool,
    import_chains: bool,
    import_settings: bool,
) -> Result<String, String> {
    use crate::config::Config;

    let source_config_path = source_dir.join(".kyco").join("config.toml");
    let target_config_path = target_dir.join(".kyco").join("config.toml");

    if !source_config_path.exists() {
        return Err("Source workspace has no config".to_string());
    }

    let source_config = Config::from_file(&source_config_path)
        .map_err(|e| format!("Failed to load source config: {}", e))?;

    let mut target_config = if target_config_path.exists() {
        Config::from_file(&target_config_path)
            .map_err(|e| format!("Failed to load target config: {}", e))?
    } else {
        Config::with_defaults()
    };

    let mut imported = Vec::new();

    // Import modes
    if import_modes && !source_config.mode.is_empty() {
        let count = source_config.mode.len();
        target_config.mode = source_config.mode;
        imported.push(format!("{} modes", count));
    }

    // Import agents
    if import_agents && !source_config.agent.is_empty() {
        let count = source_config.agent.len();
        target_config.agent = source_config.agent;
        imported.push(format!("{} agents", count));
    }

    // Import chains
    if import_chains && !source_config.chain.is_empty() {
        let count = source_config.chain.len();
        target_config.chain = source_config.chain;
        imported.push(format!("{} chains", count));
    }

    // Import settings (but preserve HTTP token)
    if import_settings {
        let http_token = target_config.settings.gui.http_token.clone();
        target_config.settings = source_config.settings;
        target_config.settings.gui.http_token = http_token; // Preserve token
        imported.push("settings".to_string());
    }

    if imported.is_empty() {
        return Err("Nothing to import".to_string());
    }

    // Write updated config
    let content = toml::to_string_pretty(&target_config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    // Ensure target directory exists
    if let Some(parent) = target_config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create target directory: {}", e))?;
    }

    std::fs::write(&target_config_path, content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(format!("Imported: {}. Restart to apply.", imported.join(", ")))
}
