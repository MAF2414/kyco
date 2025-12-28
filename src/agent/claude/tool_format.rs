//! Tool call formatting utilities for Claude adapter

/// Format a tool call for display
pub fn format_tool_call(name: &str, input: &serde_json::Value) -> String {
    match name {
        "Read" => {
            if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                format!("Read {}", path)
            } else {
                "Read file".to_string()
            }
        }
        "Write" => {
            if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                format!("Write {}", path)
            } else {
                "Write file".to_string()
            }
        }
        "Edit" => {
            if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                format!("Edit {}", path)
            } else {
                "Edit file".to_string()
            }
        }
        "Bash" => {
            if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                format!("Bash: {}", cmd)
            } else {
                "Bash command".to_string()
            }
        }
        "Glob" => {
            if let Some(pattern) = input.get("pattern").and_then(|v| v.as_str()) {
                format!("Glob: {}", pattern)
            } else {
                "Glob search".to_string()
            }
        }
        "Grep" => {
            if let Some(pattern) = input.get("pattern").and_then(|v| v.as_str()) {
                format!("Grep: {}", pattern)
            } else {
                "Grep search".to_string()
            }
        }
        _ => name.to_string(),
    }
}
