//! General settings section

use eframe::egui::{self, RichText};

use crate::gui::theme::{ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

use super::super::helpers::{render_checkbox_field, render_section_frame, render_text_field};
use super::super::save::save_settings_to_config;
use super::super::state::SettingsState;

/// Render General Settings section (includes config management)
pub fn render_settings_general(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(
        RichText::new("General Settings")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);

    // Config file info
    let config_path = crate::config::Config::global_config_path();
    let config_exists = config_path.exists();

    render_section_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Config:").color(TEXT_MUTED));
            let path_str = config_path.display().to_string();
            ui.label(RichText::new(&path_str).monospace().small().color(TEXT_DIM));
            if ui.small_button("📋").on_hover_text("Copy path").clicked() {
                ui.ctx().copy_text(path_str);
            }
        });

        // Last modified
        if config_exists {
            if let Ok(metadata) = std::fs::metadata(&config_path) {
                if let Ok(modified) = metadata.modified() {
                    let duration = std::time::SystemTime::now()
                        .duration_since(modified)
                        .unwrap_or_default();
                    let ago = if duration.as_secs() < 60 {
                        format!("{} sec ago", duration.as_secs())
                    } else if duration.as_secs() < 3600 {
                        format!("{} min ago", duration.as_secs() / 60)
                    } else {
                        format!("{} hours ago", duration.as_secs() / 3600)
                    };
                    ui.label(
                        RichText::new(format!("Last modified: {}", ago))
                            .small()
                            .color(TEXT_DIM),
                    );
                }
            }
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

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

    // Action buttons
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new("Save Settings").color(ACCENT_GREEN))
            .clicked()
        {
            save_settings_to_config(state);
        }

        ui.add_space(8.0);

        let reset_btn = ui.button(RichText::new("Reset to Defaults").color(ACCENT_CYAN));
        if reset_btn.clicked() {
            match reset_config_to_defaults(state.work_dir) {
                Ok(_) => {
                    if let Ok(new_config) = crate::config::Config::load() {
                        *state.config = new_config;
                        *state.settings_status =
                            Some(("Config reset to defaults and reloaded.".to_string(), false));
                    } else {
                        *state.settings_status =
                            Some(("Config reset but reload failed.".to_string(), false));
                    }
                }
                Err(e) => {
                    *state.settings_status = Some((format!("Reset failed: {}", e), true));
                }
            }
        }
        reset_btn.on_hover_text("Reset config to defaults (18 modes, 15 chains)");

        ui.add_space(8.0);

        if ui
            .button(RichText::new("Export").color(TEXT_PRIMARY))
            .on_hover_text("Copy config to clipboard")
            .clicked()
        {
            match export_config_to_string(state.work_dir) {
                Ok(content) => {
                    ui.ctx().copy_text(content);
                    *state.settings_status =
                        Some(("Config copied to clipboard".to_string(), false));
                }
                Err(e) => {
                    *state.settings_status = Some((format!("Export failed: {}", e), true));
                }
            }
        }

        ui.add_space(8.0);

        if ui
            .button(RichText::new("Open").color(TEXT_PRIMARY))
            .on_hover_text("Open config in editor")
            .clicked()
            && config_exists
        {
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(&config_path).spawn();
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

/// Reset global config to defaults (uses internal defaults with all modes/chains)
fn reset_config_to_defaults(_work_dir: &std::path::Path) -> Result<(), String> {
    use crate::cli::init::build_default_config;
    use crate::config::Config;

    let config_path = Config::global_config_path();
    let config_dir = Config::global_config_dir();

    // Preserve the HTTP token if it exists
    let http_token = if config_path.exists() {
        Config::from_file(&config_path).ok().and_then(|c| {
            if c.settings.gui.http_token.is_empty() {
                None
            } else {
                Some(c.settings.gui.http_token)
            }
        })
    } else {
        None
    };

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create ~/.kyco directory: {}", e))?;

    // Write full default config from embedded internal defaults
    let mut content = build_default_config();

    // Restore HTTP token if we had one
    if let Some(token) = http_token {
        content = content.replace("http_token = \"\"", &format!("http_token = \"{}\"", token));
    }

    std::fs::write(&config_path, content).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

/// Export global config to string
fn export_config_to_string(_work_dir: &std::path::Path) -> Result<String, String> {
    let config_path = crate::config::Config::global_config_path();

    if !config_path.exists() {
        return Err("No config file found".to_string());
    }

    std::fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))
}
