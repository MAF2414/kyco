//! Mode configuration types

use serde::{Deserialize, Serialize};

/// Claude-specific mode options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeModeOptions {
    /// Permission mode for Claude SDK
    /// - "default": Normal permission checks (asks for everything)
    /// - "acceptEdits": Auto-accepts file edits, asks for Bash
    /// - "bypassPermissions": Full auto, no questions (dangerous!)
    /// - "plan": Planning mode (no execution)
    #[serde(default = "default_claude_permission")]
    pub permission_mode: String,
}

impl Default for ClaudeModeOptions {
    fn default() -> Self {
        Self {
            permission_mode: default_claude_permission(),
        }
    }
}

fn default_claude_permission() -> String {
    "acceptEdits".to_string()
}

/// Codex-specific mode options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexModeOptions {
    /// Sandbox mode for Codex SDK
    /// - "read-only": No file modifications
    /// - "workspace-write": Can modify files in workspace (safe default)
    /// - "danger-full-access": Full access including network
    #[serde(default = "default_codex_sandbox")]
    pub sandbox: String,
}

impl Default for CodexModeOptions {
    fn default() -> Self {
        Self {
            sandbox: default_codex_sandbox(),
        }
    }
}

fn default_codex_sandbox() -> String {
    "workspace-write".to_string()
}

/// Session mode for the agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModeSessionType {
    /// One-shot execution - no session persistence
    #[default]
    Oneshot,
    /// Session mode - conversation can be resumed/continued
    Session,
}

/// Mode configuration - the prompt builder
///
/// Modes define HOW to instruct the agent. They combine:
/// - A prompt template with placeholders
/// - A system prompt for context
/// - Session and permission settings
/// - Tool restrictions (blacklist)
///
/// Template placeholders:
/// - {target} - what to process (from target config)
/// - {scope} - the scope description
/// - {file} - the source file path
/// - {description} - user's description from comment
/// - {mode} - the mode name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    /// Version number for versioned merging (internal configs only)
    /// Higher versions will override user customizations
    #[serde(default)]
    pub version: u32,
    /// Default agent for this mode (can be overridden in marker)
    #[serde(default)]
    pub agent: Option<String>,

    /// Default target for this mode
    #[serde(default)]
    pub target_default: Option<String>,

    /// Default scope for this mode
    #[serde(default)]
    pub scope_default: Option<String>,

    /// The prompt template - the core instruction
    /// Placeholders: {target}, {scope}, {file}, {description}, {mode}
    pub prompt: Option<String>,

    /// System prompt addition for agent context
    pub system_prompt: Option<String>,

    /// Session mode: oneshot (default) or session (persistent conversation)
    #[serde(default)]
    pub session_mode: ModeSessionType,

    /// Maximum turns/iterations for the agent (0 = unlimited)
    #[serde(default)]
    pub max_turns: u32,

    /// Optional model override for this mode (e.g., "sonnet", "opus", "haiku")
    #[serde(default)]
    pub model: Option<String>,

    /// Tools to disallow for this mode (blacklist)
    /// Examples: ["Write", "Edit", "Bash", "Bash(git push)"]
    #[serde(default)]
    pub disallowed_tools: Vec<String>,

    /// Claude SDK specific options
    #[serde(default)]
    pub claude: Option<ClaudeModeOptions>,

    /// Codex SDK specific options
    #[serde(default)]
    pub codex: Option<CodexModeOptions>,

    /// Short aliases for this mode (e.g., ["r", "rev"] for review)
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Possible output states this mode can produce (for chain triggers)
    /// Example: ["issues_found", "no_issues"] for review mode
    #[serde(default)]
    pub output_states: Vec<String>,

    /// Custom prompt for state output instructions (appended to system prompt)
    /// If not set but output_states is defined, auto-generates instructions
    /// Example: "Set state to 'issues_found' if you find problems, 'no_issues' otherwise."
    #[serde(default)]
    pub state_prompt: Option<String>,

    /// Legacy: allowed_tools (deprecated, use disallowed_tools instead)
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Force running in a git worktree for this mode
    /// - None: Use global settings (default)
    /// - Some(true): Always run in worktree
    /// - Some(false): Never run in worktree (overrides global)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_worktree: Option<bool>,
}

impl ModeConfig {
    /// Get the effective Claude permission mode
    /// Returns the configured value or derives from disallowed_tools
    pub fn get_claude_permission(&self) -> String {
        if let Some(ref claude) = self.claude {
            return claude.permission_mode.clone();
        }

        let blocks_writes = self
            .disallowed_tools
            .iter()
            .any(|t| t == "Write" || t == "Edit");

        if blocks_writes {
            "default".to_string()
        } else {
            "acceptEdits".to_string()
        }
    }

    /// Get the effective Codex sandbox mode
    /// Returns the configured value or derives from disallowed_tools
    pub fn get_codex_sandbox(&self) -> String {
        if let Some(ref codex) = self.codex {
            return codex.sandbox.clone();
        }

        let blocks_writes = self
            .disallowed_tools
            .iter()
            .any(|t| t == "Write" || t == "Edit");
        let blocks_bash = self.disallowed_tools.iter().any(|t| t == "Bash");

        if blocks_writes || blocks_bash {
            "read-only".to_string()
        } else {
            "workspace-write".to_string()
        }
    }
}
