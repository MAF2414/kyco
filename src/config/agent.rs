//! Agent configuration types

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{ClaudeAgentDefinition, McpServerConfig, SdkType, SystemPromptMode};

/// Agent configuration in TOML format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigToml {
    /// Version number for versioned merging (internal configs only)
    /// Higher versions will override user customizations
    #[serde(default)]
    pub version: u32,
    /// Short aliases for this agent (e.g., ["c", "cl"] for claude)
    #[serde(default)]
    pub aliases: Vec<String>,
    /// SDK type (claude or codex)
    ///
    /// Legacy config key: `cli_type`
    #[serde(default, alias = "cli_type", alias = "sdk_type")]
    pub sdk: SdkType,
    /// Model to use (e.g., "sonnet", "opus", "haiku" for Claude; "o3", "gpt-4o" for Codex)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Permission mode / approval preset.
    ///
    /// - Claude CLI: `--permission-mode` (e.g., `acceptEdits`, `bypassPermissions`, `plan`)
    /// - Codex CLI: unused (use `ask_for_approval` + `sandbox` instead)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,

    /// Codex CLI sandbox mode (e.g., `workspace-write`, `read-only`, `danger-full-access`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<String>,

    /// Codex CLI approvals policy (`--ask-for-approval`).
    ///
    /// Values: `untrusted`, `on-failure`, `on-request`, `never`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_for_approval: Option<String>,

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

    // Token pricing (per 1M tokens in USD) for cost estimation
    /// Input token price per 1M tokens (e.g., 3.0 for $3.00/1M)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_input: Option<f64>,
    /// Cached input token price per 1M tokens (e.g., 0.3 for $0.30/1M)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_cached_input: Option<f64>,
    /// Output token price per 1M tokens (e.g., 15.0 for $15.00/1M)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_output: Option<f64>,

    /// Whether this agent is allowed to bypass sandbox/permission restrictions.
    ///
    /// When true, enables:
    /// - Claude: `--dangerously-skip-permissions`
    /// - Codex: `--dangerously-bypass-approvals-and-sandbox` (--yolo)
    ///
    /// Default is false for safety.
    #[serde(default)]
    pub allow_dangerous_bypass: bool,
}
