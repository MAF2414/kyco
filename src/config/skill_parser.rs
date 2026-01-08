//! SKILL.md file parser
//!
//! Parses skill definition files in the SKILL.md format:
//! ```markdown
//! ---
//! name: skill-name
//! description: What this skill does
//! x-kyco:
//!   aliases: ["s", "sn"]
//!   output_states: ["done"]
//! ---
//!
//! # Instructions
//!
//! The markdown body with placeholders like {target}, {description}, etc.
//! ```

use super::skill::SkillConfig;
use std::path::Path;

/// Error type for skill parsing
#[derive(Debug, thiserror::Error)]
pub enum SkillParseError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid SKILL.md format: {0}")]
    FormatError(String),

    #[error("YAML parsing error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Parse a SKILL.md file from a path
pub fn parse_skill_file(path: &Path) -> Result<SkillConfig, SkillParseError> {
    let content = std::fs::read_to_string(path)?;
    let mut skill = parse_skill_content(&content)?;
    skill.source_path = Some(path.to_path_buf());
    Ok(skill)
}

/// Parse SKILL.md content from a string
pub fn parse_skill_content(content: &str) -> Result<SkillConfig, SkillParseError> {
    // Split frontmatter and body
    let (frontmatter, body) = split_frontmatter(content)?;

    // Parse YAML frontmatter
    let mut skill: SkillConfig = serde_yaml::from_str(&frontmatter)?;

    // Validate required fields
    if skill.name.is_empty() {
        return Err(SkillParseError::MissingField("name".to_string()));
    }

    // Set the instructions from the body
    skill.instructions = body.trim().to_string();

    Ok(skill)
}

/// Split content into YAML frontmatter and Markdown body
fn split_frontmatter(content: &str) -> Result<(String, String), SkillParseError> {
    let content = content.trim();

    // Must start with ---
    if !content.starts_with("---") {
        return Err(SkillParseError::FormatError(
            "SKILL.md must start with YAML frontmatter (---)".to_string(),
        ));
    }

    // Find the closing ---
    let after_first = &content[3..];
    let closing_pos = after_first.find("\n---");

    match closing_pos {
        Some(pos) => {
            let frontmatter = after_first[..pos].trim().to_string();
            let body = after_first[pos + 4..].to_string(); // Skip \n---
            Ok((frontmatter, body))
        }
        None => {
            // No closing ---, treat entire content after first --- as frontmatter
            // and body is empty
            Err(SkillParseError::FormatError(
                "Missing closing --- for YAML frontmatter".to_string(),
            ))
        }
    }
}

/// Create a new skill with default template
///
/// Per agentskills.io specification, the description should explain:
/// - What the skill does
/// - When to use it
///
/// Example: "Extracts text from PDF files. Use when working with PDF documents
/// or when the user mentions PDFs, forms, or document extraction."
pub fn create_skill_template(name: &str, description: Option<&str>) -> SkillConfig {
    // Create a human-readable title from the skill name
    let title = name
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let instructions = format!(
        r#"# {title}

Process `{{target}}`: {{description}}

{{ide_context}}

## Guidelines

1. Read and understand the code thoroughly before making changes
2. Make only the requested changes, avoiding unnecessary modifications
3. Preserve existing behavior unless explicitly asked to change it
4. Follow the project's existing code style and patterns

## System Context

You are a {name} assistant. Focus on clear, maintainable code that follows the project's conventions.
"#,
        title = title,
        name = name
    );

    SkillConfig {
        name: name.to_string(),
        description: description.map(String::from),
        instructions,
        kyco: Default::default(),
        source_path: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_frontmatter() {
        let content = r#"---
name: test
description: A test skill
---

# Instructions

Do something."#;

        let (frontmatter, body) = split_frontmatter(content).unwrap();
        assert!(frontmatter.contains("name: test"));
        assert!(body.contains("# Instructions"));
    }

    #[test]
    fn test_split_frontmatter_no_closing() {
        let content = r#"---
name: test
description: A test skill

# Instructions
"#;

        let result = split_frontmatter(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_skill_content_basic() {
        let content = r#"---
name: refactor
description: Refactor code
---

Refactor this: {target}"#;

        let skill = parse_skill_content(content).unwrap();
        assert_eq!(skill.name, "refactor");
        assert_eq!(skill.description, Some("Refactor code".to_string()));
        assert!(skill.instructions.contains("{target}"));
    }

    #[test]
    fn test_parse_skill_content_with_kyco_extensions() {
        let content = r#"---
name: review
description: Review code for issues
x-kyco:
  aliases: ["r", "rev"]
  disallowed_tools: ["Write", "Edit"]
  output_states: ["issues_found", "no_issues"]
---

# Review

Review this code for issues."#;

        let skill = parse_skill_content(content).unwrap();
        assert_eq!(skill.name, "review");
        assert_eq!(skill.kyco.aliases, vec!["r", "rev"]);
        assert_eq!(skill.kyco.disallowed_tools, vec!["Write", "Edit"]);
        assert_eq!(skill.kyco.output_states, vec!["issues_found", "no_issues"]);
    }

    #[test]
    fn test_parse_skill_content_missing_name() {
        let content = r#"---
description: No name here
---

Instructions"#;

        let result = parse_skill_content(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_skill_template() {
        let skill = create_skill_template("refactor", Some("Refactor code"));
        assert_eq!(skill.name, "refactor");
        assert_eq!(skill.description, Some("Refactor code".to_string()));
        assert!(skill.instructions.contains("{target}"));
        assert!(skill.instructions.contains("## System Context"));
    }
}
