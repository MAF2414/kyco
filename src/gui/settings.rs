//! Settings component for the GUI
//!
//! Renders the settings configuration view where users can:
//! - Configure general settings (max concurrent jobs, debounce, etc.)
//! - Configure output schema for agent prompts
//! - Install IDE extensions
//! - Configure voice input settings
//! - View HTTP server status

use eframe::egui::{self, Color32, RichText, ScrollArea};
use std::path::Path;

use super::app::{
    ViewMode, ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_PRIMARY, BG_SECONDARY, TEXT_DIM,
    TEXT_MUTED, TEXT_PRIMARY,
};
use crate::config::Config;

// ═══════════════════════════════════════════════════════════════════════════
// State struct for settings UI
// ═══════════════════════════════════════════════════════════════════════════

/// State for settings editing UI
pub struct SettingsState<'a> {
    // General settings
    pub settings_max_concurrent: &'a mut String,
    pub settings_debounce_ms: &'a mut String,
    pub settings_auto_run: &'a mut bool,
    pub settings_marker_prefix: &'a mut String,
    pub settings_use_worktree: &'a mut bool,
    pub settings_scan_exclude: &'a mut String,
    pub settings_output_schema: &'a mut String,
    pub settings_status: &'a mut Option<(String, bool)>,

    // Voice settings
    pub voice_settings_mode: &'a mut String,
    pub voice_settings_keywords: &'a mut String,
    pub voice_settings_model: &'a mut String,
    pub voice_settings_language: &'a mut String,
    pub voice_settings_silence_threshold: &'a mut String,
    pub voice_settings_silence_duration: &'a mut String,
    pub voice_settings_max_duration: &'a mut String,
    pub voice_install_status: &'a mut Option<(String, bool)>,
    pub voice_install_in_progress: &'a mut bool,

    // Extension status
    pub extension_status: &'a mut Option<(String, bool)>,

    // Common state
    pub view_mode: &'a mut ViewMode,
    pub config: &'a mut Config,
    pub work_dir: &'a Path,
}

// ═══════════════════════════════════════════════════════════════════════════
// UI Helper Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Render a labeled text input field
fn render_text_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    width: f32,
    hint: Option<&str>,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT_MUTED));
        let mut edit = egui::TextEdit::singleline(value)
            .font(egui::TextStyle::Monospace)
            .text_color(TEXT_PRIMARY)
            .desired_width(width);
        if let Some(h) = hint {
            edit = edit.hint_text(h);
        }
        ui.add(edit);
    });
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

/// Render a labeled checkbox with description
fn render_checkbox_field(ui: &mut egui::Ui, value: &mut bool, label: &str, description: &str) {
    ui.horizontal(|ui| {
        ui.checkbox(value, "");
        ui.label(RichText::new(label).color(TEXT_DIM));
        ui.label(RichText::new(description).small().color(TEXT_MUTED));
    });
}

/// Render a status message (success or error)
fn render_status_message(ui: &mut egui::Ui, status: &Option<(String, bool)>) {
    if let Some((msg, is_error)) = status {
        let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
        ui.label(RichText::new(msg).color(color));
    }
}

/// Render a section frame with secondary background
fn render_section_frame<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::none()
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, add_contents)
        .inner
}

// ═══════════════════════════════════════════════════════════════════════════
// Main render function
// ═══════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════
// Section render functions
// ═══════════════════════════════════════════════════════════════════════════

/// Render General Settings section
fn render_settings_general(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
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
fn render_settings_output_schema(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
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
fn render_settings_ide_extensions(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
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

    // JetBrains (coming soon)
    render_section_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("JetBrains IDEs")
                    .monospace()
                    .color(TEXT_MUTED),
            );
            ui.label(RichText::new("(coming soon)").small().color(TEXT_MUTED));
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("IntelliJ, WebStorm, PyCharm, etc.")
                .small()
                .color(TEXT_MUTED),
        );
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
fn render_settings_voice(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
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
        });

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
}

