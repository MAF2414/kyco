//! GUI-specific settings

use serde::{Deserialize, Serialize};

use super::orchestrator::OrchestratorSettings;
use super::voice::VoiceSettings;

/// GUI-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSettings {
    /// Global hotkey to trigger the popup
    /// Format: "modifier+modifier+key" (e.g., "cmd+shift+k", "ctrl+shift+space")
    /// Modifiers: cmd/super, ctrl, alt, shift
    /// Default: "cmd+shift+k" on macOS, "ctrl+shift+k" on Windows/Linux
    #[serde(default = "default_gui_hotkey")]
    pub hotkey: String,

    /// Default agent for GUI tasks
    #[serde(default = "default_gui_agent")]
    pub default_agent: String,

    /// Default mode for GUI tasks
    #[serde(default = "default_gui_mode")]
    pub default_mode: String,

    /// Output schema that agents should include in their response
    /// This YAML block helps structure the output for better GUI display
    #[serde(default = "default_output_schema")]
    pub output_schema: String,

    /// Optional JSON Schema used for SDK structured output.
    ///
    /// When set, Claude and Codex will be asked to produce JSON matching this schema.
    /// Leave empty to keep the YAML summary footer behavior.
    #[serde(default)]
    pub structured_output_schema: String,

    /// Local HTTP server port for IDE extensions
    /// Default: 9876
    #[serde(default = "default_gui_http_port")]
    pub http_port: u16,

    /// Shared secret required for IDE extension requests (sent as `X-KYCO-Token`)
    ///
    /// If empty, the server will accept unauthenticated requests (not recommended).
    #[serde(default)]
    pub http_token: String,

    /// Voice input settings
    #[serde(default)]
    pub voice: VoiceSettings,

    /// Orchestrator settings for external CLI sessions
    #[serde(default)]
    pub orchestrator: OrchestratorSettings,
}

fn default_gui_hotkey() -> String {
    #[cfg(target_os = "macos")]
    return "cmd+option+k".to_string();
    #[cfg(not(target_os = "macos"))]
    return "ctrl+alt+k".to_string();
}

fn default_gui_agent() -> String {
    "claude".to_string()
}

fn default_gui_mode() -> String {
    "implement".to_string()
}

fn default_output_schema() -> String {
    r#"
IMPORTANT: End your response with a structured YAML summary block:
---
title: Short task title (max 60 chars)
commit_subject: Suggested git commit subject (max 72 chars)
commit_body: |
  Suggested git commit body (optional, can be multiline)
details: What was done (2-3 sentences)
status: success|partial|failed
summary: |
  Detailed summary of findings and actions (optional, can be multiline).
  This is passed to the next agent in a chain for context.
state: <state_identifier>
---

STATE VALUES: issues_found, no_issues, fixed, unfixable, tests_pass, tests_fail, implemented, blocked, refactored, documented
"#
    .to_string()
}

fn default_gui_http_port() -> u16 {
    9876
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            hotkey: default_gui_hotkey(),
            default_agent: default_gui_agent(),
            default_mode: default_gui_mode(),
            output_schema: default_output_schema(),
            structured_output_schema: String::new(),
            http_port: default_gui_http_port(),
            http_token: String::new(),
            voice: VoiceSettings::default(),
            orchestrator: OrchestratorSettings::default(),
        }
    }
}
