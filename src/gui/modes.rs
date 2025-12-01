//! Mode settings component for the GUI
//!
//! Renders the modes configuration view where users can:
//! - List all available modes
//! - Create new modes
//! - Edit existing modes (aliases, prompt template, system prompt, etc.)
//! - Delete modes

use eframe::egui::{self, RichText, ScrollArea};
use std::path::Path;

use super::app::{
    ViewMode, ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_PRIMARY, BG_SECONDARY, TEXT_DIM,
    TEXT_MUTED, TEXT_PRIMARY,
};
use crate::config::{Config, ModeConfig};

/// State for mode editing UI
pub struct ModeEditorState<'a> {
    pub selected_mode: &'a mut Option<String>,
    pub mode_edit_name: &'a mut String,
    pub mode_edit_aliases: &'a mut String,
    pub mode_edit_prompt: &'a mut String,
    pub mode_edit_system_prompt: &'a mut String,
    pub mode_edit_readonly: &'a mut bool,
    pub mode_edit_status: &'a mut Option<(String, bool)>,
    pub mode_edit_agent: &'a mut String,
    pub mode_edit_allowed_tools: &'a mut String,
    pub mode_edit_disallowed_tools: &'a mut String,
    pub view_mode: &'a mut ViewMode,
    pub config: &'a mut Config,
    pub work_dir: &'a Path,
}

/// Render the modes configuration view
pub fn render_modes(ctx: &egui::Context, state: &mut ModeEditorState<'_>) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(BG_PRIMARY).inner_margin(16.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("📋 MODES")
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
                        if state.selected_mode.is_some() {
                            ui.add_space(8.0);
                            if ui
                                .button(RichText::new("← Back").color(TEXT_DIM))
                                .clicked()
                            {
                                *state.selected_mode = None;
                                *state.mode_edit_status = None;
                            }
                        }
                    });
                });
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(16.0);

                if let Some(mode_name) = state.selected_mode.clone() {
                    // Edit mode view
                    render_mode_editor(ui, state, &mode_name);
                } else {
                    // List view
                    render_modes_list(ui, state);
                }
            });
        });
}

/// Render the list of available modes
fn render_modes_list(ui: &mut egui::Ui, state: &mut ModeEditorState<'_>) {
    ui.label(RichText::new("Available Modes").monospace().color(TEXT_PRIMARY));
    ui.add_space(8.0);
    ui.label(
        RichText::new("Modes define prompt templates for different task types. Click to edit.")
            .color(TEXT_DIM),
    );
    ui.add_space(12.0);

    // Get modes from config
    let modes: Vec<(String, String)> = state
        .config
        .mode
        .iter()
        .map(|(name, mode)| {
            let aliases = mode.aliases.join(", ");
            (name.clone(), aliases)
        })
        .collect();

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (name, aliases) in &modes {
                egui::Frame::none()
                    .fill(BG_SECONDARY)
                    .corner_radius(4.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        let response = ui.horizontal(|ui| {
                            ui.label(RichText::new(name).monospace().color(ACCENT_GREEN));
                            if !aliases.is_empty() {
                                ui.label(
                                    RichText::new(format!("({})", aliases))
                                        .small()
                                        .color(TEXT_MUTED),
                                );
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(RichText::new("→").color(TEXT_DIM));
                                },
                            );
                        });
                        if response.response.interact(egui::Sense::click()).clicked() {
                            *state.selected_mode = Some(name.clone());
                            load_mode_for_editing(state, name);
                        }
                    });
                ui.add_space(4.0);
            }

            // Add new mode button
            ui.add_space(12.0);
            if ui
                .button(RichText::new("+ Add New Mode").color(ACCENT_CYAN))
                .clicked()
            {
                *state.selected_mode = Some("__new__".to_string());
                state.mode_edit_name.clear();
                state.mode_edit_aliases.clear();
                state.mode_edit_prompt.clear();
                state.mode_edit_system_prompt.clear();
                state.mode_edit_agent.clear();
                state.mode_edit_allowed_tools.clear();
                state.mode_edit_disallowed_tools.clear();
                *state.mode_edit_readonly = false;
                *state.mode_edit_status = None;
            }
        });
}

