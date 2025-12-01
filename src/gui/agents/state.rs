//! State management for agent editing UI

use std::path::Path;

use crate::config::Config;
use crate::gui::app::ViewMode;

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
