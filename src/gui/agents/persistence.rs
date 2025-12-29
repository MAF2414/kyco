//! Agent configuration persistence (save/delete/load operations)

use std::collections::HashMap;

use super::state::AgentEditorState;
use crate::config::{AgentConfigToml, Config};
use crate::{SdkType, SessionMode, SystemPromptMode};

/// Load agent data for editing
pub fn load_agent_for_editing(state: &mut AgentEditorState<'_>, name: &str) {
    if let Some(agent) = state.config.agent.get(name) {
        *state.agent_edit_name = name.to_string();
        *state.agent_edit_aliases = agent.aliases.join(", ");
        *state.agent_edit_cli_type = match agent.sdk {
            SdkType::Codex => "codex".to_string(),
            _ => "claude".to_string(),
        };
        *state.agent_edit_model = agent.model.clone().unwrap_or_default();
        *state.agent_edit_mode = match agent.session_mode {
            SessionMode::Session | SessionMode::Repl => "session".to_string(),
            _ => "oneshot".to_string(),
        };
        *state.agent_edit_system_prompt_mode =
            format!("{:?}", agent.system_prompt_mode).to_lowercase();
        *state.agent_edit_disallowed_tools = agent.disallowed_tools.join(", ");
        *state.agent_edit_allowed_tools = agent.allowed_tools.join(", ");
        // Pricing fields
        *state.agent_edit_price_input = agent
            .price_input
            .map(|v| format!("{:.2}", v))
            .unwrap_or_default();
        *state.agent_edit_price_cached_input = agent
            .price_cached_input
            .map(|v| format!("{:.3}", v))
            .unwrap_or_default();
        *state.agent_edit_price_output = agent
            .price_output
            .map(|v| format!("{:.2}", v))
            .unwrap_or_default();
        *state.agent_edit_status = None;
    }
}

/// Save agent to config file
///
/// Uses proper TOML serialization to avoid config file corruption.
pub fn save_agent_to_config(state: &mut AgentEditorState<'_>, is_new: bool) {
    let name = if is_new {
        state.agent_edit_name.trim().to_lowercase()
    } else {
        state.agent_edit_name.clone()
    };

    if name.is_empty() {
        *state.agent_edit_status = Some(("Agent name cannot be empty".to_string(), true));
        return;
    }

    let aliases: Vec<String> = state
        .agent_edit_aliases
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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

    let sdk = match state.agent_edit_cli_type.as_str() {
        "claude" => SdkType::Claude,
        "codex" => SdkType::Codex,
        _ => SdkType::Claude,
    };

    let session_mode = match state.agent_edit_mode.as_str() {
        "session" => SessionMode::Session,
        _ => SessionMode::Oneshot,
    };

    let system_prompt_mode = match state.agent_edit_system_prompt_mode.as_str() {
        "replace" => SystemPromptMode::Replace,
        "configoverride" => SystemPromptMode::ConfigOverride,
        _ => SystemPromptMode::Append,
    };

    // Preserve fields not editable in the GUI (env, MCP servers, subagents) when updating an existing agent.
    let (env, mcp_servers, agents) = state
        .config
        .agent
        .get(&name)
        .map(|a| (a.env.clone(), a.mcp_servers.clone(), a.agents.clone()))
        .unwrap_or_else(|| (HashMap::new(), HashMap::new(), HashMap::new()));

    let model = if state.agent_edit_model.is_empty() {
        None
    } else {
        Some(state.agent_edit_model.clone())
    };

    // Parse pricing fields
    let price_input = state.agent_edit_price_input.trim().parse::<f64>().ok();
    let price_cached_input = state.agent_edit_price_cached_input.trim().parse::<f64>().ok();
    let price_output = state.agent_edit_price_output.trim().parse::<f64>().ok();

    let agent_config = AgentConfigToml {
        version: 0, // User-created agents start at version 0
        aliases,
        sdk,
        model,
        session_mode,
        system_prompt_mode,
        disallowed_tools,
        allowed_tools,
        env,
        mcp_servers,
        agents,
        price_input,
        price_cached_input,
        price_output,
    };

    state.config.agent.insert(name.clone(), agent_config);

    // Save config with atomic write and file locking
    let config_path = Config::global_config_path();
    if let Err(e) = state.config.save_to_file(&config_path) {
        *state.agent_edit_status = Some((format!("Failed to save config: {}", e), true));
        return;
    }
    *state.agent_edit_status = Some(("Agent saved!".to_string(), false));
    if is_new {
        *state.selected_agent = Some(name);
    }
}

/// Delete agent from config
///
/// Removes the agent from in-memory config and saves with atomic write and file locking.
pub fn delete_agent_from_config(state: &mut AgentEditorState<'_>) {
    if let Some(name) = &state.selected_agent.clone() {
        if name == "__new__" {
            *state.selected_agent = None;
            return;
        }

        state.config.agent.remove(name);

        // Save config with atomic write and file locking
        let config_path = Config::global_config_path();
        if let Err(e) = state.config.save_to_file(&config_path) {
            *state.agent_edit_status = Some((format!("Failed to save config: {}", e), true));
            return;
        }
        *state.agent_edit_status = Some(("Agent deleted!".to_string(), false));
        *state.selected_agent = None;
    }
}
