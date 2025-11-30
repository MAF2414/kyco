use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The kind of log event from an agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEventKind {
    /// Agent is thinking/reasoning
    Thought,
    /// Agent is calling a tool
    ToolCall,
    /// Tool returned output
    ToolOutput,
    /// Agent produced a text response
    Text,
    /// Agent encountered an error
    Error,
    /// System message (e.g., start/stop)
    System,
}

impl std::fmt::Display for LogEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogEventKind::Thought => write!(f, "thought"),
            LogEventKind::ToolCall => write!(f, "tool"),
            LogEventKind::ToolOutput => write!(f, "output"),
            LogEventKind::Text => write!(f, "text"),
            LogEventKind::Error => write!(f, "error"),
            LogEventKind::System => write!(f, "system"),
        }
    }
}

/// A log event from agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    /// When this event occurred
    pub timestamp: DateTime<Utc>,

    /// The kind of event
    pub kind: LogEventKind,

    /// Job ID this event belongs to (None for system-wide events)
    pub job_id: Option<u64>,

    /// Short summary (e.g., "Read src/orders.rs")
    pub summary: String,

    /// Full content (may be truncated for display)
    pub content: Option<String>,

    /// Tool name if this is a tool event
    pub tool_name: Option<String>,

    /// Tool arguments if this is a tool call
    pub tool_args: Option<serde_json::Value>,
}

impl LogEvent {
    /// Create a new log event
    pub fn new(kind: LogEventKind, summary: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            kind,
            job_id: None,
            summary: summary.into(),
            content: None,
            tool_name: None,
            tool_args: None,
        }
    }

    /// Set the job ID for this event
    pub fn for_job(mut self, job_id: u64) -> Self {
        self.job_id = Some(job_id);
        self
    }

    /// Create a thought event
    pub fn thought(summary: impl Into<String>) -> Self {
        Self::new(LogEventKind::Thought, summary)
    }

    /// Create a tool call event
    pub fn tool_call(tool_name: impl Into<String>, summary: impl Into<String>) -> Self {
        let tool_name = tool_name.into();
        let mut event = Self::new(LogEventKind::ToolCall, summary);
        event.tool_name = Some(tool_name);
        event
    }

    /// Create a tool output event
    pub fn tool_output(tool_name: impl Into<String>, summary: impl Into<String>) -> Self {
        let tool_name = tool_name.into();
        let mut event = Self::new(LogEventKind::ToolOutput, summary);
        event.tool_name = Some(tool_name);
        event
    }

    /// Create a text event
    pub fn text(summary: impl Into<String>) -> Self {
        Self::new(LogEventKind::Text, summary)
    }

    /// Create an error event
    pub fn error(summary: impl Into<String>) -> Self {
        Self::new(LogEventKind::Error, summary)
    }

    /// Create a system event
    pub fn system(summary: impl Into<String>) -> Self {
        Self::new(LogEventKind::System, summary)
    }

    /// Add content to the event
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Add tool arguments
    pub fn with_tool_args(mut self, args: serde_json::Value) -> Self {
        self.tool_args = Some(args);
        self
    }
}