/// Load mode data for editing
pub fn load_mode_for_editing(state: &mut ModeEditorState<'_>, name: &str) {
    if let Some(mode) = state.config.mode.get(name) {
        *state.mode_edit_name = name.to_string();
        *state.mode_edit_aliases = mode.aliases.join(", ");
        *state.mode_edit_prompt = mode.prompt.clone().unwrap_or_default();
        *state.mode_edit_system_prompt = mode.system_prompt.clone().unwrap_or_default();
        *state.mode_edit_agent = mode.agent.clone().unwrap_or_default();
        *state.mode_edit_allowed_tools = mode.allowed_tools.join(", ");
        *state.mode_edit_disallowed_tools = mode.disallowed_tools.join(", ");
        *state.mode_edit_readonly = mode.disallowed_tools.contains(&"Write".to_string())
            || mode.disallowed_tools.contains(&"Edit".to_string());
        *state.mode_edit_status = None;
    }
}

/// Render the mode editor form
fn render_mode_editor(ui: &mut egui::Ui, state: &mut ModeEditorState<'_>, mode_name: &str) {
    let is_new = mode_name == "__new__";
    let title = if is_new {
        "Create New Mode".to_string()
    } else {
        format!("Edit Mode: {}", mode_name)
    };

    ui.label(RichText::new(&title).monospace().color(TEXT_PRIMARY));
    ui.add_space(16.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Name (only editable for new modes)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Name:").color(TEXT_MUTED));
                if is_new {
                    ui.add(
                        egui::TextEdit::singleline(state.mode_edit_name)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .desired_width(200.0),
                    );
                } else {
                    ui.label(
                        RichText::new(&*state.mode_edit_name)
                            .monospace()
                            .color(ACCENT_GREEN),
                    );
                }
            });
            ui.add_space(8.0);

            // Aliases
            ui.horizontal(|ui| {
                ui.label(RichText::new("Aliases:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.mode_edit_aliases)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("r, ref")
                        .desired_width(200.0),
                );
            });
            ui.add_space(8.0);

            // Default agent
            ui.horizontal(|ui| {
                ui.label(RichText::new("Default Agent:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.mode_edit_agent)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("claude (optional)")
                        .desired_width(200.0),
                );
            });
            ui.add_space(8.0);

            // Allowed tools
            ui.horizontal(|ui| {
                ui.label(RichText::new("Allowed Tools:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.mode_edit_allowed_tools)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("Read, Grep, Glob (empty = all)")
                        .desired_width(300.0),
                );
            });
            ui.add_space(8.0);

            // Disallowed tools
            ui.horizontal(|ui| {
                ui.label(RichText::new("Disallowed Tools:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.mode_edit_disallowed_tools)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("Write, Edit")
                        .desired_width(300.0),
                );
            });
            ui.add_space(8.0);

            // Read-only toggle (convenience)
            ui.horizontal(|ui| {
                ui.checkbox(state.mode_edit_readonly, "");
                ui.label(
                    RichText::new("Read-only (auto-sets disallowed: Write, Edit)").color(TEXT_DIM),
                );
            });
            ui.add_space(16.0);

            // Prompt template
            ui.label(RichText::new("Prompt Template:").color(TEXT_MUTED));
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Placeholders: {file}, {line}, {target}, {mode}, {description}, {scope_type}",
                )
                .small()
                .color(TEXT_MUTED),
            );
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::multiline(state.mode_edit_prompt)
                    .font(egui::TextStyle::Monospace)
                    .text_color(TEXT_PRIMARY)
                    .desired_width(f32::INFINITY)
                    .desired_rows(8),
            );
            ui.add_space(16.0);

            // System prompt
            ui.label(RichText::new("System Prompt:").color(TEXT_MUTED));
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::multiline(state.mode_edit_system_prompt)
                    .font(egui::TextStyle::Monospace)
                    .text_color(TEXT_PRIMARY)
                    .desired_width(f32::INFINITY)
                    .desired_rows(12),
            );
            ui.add_space(16.0);

            // Status message
            if let Some((msg, is_error)) = &state.mode_edit_status {
                let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
                ui.label(RichText::new(msg.as_str()).color(color));
                ui.add_space(8.0);
            }

            // Save button
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new("💾 Save to Config").color(ACCENT_GREEN))
                    .clicked()
                {
                    save_mode_to_config(state, is_new);
                }
                if !is_new {
                    ui.add_space(16.0);
                    if ui
                        .button(RichText::new("🗑 Delete").color(ACCENT_RED))
                        .clicked()
                    {
                        delete_mode_from_config(state);
                    }
                }
            });
        });
}

