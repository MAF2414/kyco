//! Codex CLI JSON output parser

use crate::LogEvent;

/// Result from parsing a Codex event
pub enum CodexEventResult {
    /// A log event to display
    Log(LogEvent),
    /// No event to display
    None,
}

/// Parse a Codex JSON output line
///
/// Codex exec --json output format:
/// - `item.started` / `item.completed` - individual steps
/// - `turn.completed` - task finished (with usage stats)
/// - `message` - assistant messages
/// - `error` - errors
pub fn parse_codex_event(line: &str) -> CodexEventResult {
    let json: serde_json::Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return CodexEventResult::None,
    };

    let event_type = match json.get("type").and_then(|t| t.as_str()) {
        Some(t) => t,
        None => return CodexEventResult::None,
    };

    match event_type {
        // Turn completed = success
        "turn.completed" => parse_turn_completed(&json),

        // Item events - show reasoning and commands
        "item.completed" | "item.started" => parse_item_event(&json, event_type),

        // Legacy message format
        "message" => parse_message(&json),

        // Error events
        "error" => parse_error(&json),

        // Ignore other event types silently
        "session.created" | "session.updated" | "item.input_audio_transcription.completed" => {
            CodexEventResult::None
        }

        _ => CodexEventResult::None,
    }
}

fn parse_turn_completed(json: &serde_json::Value) -> CodexEventResult {
    let usage = json.get("usage");
    let input_tokens = usage
        .and_then(|u| u.get("input_tokens"))
        .and_then(|t| t.as_u64())
        .unwrap_or(0);
    let output_tokens = usage
        .and_then(|u| u.get("output_tokens"))
        .and_then(|t| t.as_u64())
        .unwrap_or(0);

    CodexEventResult::Log(LogEvent::system(format!(
        "Completed (tokens: {} in, {} out)",
        input_tokens, output_tokens
    )))
}

fn parse_item_event(json: &serde_json::Value, event_type: &str) -> CodexEventResult {
    let item = match json.get("item") {
        Some(i) => i,
        None => return CodexEventResult::None,
    };

    let item_type = item
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");

    match item_type {
        "reasoning" => {
            let text = item.get("text").and_then(|t| t.as_str()).unwrap_or("");
            // Only show first line of reasoning
            let first_line = text.lines().next().unwrap_or("");
            CodexEventResult::Log(LogEvent::thought(first_line.to_string()))
        }
        "command_execution" => {
            let cmd = item.get("command").and_then(|c| c.as_str()).unwrap_or("");
            if event_type == "item.started" {
                CodexEventResult::Log(LogEvent::tool_call("bash", cmd.to_string()))
            } else {
                // For completed, we could show output but it's often long
                CodexEventResult::None
            }
        }
        "agent_message" => {
            let text = item.get("text").and_then(|t| t.as_str()).unwrap_or("");
            let first_line = text.lines().next().unwrap_or("");
            CodexEventResult::Log(LogEvent::text(first_line.to_string()))
        }
        "file_edit" | "file_create" => {
            let path = item.get("path").and_then(|p| p.as_str()).unwrap_or("file");
            CodexEventResult::Log(LogEvent::tool_call(item_type, path.to_string()))
        }
        _ => CodexEventResult::None,
    }
}

fn parse_message(json: &serde_json::Value) -> CodexEventResult {
    let content = match json.get("content").and_then(|c| c.as_str()) {
        Some(c) => c,
        None => return CodexEventResult::None,
    };
    let role = json
        .get("role")
        .and_then(|r| r.as_str())
        .unwrap_or("unknown");

    if role == "assistant" {
        CodexEventResult::Log(LogEvent::text(content.to_string()))
    } else {
        CodexEventResult::Log(LogEvent::system(format!("[{}] {}", role, content)))
    }
}

fn parse_error(json: &serde_json::Value) -> CodexEventResult {
    let message = json
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("Unknown error");
    CodexEventResult::Log(LogEvent::error(message.to_string()))
}
