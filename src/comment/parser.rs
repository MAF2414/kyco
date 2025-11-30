//! Comment parsing logic for KYCo markers
//!
//! New simplified syntax: {prefix}{agent:}?{mode} {description}?
//!
//! Examples (with default prefix "@"):
//! - @docs                      # just mode, default agent
//! - @docs write docstrings     # mode + description
//! - @claude:docs               # agent + mode
//! - @claude:fix handle errors  # agent + mode + description
//!
//! The prefix is configurable in .kyco/config.toml (default: "@@")
//! Works in any comment style: //, #, /* */, --, etc.

use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

use crate::{CommentTag, StatusMarker};

/// Alias resolver for agents and modes
#[derive(Debug, Clone)]
pub struct AliasResolver {
    /// Agent aliases: short -> canonical (e.g., "c" -> "claude")
    pub agents: HashMap<String, String>,
    /// Mode aliases: short -> canonical (e.g., "r" -> "refactor")
    pub modes: HashMap<String, String>,
}

impl Default for AliasResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl AliasResolver {
    /// Create a new alias resolver with default aliases
    pub fn new() -> Self {
        let mut agents = HashMap::new();
        // Claude aliases
        agents.insert("c".to_string(), "claude".to_string());
        agents.insert("cl".to_string(), "claude".to_string());
        agents.insert("claude".to_string(), "claude".to_string());
        // Codex aliases
        agents.insert("x".to_string(), "codex".to_string());
        agents.insert("cx".to_string(), "codex".to_string());
        agents.insert("codex".to_string(), "codex".to_string());
        // Gemini aliases
        agents.insert("g".to_string(), "gemini".to_string());
        agents.insert("gm".to_string(), "gemini".to_string());
        agents.insert("gemini".to_string(), "gemini".to_string());
        // CR alias for REPL mode claude (user-defined in config typically)
        agents.insert("cr".to_string(), "cr".to_string());

        let mut modes = HashMap::new();
        // Refactor
        modes.insert("r".to_string(), "refactor".to_string());
        modes.insert("ref".to_string(), "refactor".to_string());
        modes.insert("refactor".to_string(), "refactor".to_string());
        // Tests
        modes.insert("t".to_string(), "tests".to_string());
        modes.insert("test".to_string(), "tests".to_string());
        modes.insert("tests".to_string(), "tests".to_string());
        // Docs
        modes.insert("d".to_string(), "docs".to_string());
        modes.insert("doc".to_string(), "docs".to_string());
        modes.insert("docs".to_string(), "docs".to_string());
        // Review
        modes.insert("v".to_string(), "review".to_string());
        modes.insert("rev".to_string(), "review".to_string());
        modes.insert("review".to_string(), "review".to_string());
        // Implement
        modes.insert("i".to_string(), "implement".to_string());
        modes.insert("impl".to_string(), "implement".to_string());
        modes.insert("implement".to_string(), "implement".to_string());
        // Fix
        modes.insert("f".to_string(), "fix".to_string());
        modes.insert("fix".to_string(), "fix".to_string());
        // Optimize
        modes.insert("o".to_string(), "optimize".to_string());
        modes.insert("opt".to_string(), "optimize".to_string());
        modes.insert("optimize".to_string(), "optimize".to_string());

        Self { agents, modes }
    }

    /// Resolve an agent alias to its canonical name
    pub fn resolve_agent(&self, alias: &str) -> String {
        self.agents
            .get(&alias.to_lowercase())
            .cloned()
            .unwrap_or_else(|| alias.to_lowercase())
    }

    /// Resolve a mode alias to its canonical name
    pub fn resolve_mode(&self, alias: &str) -> String {
        self.modes
            .get(&alias.to_lowercase())
            .cloned()
            .unwrap_or_else(|| alias.to_lowercase())
    }

    /// Check if a string is a known mode (or mode alias)
    pub fn is_mode(&self, s: &str) -> bool {
        self.modes.contains_key(&s.to_lowercase())
    }

    /// Check if a string is a known agent (or agent alias)
    pub fn is_agent(&self, s: &str) -> bool {
        self.agents.contains_key(&s.to_lowercase())
    }

    /// Add a custom agent alias
    pub fn add_agent_alias(&mut self, alias: &str, canonical: &str) {
        self.agents
            .insert(alias.to_lowercase(), canonical.to_lowercase());
    }

