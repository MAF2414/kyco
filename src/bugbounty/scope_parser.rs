//! scope.md parser for BugBounty projects
//!
//! This is intentionally "best-effort": scope.md formats vary across platforms.
//! We parse common patterns (headings + bullet lists + simple tables) and return
//! a structured `ProjectScope` for guardrails and UI display.

use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;

use super::models::ProjectScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    InScope,
    OutOfScope,
}

/// Parse a `scope.md` file into a `ProjectScope`.
pub fn parse_scope_file(path: &Path) -> Result<ProjectScope> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read scope file: {}", path.display()))?;
    Ok(parse_scope_markdown(&content))
}

/// Parse scope markdown content into a `ProjectScope` (best effort).
pub fn parse_scope_markdown(content: &str) -> ProjectScope {
    let mut scope = ProjectScope::default();

    let mut section = Section::None;
    let mut saw_section_heading = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(heading) = parse_heading(trimmed) {
            let heading_lower = heading.to_lowercase();
            section = if is_in_scope_heading(&heading_lower) {
                saw_section_heading = true;
                Section::InScope
            } else if is_out_of_scope_heading(&heading_lower) {
                saw_section_heading = true;
                Section::OutOfScope
            } else {
                Section::None
            };
            continue;
        }

        // If the file has explicit headings, only parse list/table rows inside those sections.
        // Otherwise, fall back to parsing any bullets as in-scope.
        let effective_section = if saw_section_heading {
            section
        } else {
            Section::InScope
        };

        if let Some(item) = parse_bullet_item(trimmed) {
            push_item(&mut scope, effective_section, item);
            continue;
        }

        if let Some(item) = parse_table_row_first_column(trimmed) {
            push_item(&mut scope, effective_section, item);
            continue;
        }
    }

    scope.rate_limit = parse_rate_limit(content);

    if scope.notes.is_none() && scope.in_scope.is_empty() && scope.out_of_scope.is_empty() {
        let snippet = content.trim();
        if !snippet.is_empty() {
            scope.notes = Some(truncate(snippet, 2000));
        }
    }

    scope
}

fn parse_heading(line: &str) -> Option<&str> {
    let line = line.trim_start();
    if !line.starts_with('#') {
        return None;
    }
    Some(line.trim_start_matches('#').trim())
}

fn is_in_scope_heading(heading: &str) -> bool {
    heading.contains("in scope")
        || heading.contains("in-scope")
        || heading.contains("inscope")
        || heading == "scope"
        || heading.contains("assets in scope")
}

fn is_out_of_scope_heading(heading: &str) -> bool {
    heading.contains("out of scope")
        || heading.contains("out-of-scope")
        || heading.contains("oos")
        || heading.contains("excluded")
        || heading.contains("not in scope")
}

fn parse_bullet_item(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let Some(rest) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
    else {
        return None;
    };
    normalize_scope_item(rest)
}

fn parse_table_row_first_column(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }

    let mut parts = trimmed.split('|').map(|s| s.trim()).filter(|s| !s.is_empty());
    let first = parts.next()?;

    // Skip markdown table separators like | --- | --- |
    if first.chars().all(|c| c == '-' || c == ':' || c == ' ') {
        return None;
    }

    // Very common header cell names we want to skip.
    let first_lower = first.to_lowercase();
    if matches!(
        first_lower.as_str(),
        "asset" | "assets" | "domain" | "domains" | "url" | "urls" | "scope"
    ) {
        return None;
    }

    normalize_scope_item(first)
}

fn normalize_scope_item(raw: &str) -> Option<String> {
    let mut s = raw.trim();
    if s.is_empty() {
        return None;
    }

    // Strip common markdown code formatting/backticks.
    s = s.trim_matches('`');

    // Drop inline comments after " - " patterns (common in scope lists).
    if let Some((left, _)) = s.split_once("  ") {
        // preserve "http://x" (two spaces unlikely), this is best-effort.
        s = left.trim();
    }

    let s = s.trim().trim_end_matches(',').trim_end_matches('.');
    if s.is_empty() {
        return None;
    }

    Some(s.to_string())
}

fn push_item(scope: &mut ProjectScope, section: Section, item: String) {
    let target = match section {
        Section::InScope => &mut scope.in_scope,
        Section::OutOfScope => &mut scope.out_of_scope,
        Section::None => return,
    };

    if !target.iter().any(|x| x.eq_ignore_ascii_case(&item)) {
        target.push(item);
    }
}

fn parse_rate_limit(content: &str) -> Option<u32> {
    // Examples we want to catch:
    // - "Max 10 req/sec"
    // - "Rate limit: 5 req/s"
    // - "10 requests per second"
    let patterns = [
        r"(?i)(\d{1,4})\s*(?:req|requests?)\s*/\s*s(?:ec)?\b",
        r"(?i)(\d{1,4})\s*requests?\s*per\s*second\b",
        r"(?i)rate\s*limit[^0-9]{0,20}(\d{1,4})\s*(?:req|requests?)",
    ];

    for pat in patterns {
        let re = Regex::new(pat).ok()?;
        if let Some(caps) = re.captures(content) {
            if let Some(m) = caps.get(1) {
                if let Ok(n) = m.as_str().parse::<u32>() {
                    return Some(n);
                }
            }
        }
    }

    None
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let mut out = s[..max_len].to_string();
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headings_and_bullets() {
        let md = r#"
# Scope

## In Scope
- api.example.com
- *.example.com

## Out of Scope
- oos.example.com

Rate limit: 10 req/s
"#;

        let scope = parse_scope_markdown(md);
        assert_eq!(scope.in_scope, vec!["api.example.com", "*.example.com"]);
        assert_eq!(scope.out_of_scope, vec!["oos.example.com"]);
        assert_eq!(scope.rate_limit, Some(10));
    }

    #[test]
    fn parses_table_rows() {
        let md = r#"
## In Scope
| Asset | Notes |
| --- | --- |
| api.example.com | main |
| *.example.com | wildcard |
"#;

        let scope = parse_scope_markdown(md);
        assert_eq!(scope.in_scope, vec!["api.example.com", "*.example.com"]);
    }
}

