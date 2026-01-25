//! Bridge communication types.
//!
//! These types mirror the TypeScript types in bridge/src/types.ts for
//! communication between KYCO and the SDK Bridge.

mod events;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{ClaudeAgentDefinition, McpServerConfig};

pub use events::{BridgeEvent, UsageStats};

/// Permission mode for Claude sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    #[default]
    Default,
    AcceptEdits,
    BypassPermissions,
    Plan,
    /// Auto-approve certain tools, escalate others to user
    Delegate,
    /// Skip approval UI for pre-configured tools
    DontAsk,
}

/// Claude SDK hook events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    Notification,
    UserPromptSubmit,
    SessionStart,
    SessionEnd,
    Stop,
    SubagentStart,
    SubagentStop,
    PreCompact,
    PermissionRequest,
}

/// Hook configuration for the bridge (emits hook events via NDJSON stream)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeHooksConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<HookEvent>>,
}

/// Claude Agent SDK plugin type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaudePluginType {
    Local,
}

/// Claude Agent SDK plugin configuration (passed through to `options.plugins`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudePlugin {
    #[serde(rename = "type")]
    pub plugin_type: ClaudePluginType,
    pub path: String,
}

/// Base64-encoded image content to attach to a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    /// Base64 data (no data URL prefix).
    pub data: String,
    /// Media type (e.g., "image/png", "image/jpeg"). Defaults to "image/png".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

/// Request to start or continue a Claude query
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeQueryRequest {
    /// The prompt to send
    pub prompt: String,
    /// Optional images to attach to the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageContent>>,
    /// Working directory for the agent
    pub cwd: String,
    /// Session ID to resume (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Fork the session instead of continuing it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fork_session: Option<bool>,
    /// Permission mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<PermissionMode>,
    /// Programmatically defined Claude subagents (Claude SDK only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents: Option<HashMap<String, ClaudeAgentDefinition>>,
    /// Allowed tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    /// Disallowed tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disallowed_tools: Option<Vec<String>>,
    /// Environment variables to pass to the SDK process
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    /// MCP servers to enable for this session (Claude SDK only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<HashMap<String, McpServerConfig>>,
    /// System prompt (append/replace depending on `system_prompt_mode`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// System prompt mode ("append" or "replace")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt_mode: Option<String>,
    /// Which Claude Code settings sources to load (e.g., ["project", "local", "user"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting_sources: Option<Vec<String>>,
    /// Plugins to load (Claude SDK only; local filesystem allowlist)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<ClaudePlugin>>,
    /// Maximum turns before stopping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    /// Maximum thinking tokens for extended thinking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_thinking_tokens: Option<u32>,
    /// Model override (sonnet, opus, haiku)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Structured output schema (JSON Schema)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    /// KYCO callback URL for tool approvals
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kyco_callback_url: Option<String>,
    /// Hook configuration (Claude SDK only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<ClaudeHooksConfig>,
}

/// Model reasoning effort level for Codex (controls how thorough Codex is)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodexEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

/// Approval policy for Codex tool use
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexApprovalPolicy {
    /// Auto-approve all tool use (default for backward compatibility)
    Never,
    /// Only ask for approval when a command fails
    OnFailure,
    /// Auto-approve known-safe operations, ask for others
    UnlessAllowListed,
    /// Require explicit approval for every tool use
    Always,
}

/// Request to start or continue a Codex thread
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexQueryRequest {
    /// The prompt to send
    pub prompt: String,
    /// Optional images to attach to the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageContent>>,
    /// Working directory
    pub cwd: String,
    /// Thread ID to resume (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    /// Sandbox mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<String>,
    /// Environment variables to pass to the Codex CLI process
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    /// Structured output schema (JSON Schema)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    /// Model to use (optional, uses Codex default if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Model reasoning effort level (controls how thorough Codex is)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<CodexEffort>,
    /// Approval policy for tool use (default: Never for backward compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<CodexApprovalPolicy>,
    /// Skip the git repository check (for temp directories, non-git projects)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_git_repo_check: Option<bool>,
}

/// Tool approval response from KYCO
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolApprovalResponse {
    pub request_id: String,
    pub decision: ToolDecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_input: Option<serde_json::Value>,
}

/// Tool approval request from the bridge (pending user decision).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolApprovalRequest {
    pub request_id: String,
    pub session_id: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
}

/// Tool approval decision
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolDecision {
    Allow,
    Deny,
    Ask,
}

/// Stored session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredSession {
    pub id: String,
    #[serde(rename = "type")]
    pub session_type: String,
    pub created_at: u64,
    pub last_active_at: u64,
    pub cwd: String,
    pub turn_count: u32,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
}

/// Health check response
#[derive(Debug, Clone, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: u64,
}

/// Status response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub active_sessions: ActiveSessionCounts,
}

/// Active session counts by type
#[derive(Debug, Clone, Deserialize)]
pub struct ActiveSessionCounts {
    pub claude: usize,
    pub codex: usize,
}
