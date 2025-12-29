use std::path::Path;

use crate::config::Config;
use crate::gui::app::ViewMode;

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
    pub mode_edit_session_mode: &'a mut String,
    pub mode_edit_max_turns: &'a mut String,
    pub mode_edit_model: &'a mut String,
    pub mode_edit_claude_permission: &'a mut String,
    pub mode_edit_codex_sandbox: &'a mut String,
    pub mode_edit_output_states: &'a mut String,
    pub mode_edit_state_prompt: &'a mut String,
    pub mode_edit_use_worktree: &'a mut Option<bool>,
    pub view_mode: &'a mut ViewMode,
    pub config: &'a mut Config,
    pub work_dir: &'a Path,
}
