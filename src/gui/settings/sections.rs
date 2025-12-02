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
use super::state::SettingsState;

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
        ui.add_space(8.0);

        render_text_field_with_desc(
            ui,
            "Debounce (ms):",
            state.settings_debounce_ms,
            80.0,
            "(delay before scanning after file changes)",
        );
        ui.add_space(8.0);

        render_text_field_with_desc(
            ui,
            "Marker Prefix:",
            state.settings_marker_prefix,
            80.0,
            "(e.g. @@, @, ::)",
        );
        ui.add_space(8.0);

        render_text_field(
            ui,
            "Scan Exclude:",
            state.settings_scan_exclude,
            300.0,
            Some("node_modules, .git, target"),
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
            .button(RichText::new("💾 Save Settings").color(ACCENT_GREEN))
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
        RichText::new("Template appended to agent system prompts for structured output.")
            .color(TEXT_DIM),
    );
    ui.add_space(12.0);

    render_section_frame(ui, |ui| {
        ui.label(
            RichText::new("Placeholders: ---kyco marker for YAML output")
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
        egui::Frame::none()
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
            .button(RichText::new("💾 Save Voice Settings").color(ACCENT_GREEN))
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

    egui::Frame::none()
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

    egui::Frame::none()
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

/// Render VAD (Voice Activity Detection) settings
fn render_vad_settings(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(
        RichText::new("VAD (Voice Activity Detection)")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);

    egui::Frame::none()
        .fill(Color32::from_rgb(30, 30, 35))
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new("VAD detects speech to efficiently trigger transcription in continuous mode.")
                    .small()
                    .color(TEXT_MUTED),
            );
            ui.add_space(8.0);

            // VAD enabled toggle
            ui.horizontal(|ui| {
                ui.checkbox(state.vad_enabled, "");
                ui.label(RichText::new("Enable VAD for continuous listening").color(TEXT_PRIMARY));
            });

            if *state.vad_enabled {
                ui.add_space(8.0);

                // Speech threshold
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

                // Silence duration
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
