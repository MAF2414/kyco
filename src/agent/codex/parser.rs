//! Codex CLI JSON output parser.

use crate::LogEvent;
use std::path::PathBuf;

/// Result from parsing a Codex event
pub enum CodexEventResult {
    /// Thread started (new or resumed)
    ThreadStarted { thread_id: String },
    /// Turn completed with usage stats
    TurnCompleted {
        input_tokens: u64,
        cached_input_tokens: u64,
        output_tokens: u64,
    },
    /// Assistant message (full text)
    AssistantMessage { text: String },
    /// Files changed during this run
    FilesChanged { paths: Vec<PathBuf> },
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
        "thread.started" => parse_thread_started(&json),
        "turn.completed" => parse_turn_completed(&json),
        "item.completed" | "item.started" => parse_item_event(&json, event_type),
        // Legacy message format
        "message" => parse_message(&json),
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
    let cached_input_tokens = usage
        .and_then(|u| u.get("cached_input_tokens"))
        .and_then(|t| t.as_u64())
        .unwrap_or(0);
    let output_tokens = usage
        .and_then(|u| u.get("output_tokens"))
        .and_then(|t| t.as_u64())
        .unwrap_or(0);

    CodexEventResult::TurnCompleted {
        input_tokens,
        cached_input_tokens,
        output_tokens,
    }
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
                let aggregated_output = item
                    .get("aggregated_output")
                    .and_then(|o| o.as_str())
                    .unwrap_or("");
                let exit_code = item.get("exit_code").and_then(|c| c.as_i64()).unwrap_or(-1);

                let mut summary = format!("Exit code: {}", exit_code);
                let output_preview = aggregated_output.lines().next().unwrap_or("");
                if !output_preview.is_empty() {
                    summary.push_str(&format!("\n{}", output_preview));
                }

                let mut ev = LogEvent::tool_output("bash", summary);
                if !aggregated_output.is_empty() {
                    ev = ev.with_content(aggregated_output.to_string());
                }
                CodexEventResult::Log(ev)
            }
        }
        "agent_message" => {
            let text = item.get("text").and_then(|t| t.as_str()).unwrap_or("");
            CodexEventResult::AssistantMessage {
                text: text.to_string(),
            }
        }
        "file_change" => {
            if event_type == "item.started" {
                return CodexEventResult::None;
            }

            let changes = item
                .get("changes")
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();

            let mut paths: Vec<PathBuf> = Vec::new();
            for change in changes {
                if let Some(p) = change.get("path").and_then(|p| p.as_str()) {
                    paths.push(PathBuf::from(p));
                }
            }

            if paths.is_empty() {
                return CodexEventResult::None;
            }

            CodexEventResult::FilesChanged { paths }
        }
        _ => CodexEventResult::None,
    }
}

fn parse_thread_started(json: &serde_json::Value) -> CodexEventResult {
    let thread_id = json
        .get("thread_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if thread_id.is_empty() {
        return CodexEventResult::None;
    }
    CodexEventResult::ThreadStarted { thread_id }
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