/// Render HTTP Server info section
fn render_settings_http_server(ui: &mut egui::Ui) {
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

// ═══════════════════════════════════════════════════════════════════════════
// Helper Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Install VS Code extension
fn install_vscode_extension(state: &mut SettingsState<'_>) {
    let result = super::install::install_vscode_extension(state.work_dir);
    *state.extension_status = Some((result.message, result.is_error));
}

/// Install voice dependencies (sox, whisper-cpp)
fn install_voice_dependencies(state: &mut SettingsState<'_>) {
    *state.voice_install_in_progress = true;
    *state.voice_install_status = Some(("Installing voice dependencies...".to_string(), false));

    let result = super::voice_install::install_voice_dependencies(state.work_dir);

    *state.voice_install_status = Some((result.message, result.is_error));
    *state.voice_install_in_progress = result.in_progress;
}

/// Save settings to config file
///
/// Uses proper TOML serialization to avoid config file corruption.
fn save_settings_to_config(state: &mut SettingsState<'_>) {
    // Parse and validate values
    let max_concurrent = match state.settings_max_concurrent.trim().parse::<usize>() {
        Ok(n) if n > 0 => n,
        _ => {
            *state.settings_status =
                Some(("Invalid max concurrent jobs (must be > 0)".to_string(), true));
            return;
        }
    };

    let debounce_ms = match state.settings_debounce_ms.trim().parse::<u64>() {
        Ok(n) => n,
        _ => {
            *state.settings_status = Some(("Invalid debounce ms".to_string(), true));
            return;
        }
    };

    let scan_exclude: Vec<String> = state
        .settings_scan_exclude
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Parse voice settings
    let silence_threshold = match state.voice_settings_silence_threshold.trim().parse::<f32>() {
        Ok(n) if (0.0..=1.0).contains(&n) => n,
        _ => {
            *state.settings_status = Some((
                "Invalid silence threshold (must be 0.0-1.0)".to_string(),
                true,
            ));
            return;
        }
    };

    let silence_duration = match state.voice_settings_silence_duration.trim().parse::<f32>() {
        Ok(n) if n > 0.0 => n,
        _ => {
            *state.settings_status =
                Some(("Invalid silence duration (must be > 0)".to_string(), true));
            return;
        }
    };

    let max_duration = match state.voice_settings_max_duration.trim().parse::<f32>() {
        Ok(n) if n > 0.0 => n,
        _ => {
            *state.settings_status = Some(("Invalid max duration (must be > 0)".to_string(), true));
            return;
        }
    };

    let voice_keywords: Vec<String> = state
        .voice_settings_keywords
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Update the in-memory config with new values
    state.config.settings.max_concurrent_jobs = max_concurrent;
    state.config.settings.debounce_ms = debounce_ms;
    state.config.settings.auto_run = *state.settings_auto_run;
    state.config.settings.marker_prefix = state.settings_marker_prefix.clone();
    state.config.settings.scan_exclude = scan_exclude;
    state.config.settings.use_worktree = *state.settings_use_worktree;
    state.config.settings.gui.output_schema = state.settings_output_schema.clone();

    // Update voice settings
    state.config.settings.gui.voice.mode = state.voice_settings_mode.clone();
    state.config.settings.gui.voice.keywords = voice_keywords;
    state.config.settings.gui.voice.whisper_model = state.voice_settings_model.clone();
    state.config.settings.gui.voice.language = state.voice_settings_language.clone();
    state.config.settings.gui.voice.silence_threshold = silence_threshold;
    state.config.settings.gui.voice.silence_duration = silence_duration;
    state.config.settings.gui.voice.max_duration = max_duration;

    // Serialize entire config using proper TOML serialization
    let config_path = state.work_dir.join(".kyco").join("config.toml");
    match toml::to_string_pretty(state.config) {
        Ok(toml_content) => {
            if let Err(e) = std::fs::write(&config_path, &toml_content) {
                *state.settings_status = Some((format!("Failed to write config: {}", e), true));
                return;
            }
            *state.settings_status = Some(("Settings saved!".to_string(), false));
        }
        Err(e) => {
            *state.settings_status = Some((format!("Failed to serialize config: {}", e), true));
        }
    }
}
