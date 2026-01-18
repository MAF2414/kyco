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

        if let Some(result) = Self::parse_yaml_block(output, "---kyco") {
            return Some(result);
        }
        if let Some(result) = Self::parse_yaml_block(output, "---") {
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

    fn parse_yaml_content(yaml_content: &str) -> Option<Self> {
        let yaml_content = yaml_content.trim();

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

                let has_structured = result.title.is_some()
                    || result.commit_subject.is_some()
                    || result.commit_body.is_some()
                    || result.details.is_some()
                    || result.status.is_some()
                    || result.summary.is_some()
                    || result.state.is_some()
                    || result.next_context.is_some();

                if has_structured {
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

        // Prefer line-based markers to avoid matching markdown separators or code examples.
        let lines: Vec<&str> = output.lines().collect();

        if start_marker == "---kyco" {
            // Find the last valid `---kyco` ... `---` block.
            let start_lines: Vec<usize> = lines
                .iter()
                .enumerate()
                .filter_map(|(idx, line)| (line.trim() == start_marker).then_some(idx))
                .collect();

            for &start_line in start_lines.iter().rev() {
                let mut end_line: Option<usize> = None;
                for idx in (start_line + 1)..lines.len() {
                    if lines[idx].trim() == end_marker {
                        end_line = Some(idx);
                        break;
                    }
                }
                let Some(end_line) = end_line else { continue; };
                let block = lines[(start_line + 1)..end_line].join("\n");
                if let Some(result) = Self::parse_yaml_content(&block) {
                    return Some(result);
                }
            }
            return None;
        }

        if start_marker == "---" {
            // Find the last valid `---` ... `---` pair that parses as our JobResult mapping.
            let marker_lines: Vec<usize> = lines
                .iter()
                .enumerate()
                .filter_map(|(idx, line)| (line.trim() == start_marker).then_some(idx))
                .collect();

            if marker_lines.len() < 2 {
                return None;
            }

            // Try from the end to pick the summary block the agent was instructed to append last.
            for pair_idx in (1..marker_lines.len()).rev() {
                let start_line = marker_lines[pair_idx - 1];
                let end_line = marker_lines[pair_idx];
                if end_line <= start_line {
                    continue;
                }
                let block = lines[(start_line + 1)..end_line].join("\n");
                if let Some(result) = Self::parse_yaml_content(&block) {
                    return Some(result);
                }
            }

            return None;
        };

        None
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

    #[test]
    fn parse_picks_last_yaml_block_when_multiple_markers_exist() {
        let output = r#"
Some explanation

---
not: "the summary block"
---

More text

---
title: Final summary
status: success
state: tests_pass
---
"#;

        let result = JobResult::parse(output).expect("parse");
        assert_eq!(result.title.as_deref(), Some("Final summary"));
        assert_eq!(result.status.as_deref(), Some("success"));
        assert_eq!(result.state.as_deref(), Some("tests_pass"));
    }

    #[test]
    fn parse_accepts_state_only_yaml_block() {
        let output = r#"
Done.

---
state: implemented
summary: |
  Implemented the feature.
---
"#;

        let result = JobResult::parse(output).expect("parse");
        assert_eq!(result.state.as_deref(), Some("implemented"));
        assert!(result.title.is_none());
    }
}
