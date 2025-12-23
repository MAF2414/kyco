//! Agent configuration types.
//!
//! KYCo supports running agents via an SDK Bridge (preferred) and via CLI adapters
//! (fallback). This module defines the shared configuration surface used by both.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The type of SDK being used
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SdkType {
    /// Claude Agent SDK (Anthropic)
    #[default]
    Claude,
    /// Codex SDK (OpenAI)
    Codex,
    /// Legacy: Gemini (not supported, will use Claude)
    Gemini,
    /// Legacy: Custom (not supported, will use Claude)
    Custom,
}

impl SdkType {
    /// Returns the default agent name for this SDK type.
    pub fn default_name(&self) -> &'static str {
        match self {
            SdkType::Claude => "claude",
            SdkType::Codex => "codex",
            // Legacy: map to Claude
            SdkType::Gemini | SdkType::Custom => "claude",
        }
    }

    /// Returns the default binary for this SDK type.
    pub fn default_binary(&self) -> &'static str {
        self.default_name()
    }

    /// Returns the permission mode options for this SDK type.
    pub fn permission_modes(&self) -> &'static [&'static str] {
        match self {
            SdkType::Claude | SdkType::Gemini | SdkType::Custom => {
                &["default", "acceptEdits", "bypassPermissions", "plan"]
            }
            SdkType::Codex => &["suggest", "auto-edit", "full-auto"],
        }
    }

    /// Returns the default permission mode for this SDK type.
    pub fn default_permission_mode(&self) -> &'static str {
        match self {
            SdkType::Claude | SdkType::Gemini | SdkType::Custom => "default",
            SdkType::Codex => "suggest",
        }
    }
}

// Keep CliType as an alias for backwards compatibility during transition
pub type CliType = SdkType;

/// How to handle the system prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemPromptMode {
    /// Append to the default system prompt
    #[default]
    Append,
    /// Replace the default system prompt entirely
    Replace,
    /// Legacy: Use config override (for CLI-based agents)
    #[serde(rename = "configoverride")]
    ConfigOverride,
}

/// Agent execution mode - determines if conversation is continued or one-shot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionMode {
    /// One-shot execution - no session persistence
    #[default]
    Oneshot,
    /// Session mode - conversation can be resumed/continued
    Session,
    /// Legacy: Print mode (equivalent to Oneshot)
    Print,
    /// Legacy: REPL mode (equivalent to Session)
    Repl,
}

impl SessionMode {
    /// Returns true if this is a session-based mode (Session or Repl)
    pub fn is_session(&self) -> bool {
        matches!(self, SessionMode::Session | SessionMode::Repl)
    }

    /// Returns true if this is a one-shot mode (Oneshot or Print)
    pub fn is_oneshot(&self) -> bool {
        matches!(self, SessionMode::Oneshot | SessionMode::Print)
    }
}