    /// Add a custom mode alias
    pub fn add_mode_alias(&mut self, alias: &str, canonical: &str) {
        self.modes
            .insert(alias.to_lowercase(), canonical.to_lowercase());
    }
}

/// Kept for backwards compatibility but no longer used
#[derive(Debug, Clone)]
pub struct ModeDefaults;

/// Parser for KYCo comment markers
///
/// New simplified syntax: {prefix}{agent:}?{mode} {description}?
pub struct CommentParser {
    /// Marker prefix (default: "@")
    prefix: String,
    /// Alias resolver
    aliases: AliasResolver,
    /// Compiled regex pattern (built from prefix)
    pattern: Regex,
}

impl CommentParser {
    /// Create a new comment parser with default prefix "@@"
    pub fn new() -> Self {
        Self::with_prefix("@@")
    }

    /// Create a parser with a custom prefix
    pub fn with_prefix(prefix: &str) -> Self {
        let aliases = AliasResolver::new();
        let pattern = Self::build_pattern(prefix);
        Self {
            prefix: prefix.to_string(),
            aliases,
            pattern,
        }
    }

    /// Create a parser with custom aliases
    pub fn with_aliases(aliases: AliasResolver) -> Self {
        let pattern = Self::build_pattern("@");
        Self {
            prefix: "@".to_string(),
            aliases,
            pattern,
        }
    }

    /// Create a parser with custom prefix and aliases
    pub fn with_prefix_and_aliases(prefix: &str, aliases: AliasResolver) -> Self {
        let pattern = Self::build_pattern(prefix);
        Self {
            prefix: prefix.to_string(),
            aliases,
            pattern,
        }
    }

    /// Build regex pattern for the given prefix
    fn build_pattern(prefix: &str) -> Regex {
        // Escape special regex characters in prefix
        let escaped_prefix = regex::escape(prefix);

        // Pattern: {prefix}({agent}:)?{mode}(\s+{description})?(\s+\[{status}#{id}\])?
        // Groups: 1=agent (optional, without :), 2=mode, 3=description (optional), 4=status, 5=id
        let pattern_str = format!(
            r"{}(?:(\w+):)?(\w+)(?:\s+(.+?))?(?:\s*\[(\w+)#(\d+)\])?\s*$",
            escaped_prefix
        );

        Regex::new(&pattern_str).unwrap()
    }

    /// Parse a file and extract all comment tags
    pub fn parse_file(&self, path: &Path, content: &str) -> Vec<CommentTag> {
        let mut tags = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (idx, line) in lines.iter().enumerate() {
            let line_number = idx + 1; // 1-indexed

            if let Some(mut tag) = self.parse_line(path, line_number, line) {
                // Look for description in following comment lines (if no inline description)
                if tag.description.is_none() {
                    let description = self.extract_description(&lines, idx);
                    tag.description = description;
                }

                tags.push(tag);
            }
        }

        tags
    }

    /// Parse a single line for a KYCo marker
    pub fn parse_line(&self, path: &Path, line_number: usize, line: &str) -> Option<CommentTag> {
        let captures = self.pattern.captures(line)?;

        let agent_raw = captures.get(1).map(|m| m.as_str());
        let mode_raw = captures.get(2)?.as_str();
        let description = captures.get(3).map(|m| m.as_str().trim());
        let status_str = captures.get(4).map(|m| m.as_str());
        let id_str = captures.get(5).map(|m| m.as_str());

        // Determine if this is {agent}:{mode} or just {mode} or {agent} with description
        let (agent, mode) = if let Some(agent_str) = agent_raw {
            // We have {agent}:{mode} format
            (
                self.aliases.resolve_agent(agent_str),
                self.aliases.resolve_mode(mode_raw),
            )
        } else {
            // Just {mode} or {agent} - check what it is
            if self.aliases.is_mode(mode_raw) {
                // It's a mode, use default agent
                ("claude".to_string(), self.aliases.resolve_mode(mode_raw))
            } else if self.aliases.is_agent(mode_raw) {
                // It's an agent without explicit mode - use "implement" as default mode
                // e.g., "@@claude do something" -> agent=claude, mode=implement
                (self.aliases.resolve_agent(mode_raw), "implement".to_string())
            } else {
                // Unknown - treat as mode with default agent
                ("claude".to_string(), mode_raw.to_lowercase())
            }
        };

        let mut tag = CommentTag::new_simple(
            path.to_path_buf(),
            line_number,
            line.to_string(),
            agent,
            mode,
        );

        // Set description if present
        if let Some(desc) = description {
            if !desc.is_empty() {
                tag.description = Some(desc.to_string());
            }
        }

        // Parse status marker if present
        if let (Some(status), Some(id)) = (status_str, id_str) {
            let marker_str = format!("{}#{}", status, id);
            tag.status_marker = StatusMarker::parse(&marker_str);
            tag.job_id = tag.status_marker.as_ref().map(|m| m.job_id);
        }

        Some(tag)
    }

