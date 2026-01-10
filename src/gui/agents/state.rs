use std::path::Path;

use crate::config::Config;
use crate::gui::app::ViewMode;

/// State for agent editing UI
pub struct AgentEditorState<'a> {
    pub selected_agent: &'a mut Option<String>,
    pub agent_edit_name: &'a mut String,
    pub agent_edit_aliases: &'a mut String,
    pub agent_edit_cli_type: &'a mut String,
    pub agent_edit_model: &'a mut String,
    pub agent_edit_permission_mode: &'a mut String,
    pub agent_edit_sandbox: &'a mut String,
    pub agent_edit_ask_for_approval: &'a mut String,
    pub agent_edit_mode: &'a mut String,
    pub agent_edit_system_prompt_mode: &'a mut String,
    pub agent_edit_disallowed_tools: &'a mut String,
    pub agent_edit_allowed_tools: &'a mut String,
    pub agent_edit_status: &'a mut Option<(String, bool)>,
    // Token pricing fields (per 1M tokens)
    pub agent_edit_price_input: &'a mut String,
    pub agent_edit_price_cached_input: &'a mut String,
    pub agent_edit_price_output: &'a mut String,
    // Safety settings
    pub agent_edit_allow_dangerous_bypass: &'a mut bool,
    pub view_mode: &'a mut ViewMode,
    pub config: &'a mut Config,
    pub work_dir: &'a Path,
}
