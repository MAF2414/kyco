use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use super::parse::{unwrap_json_string_literal, value_to_string, yaml_to_json};

/// Parsed output from the agent's YAML summary block
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobResult {
    /// Short title describing what was done
    pub title: Option<String>,
    /// Suggested git commit subject line
    pub commit_subject: Option<String>,
    /// Suggested git commit body
    pub commit_body: Option<String>,
    /// Detailed description (2-3 sentences)
    pub details: Option<String>,
    /// Status: success, partial, or failed
    pub status: Option<String>,
    /// Longer summary for chain context (can be multiline, passed to next agent)
    pub summary: Option<String>,
    /// State identifier for chain triggers (e.g., "issues_found", "fixed", "tests_pass")
    pub state: Option<String>,
    /// Structured context data for next agent in chain
    pub next_context: Option<serde_json::Value>,
    /// Raw text output when no structured YAML is found
    pub raw_text: Option<String>,
}

impl JobResult {
    /// Parse a YAML summary block from agent output
    ///
    /// Supports multiple formats:
    /// 1. Standard YAML front matter with `---` markers
    /// 2. Legacy `---kyco` markers (backwards compatibility)
    /// 3. Falls back to raw text if no YAML structure found
    ///
    /// ```yaml
    /// ---
    /// title: Short title
    /// summary: |
    ///   This is a multiline summary
    ///   that spans multiple lines.
    /// state: issues_found
    /// ---
    /// ```
    pub fn parse(output: &str) -> Option<Self> {
        let trimmed = output.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Some runners return a JSON string literal as the "result" payload.
        // If we don't unwrap it here, the UI ends up showing quotes and escaped newlines
        // and YAML parsing fails because keys become "\\ntitle", "\\nstatus", etc.
        let output: Cow<'_, str> = unwrap_json_string_literal(trimmed)
            .map(Cow::Owned)
            .unwrap_or(Cow::Borrowed(trimmed));
        let output = output.as_ref().trim();

        // Try standard YAML markers first, then legacy ---kyco
        if let Some(result) = Self::parse_yaml_block(output, "---") {
            return Some(result);
        }
        if let Some(result) = Self::parse_yaml_block(output, "---kyco") {
            return Some(result);
        }

        // Try JSON structured output (SDK outputFormat / outputSchema)
        if let Some(result) = Self::parse_json_block(output) {
            return Some(result);
        }

        // No structured YAML found - extract raw text from the output
        if !output.is_empty() {
            return Some(JobResult {
                raw_text: Some(output.to_string()),
                ..Default::default()
            });
        }

        None
    }

    fn parse_json_block(output: &str) -> Option<Self> {
        let raw = output.trim();
        if !raw.starts_with('{') {
            return None;
        }

        let value: serde_json::Value = serde_json::from_str(raw).ok()?;
        let obj = value.as_object()?;

        let mut result = JobResult::default();
        result.title = obj
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.commit_subject = obj
            .get("commit_subject")
            .or_else(|| obj.get("commitSubject"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.commit_body = obj
            .get("commit_body")
            .or_else(|| obj.get("commitBody"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.details = obj
            .get("details")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.status = obj
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        result.state = obj
            .get("state")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        result.next_context = obj
            .get("next_context")
            .cloned()
            .or_else(|| obj.get("nextContext").cloned());

        result.summary = obj.get("summary").and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            other => serde_json::to_string_pretty(other).ok(),
        });

        let has_structured = result.title.is_some()
            || result.commit_subject.is_some()
            || result.commit_body.is_some()
            || result.details.is_some()
            || result.status.is_some()
            || result.summary.is_some()
            || result.state.is_some()
            || result.next_context.is_some();

        if has_structured { Some(result) } else { None }
    }

    /// Parse a YAML block with a specific start marker
    fn parse_yaml_block(output: &str, start_marker: &str) -> Option<Self> {
        let end_marker = "---";

        // Find the start marker
        let start_idx = output.find(start_marker)?;
        let content_start = start_idx + start_marker.len();

        // Find the closing --- after the start marker
        let remaining = &output[content_start..];

        // For standard `---`, we need to find the NEXT `---` (not the same one)
        // For `---kyco`, the next `---` is always the closing one
        let end_idx = if start_marker == "---" {
            // Skip whitespace and find next ---
            remaining.trim_start().find(end_marker).map(|i| {
                // Adjust for trimmed whitespace
                remaining.len() - remaining.trim_start().len() + i
            })?
        } else {
            remaining.find(end_marker)?
        };

        let yaml_content = remaining[..end_idx].trim();

        if yaml_content.is_empty() || yaml_content.len() < 5 {
            return None;
        }

        // Try to parse as proper YAML first (handles multiline values)
        if let Ok(yaml_value) = serde_yaml::from_str::<serde_yaml::Value>(yaml_content) {
            if let serde_yaml::Value::Mapping(map) = yaml_value {
                let mut result = JobResult::default();

                for (key, value) in map {
                    if let serde_yaml::Value::String(key_str) = key {
                        match key_str.as_str() {
                            "title" => result.title = value_to_string(&value),
                            "commit_subject" => result.commit_subject = value_to_string(&value),
                            "commit_body" => result.commit_body = value_to_string(&value),
                            "details" => result.details = value_to_string(&value),
                            "status" => result.status = value_to_string(&value),
                            "summary" => result.summary = value_to_string(&value),
                            "state" => result.state = value_to_string(&value),
                            "next_context" => {
                                result.next_context = yaml_to_json(&value);
                            }
                            _ => {}
                        }
                    }
                }

                if result.title.is_some()
                    || result.status.is_some()
                    || result.commit_subject.is_some()
                    || result.commit_body.is_some()
                {
                    return Some(result);
                }
            }
        }

        // Fallback: Parse simple key: value pairs (backwards compatibility)
        let mut result = JobResult::default();

        for line in yaml_content.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "title" => result.title = Some(value.to_string()),
                    "commit_subject" => result.commit_subject = Some(value.to_string()),
                    "commit_body" => result.commit_body = Some(value.to_string()),
                    "details" => result.details = Some(value.to_string()),
                    "status" => result.status = Some(value.to_string()),
                    "summary" => result.summary = Some(value.to_string()),
                    "state" => result.state = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        if result.title.is_some()
            || result.status.is_some()
            || result.commit_subject.is_some()
            || result.commit_body.is_some()
        {
            Some(result)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JobResult;

    #[test]
    fn parse_unwraps_json_string_literal_then_parses_yaml_block() {
        let inner = r#"I need permission to read the file.

---
title: Code review blocked - permission needed
commit_subject: N/A
details: Unable to read the requested file due to permission restrictions.
status: blocked
summary: |
  Cannot proceed with code review - file read permission was denied.
state: blocked
---
"#;

        let wrapped = serde_json::to_string(inner).expect("json wrap");
        let result = JobResult::parse(&wrapped).expect("parse");

        assert_eq!(
            result.title.as_deref(),
            Some("Code review blocked - permission needed")
        );
        assert_eq!(result.status.as_deref(), Some("blocked"));
        assert!(result.raw_text.is_none());
    }

    #[test]
    fn parse_unwraps_json_string_literal_for_raw_text() {
        let inner = "hello\nworld";
        let wrapped = serde_json::to_string(inner).expect("json wrap");
        let result = JobResult::parse(&wrapped).expect("parse");

        assert_eq!(result.raw_text.as_deref(), Some("hello\nworld"));
    }
}
