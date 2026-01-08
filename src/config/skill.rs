//! Skill configuration types
//!
//! Skills are the new way to define agent instructions, stored as SKILL.md files
//! in `.claude/skills/` or `.codex/skills/` directories.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Claude-specific skill options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeSkillOptions {
    /// Permission mode for Claude SDK
    /// - "default": Normal permission checks (asks for everything)
    /// - "acceptEdits": Auto-accepts file edits, asks for Bash
    /// - "bypassPermissions": Full auto, no questions (dangerous!)
    /// - "plan": Planning mode (no execution)
    #[serde(default = "default_claude_permission")]
    pub permission_mode: String,
}

impl Default for ClaudeSkillOptions {
    fn default() -> Self {
        Self {
            permission_mode: default_claude_permission(),
        }
    }
}

fn default_claude_permission() -> String {
    "acceptEdits".to_string()
}

/// Codex-specific skill options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexSkillOptions {
    /// Sandbox mode for Codex SDK
    /// - "read-only": No file modifications
    /// - "workspace-write": Can modify files in workspace (safe default)
    /// - "danger-full-access": Full access including network
    #[serde(default = "default_codex_sandbox")]
    pub sandbox: String,
}

impl Default for CodexSkillOptions {
    fn default() -> Self {
        Self {
            sandbox: default_codex_sandbox(),
        }
    }
}

fn default_codex_sandbox() -> String {
    "workspace-write".to_string()
}

/// Session type for the agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillSessionType {
    /// One-shot execution - no session persistence
    #[default]
    Oneshot,
    /// Session mode - conversation can be resumed/continued
    Session,
}

/// kyco-specific extensions stored in the `x-kyco` namespace of SKILL.md frontmatter
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillKycoExtensions {
    /// Short aliases for this skill (e.g., ["r", "rev"] for review)
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Default agent for this skill
    #[serde(default)]
    pub agent: Option<String>,

    /// Default target for this skill
    #[serde(default)]
    pub target_default: Option<String>,

    /// Default scope for this skill
    #[serde(default)]
    pub scope_default: Option<String>,

    /// Session mode: oneshot (default) or session
    #[serde(default)]
    pub session_mode: SkillSessionType,

    /// Maximum turns/iterations (0 = unlimited)
    #[serde(default)]
    pub max_turns: u32,

    /// Model override
    #[serde(default)]
    pub model: Option<String>,

    /// Tools to disallow for this skill
    #[serde(default)]
    pub disallowed_tools: Vec<String>,

    /// Tools to explicitly allow (legacy)
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Possible output states for chain triggers
    #[serde(default)]
    pub output_states: Vec<String>,

    /// Custom state detection prompt
    #[serde(default)]
    pub state_prompt: Option<String>,

    /// Force running in a git worktree for this skill
    #[serde(default)]
    pub use_worktree: Option<bool>,

    /// Claude SDK specific options
    #[serde(default)]
    pub claude: Option<ClaudeSkillOptions>,

    /// Codex SDK specific options
    #[serde(default)]
    pub codex: Option<CodexSkillOptions>,
}

/// Skill configuration - loaded from SKILL.md files
///
/// Skills define HOW to instruct the agent. They combine:
/// - A name and description (YAML frontmatter)
/// - Instructions (Markdown body with placeholders)
/// - kyco-specific extensions (x-kyco namespace)
///
/// Template placeholders in instructions:
/// - {target} - what to process (from target config)
/// - {scope} - the scope description
/// - {file} - the source file path
/// - {description} - user's description from comment
/// - {skill} - the skill name
/// - {ide_context} - IDE context injection point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    /// Skill name (required, from frontmatter)
    pub name: String,

    /// Human-readable description (from frontmatter)
    #[serde(default)]
    pub description: Option<String>,

    /// The instruction body (Markdown content after frontmatter)
    /// Contains prompt template + system prompt combined
    #[serde(skip)]
    pub instructions: String,

    /// kyco-specific extensions (from x-kyco namespace in frontmatter)
    #[serde(default, rename = "x-kyco")]
    pub kyco: SkillKycoExtensions,

    /// Source file path (where this skill was loaded from)
    #[serde(skip)]
    pub source_path: Option<PathBuf>,
}

impl Default for SkillConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            instructions: String::new(),
            kyco: SkillKycoExtensions::default(),
            source_path: None,
        }
    }
}