/// Save mode to config file
///
/// Uses proper TOML serialization to avoid config file corruption.
fn save_mode_to_config(state: &mut ModeEditorState<'_>, is_new: bool) {
    let name = if is_new {
        state.mode_edit_name.trim().to_lowercase()
    } else {
        state.mode_edit_name.clone()
    };

    if name.is_empty() {
        *state.mode_edit_status = Some(("Mode name cannot be empty".to_string(), true));
        return;
    }

    // Build aliases
    let aliases: Vec<String> = state
        .mode_edit_aliases
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Build allowed/disallowed tools
    let allowed_tools: Vec<String> = state
        .mode_edit_allowed_tools
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let disallowed_tools: Vec<String> = if *state.mode_edit_readonly {
        vec!["Write".to_string(), "Edit".to_string()]
    } else {
        state
            .mode_edit_disallowed_tools
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    // Create the ModeConfig struct
    let mode_config = ModeConfig {
        agent: if state.mode_edit_agent.is_empty() {
            None
        } else {
            Some(state.mode_edit_agent.clone())
        },
        target_default: None,
        scope_default: None,
        prompt: if state.mode_edit_prompt.is_empty() {
            None
        } else {
            Some(state.mode_edit_prompt.clone())
        },
        system_prompt: if state.mode_edit_system_prompt.is_empty() {
            None
        } else {
            Some(state.mode_edit_system_prompt.clone())
        },
        allowed_tools,
        disallowed_tools,
        aliases,
        output_states: Vec::new(),
    };

    // Update the in-memory config (insert or replace)
    state.config.mode.insert(name.clone(), mode_config);

    // Serialize entire config using proper TOML serialization
    let config_path = state.work_dir.join(".kyco").join("config.toml");
    match toml::to_string_pretty(&state.config) {
        Ok(toml_content) => {
            if let Err(e) = std::fs::write(&config_path, &toml_content) {
                *state.mode_edit_status = Some((format!("Failed to write config: {}", e), true));
                return;
            }
            *state.mode_edit_status = Some(("Mode saved!".to_string(), false));
            if is_new {
                *state.selected_mode = Some(name);
            }
        }
        Err(e) => {
            *state.mode_edit_status = Some((format!("Failed to serialize config: {}", e), true));
        }
    }
}

/// Delete mode from config
///
/// Removes the mode from in-memory config and saves using proper TOML serialization.
fn delete_mode_from_config(state: &mut ModeEditorState<'_>) {
    if let Some(name) = &state.selected_mode.clone() {
        if name == "__new__" {
            *state.selected_mode = None;
            return;
        }

        // Remove from in-memory config
        state.config.mode.remove(name);

        // Serialize entire config using proper TOML serialization
        let config_path = state.work_dir.join(".kyco").join("config.toml");
        match toml::to_string_pretty(&state.config) {
            Ok(toml_content) => {
                if let Err(e) = std::fs::write(&config_path, &toml_content) {
                    *state.mode_edit_status =
                        Some((format!("Failed to write config: {}", e), true));
                    return;
                }
                *state.mode_edit_status = Some(("Mode deleted!".to_string(), false));
                *state.selected_mode = None;
            }
            Err(e) => {
                *state.mode_edit_status =
                    Some((format!("Failed to serialize config: {}", e), true));
            }
        }
    }
}
