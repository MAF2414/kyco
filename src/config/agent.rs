//! Agent configuration types

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{AgentMode, CliType, SystemPromptMode};

/// Agent configuration in TOML format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigToml {
    /// Short aliases for this agent (e.g., ["c", "cl"] for claude)
    #[serde(default)]
    pub aliases: Vec<String>,
    /// CLI type (claude, codex, gemini, custom)
    #[serde(default)]
    pub cli_type: CliType,
    /// Execution mode (print or repl)
    #[serde(default)]
    pub mode: AgentMode,
    /// Binary to execute
    pub binary: String,
    /// Arguments for print/non-interactive mode
    #[serde(default)]
    pub print_mode_args: Vec<String>,
    /// Arguments for output format
    #[serde(default)]
    pub output_format_args: Vec<String>,
    /// Arguments for REPL/interactive mode
    #[serde(default)]
    pub repl_mode_args: Vec<String>,
    /// Legacy default args (prefer print_mode_args + output_format_args)
    #[serde(default)]
    pub default_args: Vec<String>,
    #[serde(default)]
    pub system_prompt_mode: SystemPromptMode,
    #[serde(default)]
    pub disallowed_tools: Vec<String>,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}
