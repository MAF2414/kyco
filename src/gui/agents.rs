//! Agent settings component for the GUI
//!
//! Renders the agents configuration view where users can:
//! - List all available agents
//! - Create new agents
//! - Edit existing agents (binary, cli_type, mode args, etc.)
//! - Delete agents

use eframe::egui::{self, RichText, ScrollArea};
use std::collections::HashMap;
use std::path::Path;

use super::app::{
    ViewMode, ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_PRIMARY, BG_SECONDARY, TEXT_DIM,
    TEXT_MUTED, TEXT_PRIMARY,
};
use crate::config::{AgentConfigToml, Config};
use crate::{AgentMode, CliType, SystemPromptMode};

/// State for agent editing UI
pub struct AgentEditorState<'a> {
    pub selected_agent: &'a mut Option<String>,
    pub agent_edit_name: &'a mut String,
    pub agent_edit_aliases: &'a mut String,
    pub agent_edit_binary: &'a mut String,
    pub agent_edit_cli_type: &'a mut String,
    pub agent_edit_mode: &'a mut String,
    pub agent_edit_print_args: &'a mut String,
    pub agent_edit_output_args: &'a mut String,
    pub agent_edit_repl_args: &'a mut String,
    pub agent_edit_system_prompt_mode: &'a mut String,
    pub agent_edit_disallowed_tools: &'a mut String,
    pub agent_edit_allowed_tools: &'a mut String,
    pub agent_edit_status: &'a mut Option<(String, bool)>,
    pub view_mode: &'a mut ViewMode,
    pub config: &'a mut Config,
    pub work_dir: &'a Path,
}

/// Render the agents configuration view
pub fn render_agents(ctx: &egui::Context, state: &mut AgentEditorState<'_>) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(BG_PRIMARY).inner_margin(16.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("AGENTS")
                            .monospace()
                            .size(18.0)
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("x Close").color(TEXT_DIM))
                            .clicked()
                        {
                            *state.view_mode = ViewMode::JobList;
                        }
                        if state.selected_agent.is_some() {
                            ui.add_space(8.0);
                            if ui
                                .button(RichText::new("<- Back").color(TEXT_DIM))
                                .clicked()
                            {
                                *state.selected_agent = None;
                                *state.agent_edit_status = None;
                            }
                        }
                    });
                });
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(16.0);

                if let Some(agent_name) = state.selected_agent.clone() {
                    render_agent_editor(ui, state, &agent_name);
                } else {
                    render_agents_list(ui, state);
                }
            });
        });
}

/// Render the list of available agents
fn render_agents_list(ui: &mut egui::Ui, state: &mut AgentEditorState<'_>) {
    ui.label(RichText::new("Available Agents").monospace().color(TEXT_PRIMARY));
    ui.add_space(8.0);
    ui.label(
        RichText::new("Agents define how to invoke AI CLI tools. Click to edit.").color(TEXT_DIM),
    );
    ui.add_space(12.0);

    // Get agents from config
    let agents: Vec<(String, String, String)> = state
        .config
        .agent
        .iter()
        .map(|(name, agent)| {
            let aliases = agent.aliases.join(", ");
            let binary = agent.binary.clone();
            (name.clone(), aliases, binary)
        })
        .collect();

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (name, aliases, binary) in &agents {
                egui::Frame::none()
                    .fill(BG_SECONDARY)
                    .corner_radius(4.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        let response = ui.horizontal(|ui| {
                            ui.label(RichText::new(name).monospace().color(ACCENT_CYAN));
                            if !aliases.is_empty() {
                                ui.label(
                                    RichText::new(format!("({})", aliases))
                                        .small()
                                        .color(TEXT_MUTED),
                                );
                            }
                            ui.label(
                                RichText::new(format!("-> {}", binary))
                                    .small()
                                    .color(TEXT_DIM),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(RichText::new("->").color(TEXT_DIM));
                                },
                            );
                        });
                        if response.response.interact(egui::Sense::click()).clicked() {
                            *state.selected_agent = Some(name.clone());
                            load_agent_for_editing(state, name);
                        }
                    });
                ui.add_space(4.0);
            }

            // Add new agent button
            ui.add_space(12.0);
            if ui
                .button(RichText::new("+ Add New Agent").color(ACCENT_CYAN))
                .clicked()
            {
                *state.selected_agent = Some("__new__".to_string());
                state.agent_edit_name.clear();
                state.agent_edit_aliases.clear();
                state.agent_edit_binary.clear();
                *state.agent_edit_cli_type = "claude".to_string();
                *state.agent_edit_mode = "print".to_string();
                state.agent_edit_print_args.clear();
                state.agent_edit_output_args.clear();
                state.agent_edit_repl_args.clear();
                *state.agent_edit_system_prompt_mode = "append".to_string();
                state.agent_edit_allowed_tools.clear();
                state.agent_edit_disallowed_tools.clear();
                *state.agent_edit_status = None;
            }
        });
}

