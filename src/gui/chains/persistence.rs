//! Chain persistence operations

use std::fs;

use super::state::{ChainEditorState, ChainStepEdit};
use crate::config::ModeChain;

/// Load a chain's data into the editor state
pub fn load_chain_for_editing(state: &mut ChainEditorState<'_>, chain_name: &str) {
    if let Some(chain) = state.config.chain.get(chain_name) {
        *state.chain_edit_name = chain_name.to_string();
        *state.chain_edit_description = chain.description.clone().unwrap_or_default();
        *state.chain_edit_steps = chain.steps.iter().map(ChainStepEdit::from).collect();
        *state.chain_edit_stop_on_failure = chain.stop_on_failure;
    }
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

    if state.chain_edit_steps.is_empty() {
        *state.chain_edit_status = Some(("Chain must have at least one step".to_string(), true));
        return;
    }

    // Check all steps have modes
    for (i, step) in state.chain_edit_steps.iter().enumerate() {
        if step.mode.trim().is_empty() {
            *state.chain_edit_status = Some((format!("Step {} must have a mode selected", i + 1), true));
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
        steps: state.chain_edit_steps.iter().map(|s| s.to_chain_step()).collect(),
        stop_on_failure: *state.chain_edit_stop_on_failure,
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
