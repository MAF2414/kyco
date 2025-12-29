//! Bridge event types for streaming responses.

use serde::Deserialize;

/// Token usage statistics
///
/// Supports both Claude format (cache_read/write_tokens) and
/// Codex format (cached_input_tokens).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Claude: cache read tokens
    #[serde(alias = "cacheReadTokens")]
    pub cache_read_tokens: Option<u64>,
    /// Claude: cache write tokens
    #[serde(alias = "cacheWriteTokens")]
    pub cache_write_tokens: Option<u64>,
    /// Codex: cached input tokens (maps to cache_read)
    #[serde(alias = "cachedInputTokens")]
    pub cached_input_tokens: Option<u64>,
}

impl UsageStats {
    /// Get effective cache read tokens (Claude or Codex format)
    pub fn effective_cache_read(&self) -> u64 {
        self.cache_read_tokens.or(self.cached_input_tokens).unwrap_or(0)
    }
}

/// Bridge event - union of all possible events
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeEvent {
    /// Session started
    #[serde(rename = "session.start")]
    SessionStart {
        #[serde(rename = "sessionId")]
        session_id: String,
        timestamp: u64,
        model: String,
        tools: Vec<String>,
    },

    /// Text output from the assistant
    Text {
        #[serde(rename = "sessionId")]
        session_id: String,
        timestamp: u64,
        content: String,
        partial: bool,
    },

    /// Tool use started
    #[serde(rename = "tool.use")]
    ToolUse {
        #[serde(rename = "sessionId")]
        session_id: String,
        timestamp: u64,
        #[serde(rename = "toolName")]
        tool_name: String,
        #[serde(rename = "toolInput")]
        tool_input: serde_json::Value,
        #[serde(rename = "toolUseId")]
        tool_use_id: String,
    },

    /// Tool result
    #[serde(rename = "tool.result")]
    ToolResult {
        #[serde(rename = "sessionId")]
        session_id: String,
        timestamp: u64,
        #[serde(rename = "toolUseId")]
        tool_use_id: String,
        success: bool,
        output: String,
        #[serde(rename = "filesChanged")]
        files_changed: Option<Vec<String>>,
    },

    /// Error occurred
    Error {
        #[serde(rename = "sessionId")]
        session_id: String,
        timestamp: u64,
        message: String,
        code: Option<String>,
    },

    /// Session completed
    #[serde(rename = "session.complete")]
    SessionComplete {
        #[serde(rename = "sessionId")]
        session_id: String,
        timestamp: u64,
        success: bool,
        result: Option<serde_json::Value>,
        usage: Option<UsageStats>,
        #[serde(rename = "costUsd")]
        cost_usd: Option<f64>,
        #[serde(rename = "durationMs")]
        duration_ms: u64,
    },

    /// Tool approval needed
    #[serde(rename = "tool.approval_needed")]
    ToolApprovalNeeded {
        #[serde(rename = "sessionId")]
        session_id: String,
        timestamp: u64,
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
        #[serde(rename = "toolInput")]
        tool_input: serde_json::Value,
    },

    /// Hook fired by the Claude SDK before executing a tool
    #[serde(rename = "hook.pre_tool_use")]
    HookPreToolUse {
        #[serde(rename = "sessionId")]
        session_id: String,
        timestamp: u64,
        #[serde(rename = "toolName")]
        tool_name: String,
        #[serde(rename = "toolInput")]
        tool_input: serde_json::Value,
        #[serde(rename = "toolUseId")]
        tool_use_id: String,
        #[serde(rename = "transcriptPath")]
        transcript_path: Option<String>,
    },

    /// Heartbeat event to keep HTTP connection alive during tool approval waits
    Heartbeat {
        #[serde(rename = "sessionId")]
        session_id: String,
        timestamp: u64,
        #[serde(rename = "pendingApprovalRequestId")]
        pending_approval_request_id: Option<String>,
    },
}
