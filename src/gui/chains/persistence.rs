//! Chain persistence operations

use std::collections::HashSet;
use std::fs;

use regex::Regex;

use super::state::{ChainEditorState, ChainStepEdit, StateDefinitionEdit};
use crate::config::ModeChain;

/// Load a chain's data into the editor state
pub fn load_chain_for_editing(state: &mut ChainEditorState<'_>, chain_name: &str) {
    if let Some(chain) = state.config.chain.get(chain_name) {
        *state.chain_edit_name = chain_name.to_string();
        *state.chain_edit_description = chain.description.clone().unwrap_or_default();
        *state.chain_edit_states = chain.states.iter().map(StateDefinitionEdit::from).collect();
        *state.chain_edit_steps = chain.steps.iter().map(ChainStepEdit::from).collect();
        *state.chain_edit_stop_on_failure = chain.stop_on_failure;
        *state.chain_edit_pass_full_response = chain.pass_full_response;
    }
}

/// Validate state definitions and collect valid state IDs
fn validate_states(states: &[StateDefinitionEdit]) -> Result<HashSet<String>, String> {
    let mut state_ids = HashSet::new();

    for (i, state_def) in states.iter().enumerate() {
        let id = state_def.id.trim();

        // Check ID is non-empty
        if id.is_empty() {
            return Err(format!("State {} has empty ID", i + 1));
        }

        // Check ID is unique
        if !state_ids.insert(id.to_string()) {
            return Err(format!("Duplicate state ID: '{}'", id));
        }

        // Check patterns are non-empty
        let patterns: Vec<&str> = state_def
            .patterns
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if patterns.is_empty() {
            return Err(format!("State '{}' has no patterns defined", id));
        }

        // Validate regex patterns if is_regex is enabled
        if state_def.is_regex {
            for pattern in &patterns {
                if let Err(e) = Regex::new(pattern) {
                    return Err(format!(
                        "State '{}': invalid regex '{}' - {}",
                        id, pattern, e
                    ));
                }
            }
        }
    }

    Ok(state_ids)
}

/// Validate that trigger_on/skip_on references exist in defined states
fn validate_step_state_refs(
    steps: &[ChainStepEdit],
    valid_state_ids: &HashSet<String>,
) -> Result<(), String> {
    for (i, step) in steps.iter().enumerate() {
        // Skip first step (no triggers)
        if i == 0 {
            continue;
        }

        // Validate trigger_on references
        for state_ref in step
            .trigger_on
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            if !valid_state_ids.contains(state_ref) {
                return Err(format!(
                    "Step {}: trigger_on references unknown state '{}'. Available: {}",
                    i + 1,
                    state_ref,
                    valid_state_ids
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        // Validate skip_on references
        for state_ref in step
            .skip_on
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            if !valid_state_ids.contains(state_ref) {
                return Err(format!(
                    "Step {}: skip_on references unknown state '{}'. Available: {}",
                    i + 1,
                    state_ref,
                    valid_state_ids
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    }

    Ok(())
}

/// Save the current chain to the config file
pub fn save_chain_to_config(state: &mut ChainEditorState<'_>, is_new: bool) {
    let name = if is_new {
        state.chain_edit_name.trim().to_string()
    } else {
        state.selected_chain.clone().unwrap_or_default()
    };

    if name.is_empty() {
        *state.chain_edit_status = Some(("Chain name cannot be empty".to_string(), true));
        return;
    }

    // Warn about overwriting existing chain when creating new
    if is_new && state.config.chain.contains_key(&name) {
        *state.chain_edit_status = Some((
            format!(
                "A chain named '{}' already exists. Choose a different name or edit the existing chain.",
                name
            ),
            true,
        ));
        return;
    }

    if state.chain_edit_steps.is_empty() {
        *state.chain_edit_status = Some(("Chain must have at least one step".to_string(), true));
        return;
    }

    // Check all steps have valid modes
    for (i, step) in state.chain_edit_steps.iter().enumerate() {
        let mode_name = step.mode.trim();
        if mode_name.is_empty() {
            *state.chain_edit_status =
                Some((format!("Step {} must have a mode selected", i + 1), true));
            return;
        }
        // Validate that mode exists in config
        if !state.config.mode.contains_key(mode_name) {
            *state.chain_edit_status = Some((
                format!("Step {}: mode '{}' does not exist", i + 1, mode_name),
                true,
            ));
            return;
        }
    }

    // Validate state definitions (if any exist)
    let valid_state_ids = if !state.chain_edit_states.is_empty() {
        match validate_states(&state.chain_edit_states) {
            Ok(ids) => ids,
            Err(e) => {
                *state.chain_edit_status = Some((e, true));
                return;
            }
        }
    } else {
        HashSet::new()
    };

    // Validate trigger_on/skip_on references (only if states are defined)
    if !valid_state_ids.is_empty() {
        if let Err(e) = validate_step_state_refs(&state.chain_edit_steps, &valid_state_ids) {
            *state.chain_edit_status = Some((e, true));
            return;
        }
    }

    // Build chain
    let chain = ModeChain {
        description: if state.chain_edit_description.trim().is_empty() {
            None
        } else {
            Some(state.chain_edit_description.clone())
        },
        states: state
            .chain_edit_states
            .iter()
            .map(|s| s.to_state_definition())
            .collect(),
        steps: state
            .chain_edit_steps
            .iter()
            .map(|s| s.to_chain_step())
            .collect(),
        stop_on_failure: *state.chain_edit_stop_on_failure,
        pass_full_response: *state.chain_edit_pass_full_response,
    };

    // Update config
    state.config.chain.insert(name.clone(), chain);

    // Save to file
    if let Err(e) = save_config_to_file(state) {
        *state.chain_edit_status = Some((format!("Failed to save: {}", e), true));
        return;
    }

    if is_new {
        *state.selected_chain = Some(name.clone());
        *state.chain_edit_name = name;
    }

    *state.chain_edit_status = Some(("Chain saved successfully".to_string(), false));
}

/// Delete the current chain from the config
pub fn delete_chain_from_config(state: &mut ChainEditorState<'_>) {
    let name = match &*state.selected_chain {
        Some(n) => n.clone(),
        None => return,
    };

    state.config.chain.remove(&name);

    if let Err(e) = save_config_to_file(state) {
        *state.chain_edit_status = Some((format!("Failed to delete: {}", e), true));
        return;
    }

    *state.selected_chain = None;
    *state.chain_edit_status = None;
    // Clear edit fields to avoid stale data
    state.chain_edit_name.clear();
    state.chain_edit_description.clear();
    state.chain_edit_states.clear();
    state.chain_edit_steps.clear();
    *state.chain_edit_stop_on_failure = true;
    *state.chain_edit_pass_full_response = true;
}

/// Save the config to the config file
fn save_config_to_file(state: &ChainEditorState<'_>) -> anyhow::Result<()> {
    let config_path = state.work_dir.join(".kyco/config.toml");

    // Ensure directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(&state.config)?;
    fs::write(&config_path, content)?;

    Ok(())
}