impl SkillConfig {
    /// Get the effective Claude permission mode
    /// Returns the configured value or derives from disallowed_tools
    pub fn get_claude_permission(&self) -> String {
        if let Some(ref claude) = self.kyco.claude {
            return claude.permission_mode.clone();
        }

        let blocks_writes = self
            .kyco
            .disallowed_tools
            .iter()
            .any(|t| t == "Write" || t == "Edit");

        if blocks_writes {
            "default".to_string()
        } else {
            "acceptEdits".to_string()
        }
    }

    /// Get the effective Codex sandbox mode
    /// Returns the configured value or derives from disallowed_tools
    pub fn get_codex_sandbox(&self) -> String {
        if let Some(ref codex) = self.kyco.codex {
            return codex.sandbox.clone();
        }

        let blocks_writes = self
            .kyco
            .disallowed_tools
            .iter()
            .any(|t| t == "Write" || t == "Edit");
        let blocks_bash = self.kyco.disallowed_tools.iter().any(|t| t == "Bash");

        if blocks_writes || blocks_bash {
            "read-only".to_string()
        } else {
            "workspace-write".to_string()
        }
    }

    /// Get the prompt template from instructions
    /// Extracts the main instruction content (before any ## System Context section)
    pub fn get_prompt_template(&self) -> &str {
        // If there's a "## System Context" or "## System" section, return everything before it
        if let Some(pos) = self.instructions.find("## System") {
            self.instructions[..pos].trim()
        } else {
            self.instructions.trim()
        }
    }

    /// Get the system prompt from instructions
    /// Extracts content after ## System Context section (if present)
    pub fn get_system_prompt(&self) -> Option<&str> {
        // Find "## System Context" or "## System" section
        let markers = ["## System Context", "## System"];
        for marker in markers {
            if let Some(pos) = self.instructions.find(marker) {
                let after_header = &self.instructions[pos + marker.len()..];
                // Skip to the next line
                if let Some(newline_pos) = after_header.find('\n') {
                    let content = after_header[newline_pos + 1..].trim();
                    if !content.is_empty() {
                        return Some(content);
                    }
                }
            }
        }
        None
    }

