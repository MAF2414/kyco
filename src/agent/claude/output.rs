//! Claude Code agent output stream parsing

use serde::{Deserialize, Serialize};

/// Events from the Claude stream-json output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// System message
    System {
        subtype: String,
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        session_id: Option<String>,
    },

    /// Assistant message (text or tool use)
    Assistant {
        #[serde(default)]
        message: AssistantMessage,
    },

    /// User message (usually tool results)
    User {
        #[serde(default)]
        message: UserMessage,
    },

    /// Result message
    Result {
        subtype: String,
        #[serde(default)]
        result: Option<String>,
        #[serde(default, alias = "total_cost_usd")]
        cost_usd: Option<f64>,
        #[serde(default)]
        duration_ms: Option<u64>,
        #[serde(default)]
        duration_api_ms: Option<u64>,
        #[serde(default)]
        session_id: Option<String>,
    },
}

/// Assistant message content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssistantMessage {
    #[serde(default)]
    pub content: Vec<ContentBlock>,
}

/// User message content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserMessage {
    #[serde(default)]
    pub content: Vec<ContentBlock>,
}

/// Content block (text or tool use)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text { text: String },

    /// Tool use request
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    /// Tool result
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
}

impl StreamEvent {
    /// Parse a JSON line into a stream event
    pub fn parse(line: &str) -> Option<Self> {
        serde_json::from_str(line).ok()
    }

    /// Extract a human-readable summary from this event
    pub fn summary(&self) -> String {
        match self {
            StreamEvent::System {
                subtype, message, ..
            } => {
                format!("[system:{}] {}", subtype, message.as_deref().unwrap_or(""))
            }
            StreamEvent::Assistant { message } => {
                let mut parts = Vec::new();
                for block in &message.content {
                    match block {
                        ContentBlock::Text { text } => {
                            parts.push(format!("[text] {}", text));
                        }
                        ContentBlock::ToolUse { name, .. } => {
                            parts.push(format!("[tool] {}", name));
                        }
                        _ => {}
                    }
                }
                parts.join(" | ")
            }
            StreamEvent::User { message } => {
                let mut parts = Vec::new();
                for block in &message.content {
                    if let ContentBlock::ToolResult {
                        content, is_error, ..
                    } = block
                    {
                        let prefix = if *is_error { "error" } else { "result" };
                        parts.push(format!("[{}] {}", prefix, content));
                    }
                }
                parts.join(" | ")
            }
            StreamEvent::Result {
                subtype,
                cost_usd,
                duration_ms,
                ..
            } => {
                let cost = cost_usd.map(|c| format!("${:.4}", c)).unwrap_or_default();
                let duration = duration_ms.map(|d| format!("{}ms", d)).unwrap_or_default();
                format!("[result:{}] {} {}", subtype, cost, duration)
            }
        }
    }
}