// Keep AgentMode as alias for backwards compatibility
pub type AgentMode = SessionMode;

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

    /// Session mode for this mode (oneshot or session)
    #[serde(default)]
    pub session_mode: SessionMode,

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

    /// Session mode (oneshot or session) - default for this agent
    #[serde(default)]
    pub session_mode: SessionMode,

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

    // Legacy fields for backwards compatibility - will be ignored
    #[serde(default, skip_serializing)]
    pub cli_type: Option<SdkType>,
    #[serde(default, skip_serializing)]
    pub mode: Option<SessionMode>,
    #[serde(default, skip_serializing)]
    pub binary: Option<String>,
    #[serde(default, skip_serializing)]
    pub print_mode_args: Vec<String>,
    #[serde(default, skip_serializing)]
    pub output_format_args: Vec<String>,
    #[serde(default, skip_serializing)]
    pub repl_mode_args: Vec<String>,
    #[serde(default, skip_serializing)]
    pub default_args: Vec<String>,
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
            session_mode: SessionMode::Oneshot,
            permission_mode: sdk_type.default_permission_mode().to_string(),
            model: None,
            sandbox: None,
            max_turns: 0,
            system_prompt_mode: SystemPromptMode::Append,
            mode_templates: Self::default_mode_templates(),
            env: HashMap::new(),
            disallowed_tools: vec![],
            allowed_tools: Vec::new(),
            mcp_servers: HashMap::new(),
            agents: HashMap::new(),
            plugins: Vec::new(),
            output_schema: None,
            structured_output_schema: None,
            // Legacy fields
            cli_type: None,
            mode: None,
            binary: None,
            print_mode_args: vec![],
            output_format_args: vec![],
            repl_mode_args: vec![],
            default_args: vec![],
        }
    }

    /// Create a default Codex SDK agent configuration
    pub fn codex_default() -> Self {
        let sdk_type = SdkType::Codex;
        Self {
            id: "codex".to_string(),
            sdk_type,
            session_mode: SessionMode::Oneshot,
            permission_mode: sdk_type.default_permission_mode().to_string(),
            model: None,
            sandbox: None,
            max_turns: 0,
            system_prompt_mode: SystemPromptMode::Append,
            mode_templates: Self::default_mode_templates(),
            env: HashMap::new(),
            disallowed_tools: vec![],
            allowed_tools: Vec::new(),
            mcp_servers: HashMap::new(),
            agents: HashMap::new(),
            plugins: Vec::new(),
            output_schema: None,
            structured_output_schema: None,
            // Legacy fields
            cli_type: None,
            mode: None,
            binary: None,
            print_mode_args: vec![],
            output_format_args: vec![],
            repl_mode_args: vec![],
            default_args: vec![],
        }
    }

    /// Default mode templates
    fn default_mode_templates() -> HashMap<String, ModeTemplate> {
        let mut templates = HashMap::new();

        templates.insert(
            "refactor".to_string(),
            ModeTemplate {
                prompt_template: "Refactor the {scope_type} `{target}` in `{file}`. {description}"
                    .to_string(),
                system_prompt: Some(
                    "You are running in KYCo 'refactor' mode. You may read the entire repo. \
                     Make code changes only within the marked scope. Write idiomatic code. \
                     Do not change function signatures unless explicitly requested."
                        .to_string(),
                ),
                default_agent: None,
                session_mode: SessionMode::Oneshot,
                disallowed_tools: vec![],
                allowed_tools: vec![],
                output_states: vec![],
                state_prompt: None,
            },
        );

        templates.insert(
            "fix".to_string(),
            ModeTemplate {
                prompt_template:
                    "Fix the issue in {scope_type} `{target}` in `{file}`. {description}"
                        .to_string(),
                system_prompt: Some(
                    "You are running in KYCo 'fix' mode. You may read the entire repo. \
                     Analyze the code and fix the described issue. Make minimal changes necessary."
                        .to_string(),
                ),
                default_agent: None,
                session_mode: SessionMode::Oneshot,
                disallowed_tools: vec![],
                allowed_tools: vec![],
                output_states: vec![],
                state_prompt: None,
            },
        );

        templates.insert(
            "tests".to_string(),
            ModeTemplate {
                prompt_template:
                    "Write unit tests for {scope_type} `{target}` in `{file}`. {description}"
                        .to_string(),
                system_prompt: Some(
                    "You are running in KYCo 'tests' mode. You may read the entire repo. \
                     Write comprehensive unit tests. Use the existing test framework and patterns \
                     found in the codebase."
                        .to_string(),
                ),
                default_agent: None,
                session_mode: SessionMode::Oneshot,
                disallowed_tools: vec![],
                allowed_tools: vec![],
                output_states: vec![],
                state_prompt: None,
            },
        );

        templates.insert(
            "docs".to_string(),
            ModeTemplate {
                prompt_template:
                    "Write documentation for {scope_type} `{target}` in `{file}`. {description}"
                        .to_string(),
                system_prompt: Some(
                    "You are running in KYCo 'docs' mode. You may read the entire repo. \
                     Write clear, concise documentation. Follow existing documentation patterns \
                     in the codebase."
                        .to_string(),
                ),
                default_agent: None,
                session_mode: SessionMode::Oneshot,
                disallowed_tools: vec![],
                allowed_tools: vec![],
                output_states: vec![],
                state_prompt: None,
            },
        );

        templates.insert(
            "review".to_string(),
            ModeTemplate {
                prompt_template: "Review {scope_type} `{target}` in `{file}`. {description}"
                    .to_string(),
                system_prompt: Some(
                    "You are running in KYCo 'review' mode. You may read the entire repo. \
                     Analyze the code for bugs, performance issues, and code quality. \
                     Suggest improvements but do not make changes unless explicitly asked."
                        .to_string(),
                ),
                default_agent: None,
                session_mode: SessionMode::Oneshot,
                disallowed_tools: vec![],
                allowed_tools: vec![],
                output_states: vec![],
                state_prompt: None,
            },
        );

        templates.insert(
            "chat".to_string(),
            ModeTemplate {
                prompt_template: "{description}".to_string(),
                system_prompt: Some(
                    "You are running in KYCo 'chat' mode. This is a conversational session. \
                     You can continue the conversation and remember previous context."
                        .to_string(),
                ),
                default_agent: None,
                session_mode: SessionMode::Session, // Chat mode uses sessions by default
                disallowed_tools: vec![],
                allowed_tools: vec![],
                output_states: vec![],
                state_prompt: None,
            },
        );

        templates
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
                session_mode: self.session_mode,
                disallowed_tools: vec![],
                allowed_tools: vec![],
                output_states: vec![],
                state_prompt: None,
            })
    }

    /// Get the effective session mode for a given mode
    pub fn get_session_mode(&self, mode: &str) -> SessionMode {
        self.mode_templates
            .get(mode)
            .map(|t| t.session_mode)
            .unwrap_or(self.session_mode)
    }

    // Legacy compatibility methods - these will be deprecated

    /// Legacy: Get the binary name (for CLI-based adapters)
    pub fn get_binary(&self) -> String {
        self.binary
            .clone()
            .unwrap_or_else(|| self.sdk_type.default_name().to_string())
    }

    /// Legacy: Get run args for CLI-based adapters.
    ///
    /// This combines `print_mode_args` and `output_format_args`, falling back to
    /// `default_args` if both are empty.
    pub fn get_run_args(&self) -> Vec<String> {
        if self.print_mode_args.is_empty() && self.output_format_args.is_empty() {
            return self.default_args.clone();
        }

        let mut args = self.print_mode_args.clone();
        args.extend(self.output_format_args.clone());
        args
    }

    /// Legacy: Get repl args (returns empty for SDK agents)
    pub fn get_repl_args(&self) -> Vec<String> {
        self.repl_mode_args.clone()
    }

    /// Legacy: Get output format args
    pub fn get_output_format_args(&self) -> Vec<String> {
        self.output_format_args.clone()
    }
}
