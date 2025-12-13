//! Agent configuration types

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{ClaudeAgentDefinition, McpServerConfig, SdkType, SessionMode, SystemPromptMode};

/// Agent configuration in TOML format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigToml {
    /// Short aliases for this agent (e.g., ["c", "cl"] for claude)
    #[serde(default)]
    pub aliases: Vec<String>,
    /// SDK type (claude or codex)
    ///
    /// Legacy config key: `cli_type`
    #[serde(default, alias = "cli_type", alias = "sdk_type")]
    pub sdk: SdkType,
    /// Session mode ("oneshot" or "session")
    ///
    /// Legacy config key: `mode`
    #[serde(default, alias = "mode")]
    pub session_mode: SessionMode,

    // Legacy CLI fields (ignored for SDK-based agents)
    /// Binary to execute (legacy)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,
    /// Arguments for print/non-interactive mode (legacy)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub print_mode_args: Vec<String>,
    /// Arguments for output format (legacy)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_format_args: Vec<String>,
    /// Arguments for REPL/interactive mode (legacy)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repl_mode_args: Vec<String>,
    /// Legacy default args (prefer print_mode_args + output_format_args)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_args: Vec<String>,
    #[serde(default)]
    pub system_prompt_mode: SystemPromptMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disallowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    /// MCP servers to enable for this agent (Claude SDK only)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub mcp_servers: HashMap<String, McpServerConfig>,

    /// Programmatically defined Claude subagents (Claude SDK only)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub agents: HashMap<String, ClaudeAgentDefinition>,
}