/// Load agent data for editing
pub fn load_agent_for_editing(state: &mut AgentEditorState<'_>, name: &str) {
    if let Some(agent) = state.config.agent.get(name) {
        *state.agent_edit_name = name.to_string();
        *state.agent_edit_aliases = agent.aliases.join(", ");
        *state.agent_edit_binary = agent.binary.clone();
        *state.agent_edit_cli_type = format!("{:?}", agent.cli_type).to_lowercase();
        *state.agent_edit_mode = format!("{:?}", agent.mode).to_lowercase();
        *state.agent_edit_print_args = agent.print_mode_args.join(" ");
        *state.agent_edit_output_args = agent.output_format_args.join(" ");
        *state.agent_edit_repl_args = agent.repl_mode_args.join(" ");
        *state.agent_edit_system_prompt_mode =
            format!("{:?}", agent.system_prompt_mode).to_lowercase();
        *state.agent_edit_disallowed_tools = agent.disallowed_tools.join(", ");
        *state.agent_edit_allowed_tools = agent.allowed_tools.join(", ");
        *state.agent_edit_status = None;
    }
}

/// Render the agent editor form
fn render_agent_editor(ui: &mut egui::Ui, state: &mut AgentEditorState<'_>, agent_name: &str) {
    let is_new = agent_name == "__new__";
    let title = if is_new {
        "Create New Agent".to_string()
    } else {
        format!("Edit Agent: {}", agent_name)
    };

    ui.label(RichText::new(&title).monospace().color(TEXT_PRIMARY));
    ui.add_space(16.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Name (only editable for new agents)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Name:").color(TEXT_MUTED));
                if is_new {
                    ui.add(
                        egui::TextEdit::singleline(state.agent_edit_name)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .desired_width(200.0),
                    );
                } else {
                    ui.label(
                        RichText::new(&*state.agent_edit_name)
                            .monospace()
                            .color(ACCENT_CYAN),
                    );
                }
            });
            ui.add_space(8.0);

            // Aliases
            ui.horizontal(|ui| {
                ui.label(RichText::new("Aliases:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_aliases)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("c, cl")
                        .desired_width(200.0),
                );
            });
            ui.add_space(8.0);

            // Binary
            ui.horizontal(|ui| {
                ui.label(RichText::new("Binary:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_binary)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("claude")
                        .desired_width(200.0),
                );
            });
            ui.add_space(8.0);

            // CLI Type
            ui.horizontal(|ui| {
                ui.label(RichText::new("CLI Type:").color(TEXT_MUTED));
                egui::ComboBox::from_id_salt("cli_type")
                    .selected_text(&*state.agent_edit_cli_type)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            state.agent_edit_cli_type,
                            "claude".to_string(),
                            "claude",
                        );
                        ui.selectable_value(
                            state.agent_edit_cli_type,
                            "codex".to_string(),
                            "codex",
                        );
                        ui.selectable_value(
                            state.agent_edit_cli_type,
                            "gemini".to_string(),
                            "gemini",
                        );
                        ui.selectable_value(
                            state.agent_edit_cli_type,
                            "custom".to_string(),
                            "custom",
                        );
                    });
            });
            ui.add_space(8.0);

            // Execution Mode
            ui.horizontal(|ui| {
                ui.label(RichText::new("Mode:").color(TEXT_MUTED));
                egui::ComboBox::from_id_salt("agent_mode")
                    .selected_text(&*state.agent_edit_mode)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            state.agent_edit_mode,
                            "print".to_string(),
                            "print (non-interactive)",
                        );
                        ui.selectable_value(
                            state.agent_edit_mode,
                            "repl".to_string(),
                            "repl (Terminal.app)",
                        );
                    });
            });
            ui.add_space(8.0);

            // System Prompt Mode
            ui.horizontal(|ui| {
                ui.label(RichText::new("System Prompt Mode:").color(TEXT_MUTED));
                egui::ComboBox::from_id_salt("system_prompt_mode")
                    .selected_text(&*state.agent_edit_system_prompt_mode)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            state.agent_edit_system_prompt_mode,
                            "append".to_string(),
                            "append",
                        );
                        ui.selectable_value(
                            state.agent_edit_system_prompt_mode,
                            "replace".to_string(),
                            "replace",
                        );
                        ui.selectable_value(
                            state.agent_edit_system_prompt_mode,
                            "configoverride".to_string(),
                            "configoverride",
                        );
                    });
            });
            ui.add_space(16.0);

            ui.separator();
            ui.add_space(8.0);
            ui.label(
                RichText::new("Command Line Arguments")
                    .monospace()
                    .color(TEXT_PRIMARY),
            );
            ui.add_space(8.0);

            // Print mode args
            ui.horizontal(|ui| {
                ui.label(RichText::new("Print Mode Args:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_print_args)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("-p --permission-mode bypassPermissions")
                        .desired_width(400.0),
                );
            });
            ui.add_space(8.0);

            // Output format args
            ui.horizontal(|ui| {
                ui.label(RichText::new("Output Format Args:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_output_args)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("--output-format stream-json --verbose")
                        .desired_width(400.0),
                );
            });
            ui.add_space(8.0);

            // REPL mode args
            ui.horizontal(|ui| {
                ui.label(RichText::new("REPL Mode Args:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_repl_args)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("--permission-mode bypassPermissions")
                        .desired_width(400.0),
                );
            });
            ui.add_space(16.0);

            ui.separator();
            ui.add_space(8.0);
            ui.label(
                RichText::new("Tool Restrictions")
                    .monospace()
                    .color(TEXT_PRIMARY),
            );
            ui.add_space(8.0);

            // Allowed tools
            ui.horizontal(|ui| {
                ui.label(RichText::new("Allowed Tools:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_allowed_tools)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("Read, Grep (empty = all)")
                        .desired_width(300.0),
                );
            });
            ui.add_space(8.0);

            // Disallowed tools
            ui.horizontal(|ui| {
                ui.label(RichText::new("Disallowed Tools:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.agent_edit_disallowed_tools)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("Write, Edit")
                        .desired_width(300.0),
                );
            });
            ui.add_space(16.0);

            // Status message
            if let Some((msg, is_error)) = &state.agent_edit_status {
                let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
                ui.label(RichText::new(msg.as_str()).color(color));
                ui.add_space(8.0);
            }

            // Save button
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new("Save to Config").color(ACCENT_GREEN))
                    .clicked()
                {
                    save_agent_to_config(state, is_new);
                }
                if !is_new {
                    ui.add_space(16.0);
                    if ui
                        .button(RichText::new("Delete").color(ACCENT_RED))
                        .clicked()
                    {
                        delete_agent_from_config(state);
                    }
                }
            });
        });
}