    /// Convert to SKILL.md format string
    pub fn to_skill_md(&self) -> String {
        let mut output = String::new();

        // YAML frontmatter
        output.push_str("---\n");
        output.push_str(&format!("name: {}\n", self.name));

        if let Some(ref desc) = self.description {
            output.push_str(&format!("description: {}\n", desc));
        }

        // x-kyco extensions (only if non-default)
        if !self.kyco.aliases.is_empty()
            || self.kyco.agent.is_some()
            || self.kyco.session_mode != SkillSessionType::Oneshot
            || self.kyco.max_turns != 0
            || self.kyco.model.is_some()
            || !self.kyco.disallowed_tools.is_empty()
            || !self.kyco.output_states.is_empty()
            || self.kyco.use_worktree.is_some()
        {
            output.push_str("x-kyco:\n");

            if !self.kyco.aliases.is_empty() {
                output.push_str(&format!(
                    "  aliases: [{}]\n",
                    self.kyco
                        .aliases
                        .iter()
                        .map(|a| format!("\"{}\"", a))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }

            if let Some(ref agent) = self.kyco.agent {
                output.push_str(&format!("  agent: {}\n", agent));
            }

            if self.kyco.session_mode != SkillSessionType::Oneshot {
                output.push_str("  session_mode: session\n");
            }

            if self.kyco.max_turns != 0 {
                output.push_str(&format!("  max_turns: {}\n", self.kyco.max_turns));
            }

            if let Some(ref model) = self.kyco.model {
                output.push_str(&format!("  model: {}\n", model));
            }

            if !self.kyco.disallowed_tools.is_empty() {
                output.push_str(&format!(
                    "  disallowed_tools: [{}]\n",
                    self.kyco
                        .disallowed_tools
                        .iter()
                        .map(|t| format!("\"{}\"", t))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }

            if !self.kyco.output_states.is_empty() {
                output.push_str(&format!(
                    "  output_states: [{}]\n",
                    self.kyco
                        .output_states
                        .iter()
                        .map(|s| format!("\"{}\"", s))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }

            if let Some(use_worktree) = self.kyco.use_worktree {
                output.push_str(&format!("  use_worktree: {}\n", use_worktree));
            }
        }

        output.push_str("---\n\n");

        // Markdown body (instructions)
        output.push_str(&self.instructions);

        output
    }
}

// ============================================================================
// Skill validation (per agentskills.io specification)
// ============================================================================

/// Skill validation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillValidationError {
    /// Name is empty
    NameEmpty,
    /// Name exceeds 64 characters
    NameTooLong(usize),
    /// Name contains invalid characters (must be lowercase alphanumeric + hyphens)
    NameInvalidChars,
    /// Name starts with a hyphen
    NameStartsWithHyphen,
    /// Name ends with a hyphen
    NameEndsWithHyphen,
    /// Name contains consecutive hyphens
    NameConsecutiveHyphens,
    /// Description is missing (required per spec)
    DescriptionMissing,
    /// Description exceeds 1024 characters
    DescriptionTooLong(usize),
}

impl std::fmt::Display for SkillValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NameEmpty => write!(f, "Skill name cannot be empty"),
            Self::NameTooLong(len) => {
                write!(f, "Skill name too long ({} chars, max 64)", len)
            }
            Self::NameInvalidChars => {
                write!(f, "Skill name must be lowercase alphanumeric with hyphens only")
            }
            Self::NameStartsWithHyphen => write!(f, "Skill name cannot start with a hyphen"),
            Self::NameEndsWithHyphen => write!(f, "Skill name cannot end with a hyphen"),
            Self::NameConsecutiveHyphens => {
                write!(f, "Skill name cannot contain consecutive hyphens")
            }
            Self::DescriptionMissing => write!(f, "Skill description is required"),
            Self::DescriptionTooLong(len) => {
                write!(f, "Skill description too long ({} chars, max 1024)", len)
            }
        }
    }
}

impl std::error::Error for SkillValidationError {}

/// Validate a skill name per agentskills.io specification
///
/// Requirements:
/// - 1-64 characters
/// - Lowercase alphanumeric + hyphens only
/// - No leading/trailing/consecutive hyphens
/// - Must match parent directory name
pub fn validate_skill_name(name: &str) -> Result<(), SkillValidationError> {
    // Check empty
    if name.is_empty() {
        return Err(SkillValidationError::NameEmpty);
    }

    // Check length (max 64)
    if name.len() > 64 {
        return Err(SkillValidationError::NameTooLong(name.len()));
    }

    // Check for valid characters (lowercase alphanumeric + hyphens)
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(SkillValidationError::NameInvalidChars);
    }

    // Check for leading hyphen
    if name.starts_with('-') {
        return Err(SkillValidationError::NameStartsWithHyphen);
    }

    // Check for trailing hyphen
    if name.ends_with('-') {
        return Err(SkillValidationError::NameEndsWithHyphen);
    }

    // Check for consecutive hyphens
    if name.contains("--") {
        return Err(SkillValidationError::NameConsecutiveHyphens);
    }

    Ok(())
}

/// Validate a skill description per agentskills.io specification
///
/// Requirements:
/// - Required (1-1024 characters)
/// - Should describe what the skill does AND when to use it
pub fn validate_skill_description(description: Option<&str>) -> Result<(), SkillValidationError> {
    match description {
        None => Err(SkillValidationError::DescriptionMissing),
        Some(desc) if desc.is_empty() => Err(SkillValidationError::DescriptionMissing),
        Some(desc) if desc.len() > 1024 => {
            Err(SkillValidationError::DescriptionTooLong(desc.len()))
        }
        Some(_) => Ok(()),
    }
}

/// Validate a complete skill configuration
pub fn validate_skill(skill: &SkillConfig) -> Result<(), SkillValidationError> {
    validate_skill_name(&skill.name)?;
    validate_skill_description(skill.description.as_deref())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_session_type_default() {
        let session_type = SkillSessionType::default();
        assert_eq!(session_type, SkillSessionType::Oneshot);
    }

    #[test]
    fn test_skill_config_default() {
        let config = SkillConfig::default();
        assert!(config.name.is_empty());
        assert!(config.description.is_none());
        assert!(config.instructions.is_empty());
    }

    #[test]
    fn test_get_claude_permission_default() {
        let config = SkillConfig::default();
        assert_eq!(config.get_claude_permission(), "acceptEdits");
    }

    #[test]
    fn test_get_claude_permission_readonly() {
        let mut config = SkillConfig::default();
        config.kyco.disallowed_tools = vec!["Write".to_string(), "Edit".to_string()];
        assert_eq!(config.get_claude_permission(), "default");
    }

    #[test]
    fn test_get_codex_sandbox_default() {
        let config = SkillConfig::default();
        assert_eq!(config.get_codex_sandbox(), "workspace-write");
    }

    #[test]
    fn test_get_codex_sandbox_readonly() {
        let mut config = SkillConfig::default();
        config.kyco.disallowed_tools = vec!["Bash".to_string()];
        assert_eq!(config.get_codex_sandbox(), "read-only");
    }

    #[test]
    fn test_get_prompt_template() {
        let mut config = SkillConfig::default();
        config.instructions = "Refactor this code.\n\n## System Context\nYou are a refactoring assistant.".to_string();
        assert_eq!(config.get_prompt_template(), "Refactor this code.");
    }

    #[test]
    fn test_get_system_prompt() {
        let mut config = SkillConfig::default();
        config.instructions = "Refactor this code.\n\n## System Context\nYou are a refactoring assistant.".to_string();
        assert_eq!(config.get_system_prompt(), Some("You are a refactoring assistant."));
    }

    #[test]
    fn test_to_skill_md() {
        let mut config = SkillConfig::default();
        config.name = "refactor".to_string();
        config.description = Some("Refactor code while preserving behavior".to_string());
        config.instructions = "# Instructions\n\nRefactor `{target}`: {description}".to_string();
        config.kyco.aliases = vec!["r".to_string(), "ref".to_string()];

        let md = config.to_skill_md();
        assert!(md.contains("name: refactor"));
        assert!(md.contains("description: Refactor code while preserving behavior"));
        assert!(md.contains("aliases: [\"r\", \"ref\"]"));
        assert!(md.contains("# Instructions"));
    }

    // ========================================================================
    // Validation tests (per agentskills.io specification)
    // ========================================================================

    #[test]
    fn test_validate_skill_name_valid() {
        assert!(validate_skill_name("pdf-processing").is_ok());
        assert!(validate_skill_name("review").is_ok());
        assert!(validate_skill_name("my-skill-123").is_ok());
        assert!(validate_skill_name("a").is_ok()); // Minimum 1 char
    }

    #[test]
    fn test_validate_skill_name_empty() {
        assert_eq!(
            validate_skill_name(""),
            Err(SkillValidationError::NameEmpty)
        );
    }

    #[test]
    fn test_validate_skill_name_too_long() {
        let long_name = "a".repeat(65);
        assert_eq!(
            validate_skill_name(&long_name),
            Err(SkillValidationError::NameTooLong(65))
        );

        // 64 chars should be OK
        let max_name = "a".repeat(64);
        assert!(validate_skill_name(&max_name).is_ok());
    }

    #[test]
    fn test_validate_skill_name_invalid_chars() {
        assert_eq!(
            validate_skill_name("PDF-Processing"), // Uppercase
            Err(SkillValidationError::NameInvalidChars)
        );
        assert_eq!(
            validate_skill_name("skill_name"), // Underscore
            Err(SkillValidationError::NameInvalidChars)
        );
        assert_eq!(
            validate_skill_name("skill name"), // Space
            Err(SkillValidationError::NameInvalidChars)
        );
    }

    #[test]
    fn test_validate_skill_name_hyphen_position() {
        assert_eq!(
            validate_skill_name("-pdf"),
            Err(SkillValidationError::NameStartsWithHyphen)
        );
        assert_eq!(
            validate_skill_name("pdf-"),
            Err(SkillValidationError::NameEndsWithHyphen)
        );
        assert_eq!(
            validate_skill_name("pdf--processing"),
            Err(SkillValidationError::NameConsecutiveHyphens)
        );
    }

    #[test]
    fn test_validate_skill_description() {
        // Valid description
        assert!(validate_skill_description(Some("A valid description")).is_ok());

        // Missing description
        assert_eq!(
            validate_skill_description(None),
            Err(SkillValidationError::DescriptionMissing)
        );
        assert_eq!(
            validate_skill_description(Some("")),
            Err(SkillValidationError::DescriptionMissing)
        );

        // Too long description
        let long_desc = "a".repeat(1025);
        assert_eq!(
            validate_skill_description(Some(&long_desc)),
            Err(SkillValidationError::DescriptionTooLong(1025))
        );

        // Max length is OK
        let max_desc = "a".repeat(1024);
        assert!(validate_skill_description(Some(&max_desc)).is_ok());
    }

    #[test]
    fn test_validate_skill_complete() {
        let mut skill = SkillConfig::default();
        skill.name = "valid-skill".to_string();
        skill.description = Some("A valid description".to_string());

        assert!(validate_skill(&skill).is_ok());

        // Invalid name
        skill.name = "Invalid".to_string();
        assert_eq!(
            validate_skill(&skill),
            Err(SkillValidationError::NameInvalidChars)
        );

        // Missing description
        skill.name = "valid-skill".to_string();
        skill.description = None;
        assert_eq!(
            validate_skill(&skill),
            Err(SkillValidationError::DescriptionMissing)
        );
    }
}
