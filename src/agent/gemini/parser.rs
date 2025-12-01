//! Gemini CLI output parsing

use crate::LogEvent;

/// Parse a Gemini output line into a LogEvent
///
/// Gemini CLI output format may vary. This provides a best-effort parsing.
pub fn parse_gemini_event(line: &str) -> LogEvent {
    // Try to parse as JSON first
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        if let Some(event_type) = json.get("type").and_then(|t| t.as_str()) {
            match event_type {
                "text" | "message" => {
                    let content = json
                        .get("content")
                        .or_else(|| json.get("text"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("");
                    return LogEvent::text(content.to_string());
                }
                "tool_call" | "function_call" => {
                    let name = json
                        .get("name")
                        .or_else(|| json.get("tool"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown");
                    return LogEvent::tool_call(name.to_string(), format!("Calling {}", name));
                }
                "tool_result" | "function_result" => {
                    let output = json
                        .get("output")
                        .or_else(|| json.get("result"))
                        .and_then(|o| o.as_str())
                        .unwrap_or("");
                    return LogEvent::tool_output("result", output.to_string());
                }
                "error" => {
                    let message = json
                        .get("message")
                        .or_else(|| json.get("error"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown error");
                    return LogEvent::error(message.to_string());
                }
                _ => {}
            }
        }
    }

    // Fall back to treating as plain text
    if line.trim().is_empty() {
        LogEvent::system("")
    } else {
        LogEvent::text(line.to_string())
    }
}
