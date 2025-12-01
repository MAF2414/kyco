//! Mode configuration persistence (save/delete/load operations)

use super::state::ModeEditorState;
use crate::config::ModeConfig;

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

/// Save mode to config file
///
/// Uses proper TOML serialization to avoid config file corruption.
pub fn save_mode_to_config(state: &mut ModeEditorState<'_>, is_new: bool) {
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
pub fn delete_mode_from_config(state: &mut ModeEditorState<'_>) {
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