    /// Extract description from following comment lines
    fn extract_description(&self, lines: &[&str], start_idx: usize) -> Option<String> {
        let mut description_lines = Vec::new();

        for line in lines.iter().skip(start_idx + 1) {
            let trimmed = line.trim();

            // Check if it's a continuation comment (but not another marker)
            let is_comment = trimmed.starts_with("//")
                || trimmed.starts_with('#')
                || trimmed.starts_with("/*")
                || trimmed.starts_with("--")
                || trimmed.starts_with('*');

            let has_marker = trimmed.contains(&self.prefix);

            if is_comment && !has_marker {
                // Extract the comment content
                let content = trimmed
                    .trim_start_matches('/')
                    .trim_start_matches('#')
                    .trim_start_matches('*')
                    .trim_start_matches('-')
                    .trim();
                if !content.is_empty() {
                    description_lines.push(content.to_string());
                }
            } else {
                // Stop at non-comment line or another marker
                break;
            }
        }

        if description_lines.is_empty() {
            None
        } else {
            Some(description_lines.join(" "))
        }
    }
}

impl Default for CommentParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mode_only() {
        let parser = CommentParser::new(); // default prefix is @@
        let path = Path::new("test.rs");

        let tag = parser.parse_line(path, 1, "// @@docs").unwrap();
        assert_eq!(tag.agent, "claude"); // default
        assert_eq!(tag.mode, "docs");
        assert!(tag.description.is_none());
    }

    #[test]
    fn test_parse_mode_with_description() {
        let parser = CommentParser::new();
        let path = Path::new("test.rs");

        let tag = parser.parse_line(path, 1, "// @@docs write docstrings here").unwrap();
        assert_eq!(tag.agent, "claude");
        assert_eq!(tag.mode, "docs");
        assert_eq!(tag.description, Some("write docstrings here".to_string()));
    }

    #[test]
    fn test_parse_agent_and_mode() {
        let parser = CommentParser::new();
        let path = Path::new("test.rs");

        let tag = parser.parse_line(path, 1, "// @@codex:fix").unwrap();
        assert_eq!(tag.agent, "codex");
        assert_eq!(tag.mode, "fix");
    }

    #[test]
    fn test_parse_agent_and_mode_with_description() {
        let parser = CommentParser::new();
        let path = Path::new("test.rs");

        let tag = parser.parse_line(path, 1, "// @@claude:refactor make this cleaner").unwrap();
        assert_eq!(tag.agent, "claude");
        assert_eq!(tag.mode, "refactor");
        assert_eq!(tag.description, Some("make this cleaner".to_string()));
    }

    #[test]
    fn test_parse_short_aliases() {
        let parser = CommentParser::new();
        let path = Path::new("test.rs");

        // Short mode alias
        let tag = parser.parse_line(path, 1, "// @@d").unwrap();
        assert_eq!(tag.mode, "docs");

        // Short agent and mode aliases
        let tag = parser.parse_line(path, 1, "// @@c:r").unwrap();
        assert_eq!(tag.agent, "claude");
        assert_eq!(tag.mode, "refactor");

        // Codex with short mode
        let tag = parser.parse_line(path, 1, "// @@x:t").unwrap();
        assert_eq!(tag.agent, "codex");
        assert_eq!(tag.mode, "tests");
    }

    #[test]
    fn test_parse_with_status_marker() {
        let parser = CommentParser::new();
        let path = Path::new("test.rs");

        let tag = parser.parse_line(path, 1, "// @@fix handle error [pending#42]").unwrap();
        assert_eq!(tag.mode, "fix");
        assert_eq!(tag.description, Some("handle error".to_string()));
        assert_eq!(tag.job_id, Some(42));
    }

    #[test]
    fn test_different_comment_styles() {
        let parser = CommentParser::new();
        let path = Path::new("test.py");

        // Python style
        let tag = parser.parse_line(path, 1, "# @@docs").unwrap();
        assert_eq!(tag.mode, "docs");

        // Block comment
        let tag = parser.parse_line(path, 1, "/* @@fix this */").unwrap();
        assert_eq!(tag.mode, "fix");

        // SQL style
        let tag = parser.parse_line(path, 1, "-- @@review").unwrap();
        assert_eq!(tag.mode, "review");
    }

    #[test]
    fn test_custom_prefix() {
        let parser = CommentParser::with_prefix("::");
        let path = Path::new("test.rs");

        let tag = parser.parse_line(path, 1, "// ::docs write docs").unwrap();
        assert_eq!(tag.mode, "docs");
        assert_eq!(tag.description, Some("write docs".to_string()));

        let tag = parser.parse_line(path, 1, "// ::claude:fix").unwrap();
        assert_eq!(tag.agent, "claude");
        assert_eq!(tag.mode, "fix");
    }

    #[test]
    fn test_single_at_prefix() {
        // User can configure @ if they want (but @@ is safer default)
        let parser = CommentParser::with_prefix("@");
        let path = Path::new("test.rs");

        let tag = parser.parse_line(path, 1, "// @docs").unwrap();
        assert_eq!(tag.mode, "docs");

        let tag = parser.parse_line(path, 1, "// @claude:fix handle this").unwrap();
        assert_eq!(tag.agent, "claude");
        assert_eq!(tag.mode, "fix");
    }

    #[test]
    fn test_cr_agent_alias() {
        let parser = CommentParser::new();
        let path = Path::new("test.rs");

        // @@cr:docs - cr is an agent alias (user-defined REPL claude)
        let tag = parser.parse_line(path, 1, "// @@cr:docs").unwrap();
        assert_eq!(tag.agent, "cr");
        assert_eq!(tag.mode, "docs");
    }

    #[test]
    fn test_inline_marker() {
        let parser = CommentParser::new();
        let path = Path::new("test.rs");

        let tag = parser.parse_line(path, 1, "fn foo() { // @@fix handle edge case").unwrap();
        assert_eq!(tag.mode, "fix");
        assert_eq!(tag.description, Some("handle edge case".to_string()));
    }

    #[test]
    fn test_multiline_description() {
        let parser = CommentParser::new();
        let path = Path::new("test.rs");

        let content = r#"// @@refactor
// Make this function more readable
// Keep the same behavior
fn process_order() {}"#;

        let tags = parser.parse_file(path, content);
        assert_eq!(tags.len(), 1);

        let tag = &tags[0];
        assert_eq!(tag.mode, "refactor");
        assert_eq!(
            tag.description,
            Some("Make this function more readable Keep the same behavior".to_string())
        );
    }

    #[test]
    fn test_no_match_python_decorator() {
        // Ensure @staticmethod etc don't match with @@ prefix
        let parser = CommentParser::new();
        let path = Path::new("test.py");

        let result = parser.parse_line(path, 1, "@staticmethod");
        assert!(result.is_none(), "Should not match Python decorators");

        let result = parser.parse_line(path, 1, "@property");
        assert!(result.is_none(), "Should not match Python decorators");
    }

    #[test]
    fn test_agent_only_with_description() {
        // @@claude description -> agent=claude, mode=implement (default)
        let parser = CommentParser::new();
        let path = Path::new("test.rs");

        let tag = parser.parse_line(path, 1, "// @@claude wir brauchen professionellere docs hier").unwrap();
        assert_eq!(tag.agent, "claude");
        assert_eq!(tag.mode, "implement"); // default mode
        assert_eq!(tag.description, Some("wir brauchen professionellere docs hier".to_string()));

        // Short alias also works
        let tag = parser.parse_line(path, 1, "// @@c do something").unwrap();
        assert_eq!(tag.agent, "claude");
        assert_eq!(tag.mode, "implement");
        assert_eq!(tag.description, Some("do something".to_string()));

        // Codex
        let tag = parser.parse_line(path, 1, "// @@x fix this bug").unwrap();
        assert_eq!(tag.agent, "codex");
        assert_eq!(tag.mode, "implement");
        assert_eq!(tag.description, Some("fix this bug".to_string()));
    }
}
