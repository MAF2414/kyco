//! Detail panel types and state

use std::collections::HashMap;

use crate::agent::bridge::PermissionMode;
use crate::config::Config;
use crate::{AgentGroupId, JobId, LogEvent};

/// Actions that can be triggered from the detail panel
#[derive(Debug, Clone)]
pub enum DetailPanelAction {
    Queue(JobId),
    Apply(JobId),
    Reject(JobId),
    ViewDiff(JobId),
    /// Open the multi-agent comparison popup for this group
    CompareGroup(AgentGroupId),
    /// Stop/kill a running job
    Kill(JobId),
    /// Mark a REPL job as complete (user confirms they finished in Terminal)
    MarkComplete(JobId),
    /// Continue a session with a follow-up prompt
    Continue(JobId, String), // job_id, prompt
    /// Change permission mode for a running Claude session
    SetPermissionMode(JobId, PermissionMode),
    /// Restart a failed or rejected job with the same parameters
    Restart(JobId),
}

/// UI filters for activity log display.
///
/// Defaults to showing only text events to keep the log readable.
#[derive(Debug, Clone)]
pub struct ActivityLogFilters {
    pub show_thought: bool,
    pub show_tool_call: bool,
    pub show_tool_output: bool,
    pub show_text: bool,
    pub show_error: bool,
    pub show_system: bool,
    pub show_permission: bool,
}

impl Default for ActivityLogFilters {
    fn default() -> Self {
        Self {
            show_thought: false,
            show_tool_call: false,
            show_tool_output: false,
            show_text: true,
            show_error: false,
            show_system: false,
            show_permission: false,
        }
    }
}

impl ActivityLogFilters {
    pub(super) fn is_enabled(&self, kind: &crate::LogEventKind) -> bool {
        use crate::LogEventKind;
        match kind {
            LogEventKind::Thought => self.show_thought,
            LogEventKind::ToolCall => self.show_tool_call,
            LogEventKind::ToolOutput => self.show_tool_output,
            LogEventKind::Text => self.show_text,
            LogEventKind::Error => self.show_error,
            LogEventKind::System => self.show_system,
            LogEventKind::Permission => self.show_permission,
        }
    }

    pub(super) fn selected_summary(&self) -> String {
        let mut selected = 0usize;
        let mut label: Option<&'static str> = None;

        let mut consider = |enabled: bool, name: &'static str| {
            if enabled {
                selected += 1;
                if label.is_none() {
                    label = Some(name);
                }
            }
        };

        consider(self.show_text, "Text");
        consider(self.show_tool_call, "Tool calls");
        consider(self.show_tool_output, "Tool output");
        consider(self.show_thought, "Thought");
        consider(self.show_system, "System");
        consider(self.show_error, "Error");
        consider(self.show_permission, "Permission");

        match (selected, label) {
            (0, _) => "None".to_string(),
            (1, Some(name)) => name.to_string(),
            (n, Some(name)) => format!("{name} +{}", n.saturating_sub(1)),
            _ => "Selected".to_string(),
        }
    }
}

/// State required for rendering the detail panel
pub struct DetailPanelState<'a> {
    pub selected_job_id: Option<u64>,
    pub cached_jobs: &'a [crate::Job],
    pub logs: &'a [LogEvent],
    pub config: &'a Config,
    pub log_scroll_to_bottom: bool,
    pub activity_log_filters: &'a mut ActivityLogFilters,
    /// Input buffer for session continuation prompt
    pub continuation_prompt: &'a mut String,
    /// Markdown cache for rendering agent responses
    pub commonmark_cache: &'a mut egui_commonmark::CommonMarkCache,
    /// Current Claude permission mode overrides per job
    pub permission_mode_overrides: &'a HashMap<JobId, PermissionMode>,
    /// Diff content for the selected job (if available)
    pub diff_content: Option<&'a str>,
}
