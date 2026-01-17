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

    /// Get effective *fresh* input tokens (non-cached).
    ///
    /// Codex reports `input_tokens` (total) + `cached_input_tokens` (cache read). For cost and
    /// token breakdowns, we want the uncached remainder.
    pub fn effective_fresh_input(&self) -> u64 {
        self.input_tokens
            .saturating_sub(self.cached_input_tokens.unwrap_or(0))
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

    /// Reasoning/thinking from the model (Codex)
    Reasoning {
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
        /// SDK Structured Output (validated JSON from json_schema outputFormat)
        #[serde(rename = "structuredOutput")]
        structured_output: Option<serde_json::Value>,
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

#[cfg(test)]
mod tests {
    use super::UsageStats;

    #[test]
    fn usage_stats_fresh_input_from_codex_cached_tokens() {
        let usage: UsageStats = serde_json::from_str(
            r#"{"inputTokens":24763,"outputTokens":122,"cachedInputTokens":24448}"#,
        )
        .expect("parse usage");

        assert_eq!(usage.effective_cache_read(), 24_448);
        assert_eq!(usage.effective_fresh_input(), 315);
    }

    #[test]
    fn usage_stats_fresh_input_passthrough_for_claude_style() {
        let usage: UsageStats = serde_json::from_str(
            r#"{"inputTokens":1000,"outputTokens":200,"cacheReadTokens":300,"cacheWriteTokens":50}"#,
        )
        .expect("parse usage");

        assert_eq!(usage.effective_cache_read(), 300);
        assert_eq!(usage.effective_fresh_input(), 1000);
    }
}
