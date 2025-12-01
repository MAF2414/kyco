//! Mode configuration types

use serde::{Deserialize, Serialize};

/// Mode configuration - the prompt builder
///
/// Modes define HOW to instruct the agent. They combine:
/// - A prompt template with placeholders
/// - A system prompt for context
/// - Default target and scope
/// - Allowed tools restrictions
///
/// Template placeholders:
/// - {target} - what to process (from target config)
/// - {scope} - the scope description
/// - {file} - the source file path
/// - {description} - user's description from comment
/// - {mode} - the mode name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    /// Default agent for this mode (can be overridden in marker)
    #[serde(default)]
    pub agent: Option<String>,

    /// Default target for this mode
    #[serde(default)]
    pub target_default: Option<String>,

    /// Default scope for this mode
    #[serde(default)]
    pub scope_default: Option<String>,

    /// The prompt template - the core instruction
    /// Placeholders: {target}, {scope}, {file}, {description}, {mode}
    pub prompt: Option<String>,

    /// System prompt addition for agent context
    pub system_prompt: Option<String>,

    /// Tools to allow for this mode (empty = all allowed)
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Tools to disallow for this mode
    #[serde(default)]
    pub disallowed_tools: Vec<String>,

    /// Short aliases for this mode (e.g., ["r", "ref"] for refactor)
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Possible output states this mode can produce (for chain triggers)
    /// Example: ["issues_found", "no_issues"] for review mode
    #[serde(default)]
    pub output_states: Vec<String>,
}
