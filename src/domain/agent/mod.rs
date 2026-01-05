//! Agent configuration types.
//!
//! KYCo supports running agents via an SDK Bridge (preferred) and via CLI adapters
//! (fallback). This module defines the shared configuration surface used by both.

mod templates;
mod types;

pub use types::{CliType, SdkType, SystemPromptMode};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP (Model Context Protocol) Server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpServerConfig {
    /// Command to run the MCP server (e.g., "npx", "node", path to binary)
    pub command: String,
    /// Arguments to pass to the command
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables for the MCP server
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Optional working directory
    pub cwd: Option<String>,
}

/// Definition for a Claude subagent that can be invoked via the Task tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeAgentDefinition {
    /// Natural language description of when to use this agent
    pub description: String,

    /// The agent's system prompt
    pub prompt: String,

    /// Array of allowed tool names. If omitted, inherits all tools from parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,

    /// Array of tool names to explicitly disallow for this agent
    #[serde(
        default,
        rename = "disallowedTools",
        alias = "disallowedTools",
        alias = "disallowed_tools",
        skip_serializing_if = "Option::is_none"
    )]
    pub disallowed_tools: Option<Vec<String>>,

    /// Model alias (e.g., "sonnet", "opus", "haiku", "inherit")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Experimental: Critical reminder added to the system prompt
    #[serde(
        default,
        rename = "criticalSystemReminder_EXPERIMENTAL",
        alias = "criticalSystemReminder_EXPERIMENTAL",
        alias = "critical_system_reminder_experimental",
        skip_serializing_if = "Option::is_none"
    )]
    pub critical_system_reminder_experimental: Option<String>,
}

/// Configuration for a specific mode's agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeTemplate {
    /// The prompt template for this mode
    pub prompt_template: String,

    /// System prompt additions for this mode
    pub system_prompt: Option<String>,

    /// Default agent for this mode (if not specified in command)
    pub default_agent: Option<String>,

    /// Tools to disallow for this mode
    #[serde(default)]
    pub disallowed_tools: Vec<String>,

    /// Tools to explicitly allow for this mode
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Possible output states this mode can produce (for chain triggers)
    #[serde(default)]
    pub output_states: Vec<String>,

    /// Custom prompt for state output instructions
    /// If set, used instead of auto-generating from output_states
    #[serde(default)]
    pub state_prompt: Option<String>,
}

/// Configuration for an SDK-based agent (Claude or Codex)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique identifier (e.g., "claude", "codex")
    pub id: String,

    /// The SDK type (Claude or Codex)
    #[serde(default)]
    pub sdk_type: SdkType,

    /// Permission mode (e.g., "bypassPermissions" for Claude, "full-auto" for Codex)
    #[serde(default)]
    pub permission_mode: String,

    /// Optional model override (primarily for Claude)
    #[serde(default)]
    pub model: Option<String>,

    /// Sandbox mode (primarily for Codex)
    #[serde(default)]
    pub sandbox: Option<String>,

    /// Maximum number of turns for the agent (0 = unlimited)
    #[serde(default)]
    pub max_turns: u32,

    /// How to handle system prompts
    #[serde(default)]
    pub system_prompt_mode: SystemPromptMode,

    /// Mode-specific templates
    #[serde(default)]
    pub mode_templates: HashMap<String, ModeTemplate>,

    /// Environment variables to pass to the agent
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Tools to disallow (e.g., ["Bash(git commit)", "Bash(git push)"])
    #[serde(default)]
    pub disallowed_tools: Vec<String>,

    /// Tools to explicitly allow (if empty, all tools are allowed)
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// MCP servers to enable for this agent
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,

    /// Programmatically defined Claude subagents (Claude SDK only)
    #[serde(default)]
    pub agents: HashMap<String, ClaudeAgentDefinition>,

    /// Claude Agent SDK plugins to load (local filesystem paths).
    ///
    /// These paths come from `settings.claude.allowed_plugin_paths` and are always treated as an
    /// allowlist. Plugins are executed as Node.js code inside the bridge process.
    #[serde(default)]
    pub plugins: Vec<String>,

    /// Output schema to append to system prompt (for structured GUI output)
    #[serde(default)]
    pub output_schema: Option<String>,

    /// Optional JSON Schema for true SDK structured output.
    ///
    /// When set, the bridge will request JSON output that conforms to this schema.
    #[serde(default)]
    pub structured_output_schema: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self::claude_default()
    }
}

impl AgentConfig {
    /// Create a default Claude SDK agent configuration
    pub fn claude_default() -> Self {
        let sdk_type = SdkType::Claude;
        Self {
            id: "claude".to_string(),
            sdk_type,
            permission_mode: sdk_type.default_permission_mode().to_string(),
            model: None,
            sandbox: None,
            max_turns: 0,
            system_prompt_mode: SystemPromptMode::Append,
            mode_templates: templates::default_mode_templates(),
            env: HashMap::new(),
            disallowed_tools: vec![],
            allowed_tools: Vec::new(),
            mcp_servers: HashMap::new(),
            agents: HashMap::new(),
            plugins: Vec::new(),
            output_schema: None,
            structured_output_schema: None,
        }
    }

    /// Create a default Codex SDK agent configuration
    pub fn codex_default() -> Self {
        let sdk_type = SdkType::Codex;
        Self {
            id: "codex".to_string(),
            sdk_type,
            permission_mode: sdk_type.default_permission_mode().to_string(),
            model: None,
            sandbox: None,
            max_turns: 0,
            system_prompt_mode: SystemPromptMode::Append,
            mode_templates: templates::default_mode_templates(),
            env: HashMap::new(),
            disallowed_tools: vec![],
            allowed_tools: Vec::new(),
            mcp_servers: HashMap::new(),
            agents: HashMap::new(),
            plugins: Vec::new(),
            output_schema: None,
            structured_output_schema: None,
        }
    }

    /// Get the mode template for a given mode, falling back to a generic template
    pub fn get_mode_template(&self, mode: &str) -> ModeTemplate {
        self.mode_templates
            .get(mode)
            .cloned()
            .unwrap_or_else(|| ModeTemplate {
                prompt_template:
                    "Execute '{mode}' on {scope_type} `{target}` in `{file}`. {description}"
                        .to_string(),
                system_prompt: Some(format!(
                    "You are running in KYCo '{mode}' mode. You may read the entire repo. \
                     Make changes only within the marked scope.",
                    mode = mode
                )),
                default_agent: None,
                disallowed_tools: vec![],
                allowed_tools: vec![],
                output_states: vec![],
                state_prompt: None,
            })
    }

    /// Get the binary name for CLI-based adapters (fallback to SDK type name)
    pub fn get_binary(&self) -> String {
        self.sdk_type.default_name().to_string()
    }

    /// Get run args for CLI-based adapters (returns empty for SDK-based agents)
    pub fn get_run_args(&self) -> Vec<String> {
        Vec::new()
    }

    /// Get repl args for CLI-based adapters (returns empty for SDK-based agents)
    pub fn get_repl_args(&self) -> Vec<String> {
        Vec::new()
    }
}