/// Save agent to config file
///
/// Uses proper TOML serialization to avoid config file corruption.
fn save_agent_to_config(state: &mut AgentEditorState<'_>, is_new: bool) {
    let name = if is_new {
        state.agent_edit_name.trim().to_lowercase()
    } else {
        state.agent_edit_name.clone()
    };

    if name.is_empty() {
        *state.agent_edit_status = Some(("Agent name cannot be empty".to_string(), true));
        return;
    }

    if state.agent_edit_binary.is_empty() {
        *state.agent_edit_status = Some(("Binary path cannot be empty".to_string(), true));
        return;
    }

    // Build aliases
    let aliases: Vec<String> = state
        .agent_edit_aliases
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Build args arrays
    let print_mode_args: Vec<String> = state
        .agent_edit_print_args
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let output_format_args: Vec<String> = state
        .agent_edit_output_args
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let repl_mode_args: Vec<String> = state
        .agent_edit_repl_args
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let allowed_tools: Vec<String> = state
        .agent_edit_allowed_tools
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let disallowed_tools: Vec<String> = state
        .agent_edit_disallowed_tools
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Parse enums from string values
    let cli_type = match state.agent_edit_cli_type.as_str() {
        "claude" => CliType::Claude,
        "codex" => CliType::Codex,
        "gemini" => CliType::Gemini,
        "custom" => CliType::Custom,
        _ => CliType::Claude,
    };

    let mode = match state.agent_edit_mode.as_str() {
        "repl" => AgentMode::Repl,
        _ => AgentMode::Print,
    };

    let system_prompt_mode = match state.agent_edit_system_prompt_mode.as_str() {
        "replace" => SystemPromptMode::Replace,
        "configoverride" => SystemPromptMode::ConfigOverride,
        _ => SystemPromptMode::Append,
    };

    // Create the AgentConfigToml struct
    let agent_config = AgentConfigToml {
        aliases,
        cli_type,
        mode,
        binary: state.agent_edit_binary.clone(),
        print_mode_args,
        output_format_args,
        repl_mode_args,
        default_args: vec![],
        system_prompt_mode,
        disallowed_tools,
        allowed_tools,
        env: HashMap::new(),
    };

    // Update the in-memory config (insert or replace)
    state.config.agent.insert(name.clone(), agent_config);

    // Serialize entire config using proper TOML serialization
    let config_path = state.work_dir.join(".kyco").join("config.toml");
    match toml::to_string_pretty(&state.config) {
        Ok(toml_content) => {
            if let Err(e) = std::fs::write(&config_path, &toml_content) {
                *state.agent_edit_status = Some((format!("Failed to write config: {}", e), true));
                return;
            }
            *state.agent_edit_status = Some(("Agent saved!".to_string(), false));
            if is_new {
                *state.selected_agent = Some(name);
            }
        }
        Err(e) => {
            *state.agent_edit_status = Some((format!("Failed to serialize config: {}", e), true));
        }
    }
}

/// Delete agent from config
///
/// Removes the agent from in-memory config and saves using proper TOML serialization.
fn delete_agent_from_config(state: &mut AgentEditorState<'_>) {
    if let Some(name) = &state.selected_agent.clone() {
        if name == "__new__" {
            *state.selected_agent = None;
            return;
        }

        // Remove from in-memory config
        state.config.agent.remove(name);

        // Serialize entire config using proper TOML serialization
        let config_path = state.work_dir.join(".kyco").join("config.toml");
        match toml::to_string_pretty(&state.config) {
            Ok(toml_content) => {
                if let Err(e) = std::fs::write(&config_path, &toml_content) {
                    *state.agent_edit_status =
                        Some((format!("Failed to write config: {}", e), true));
                    return;
                }
                *state.agent_edit_status = Some(("Agent deleted!".to_string(), false));
                *state.selected_agent = None;
            }
            Err(e) => {
                *state.agent_edit_status =
                    Some((format!("Failed to serialize config: {}", e), true));
            }
        }
    }
}
