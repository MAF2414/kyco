//! Mode chain configuration types

use serde::{Deserialize, Serialize};

use super::ModeConfig;

/// A state definition for chain control flow
/// States are detected by searching for patterns in the previous step's output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDefinition {
    /// Unique identifier for this state (e.g., "issues_found", "tests_pass")
    pub id: String,
    /// Human-readable description of what this state means
    #[serde(default)]
    pub description: Option<String>,
    /// Patterns to search for in the output (any match triggers this state)
    /// Can be simple text or regex patterns
    pub patterns: Vec<String>,
    /// Whether patterns should be treated as regex (default: false = plain text search)
    #[serde(default)]
    pub is_regex: bool,
    /// Case-insensitive matching (default: true)
    #[serde(default = "default_case_insensitive")]
    pub case_insensitive: bool,
}

fn default_case_insensitive() -> bool {
    true
}

/// A step in a mode chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStep {
    /// The mode to execute in this step
    pub mode: String,
    /// States that trigger this step (if None, always runs)
    /// References state IDs defined in the chain's `states` array
    #[serde(default)]
    pub trigger_on: Option<Vec<String>>,
    /// States that skip this step
    /// References state IDs defined in the chain's `states` array
    #[serde(default)]
    pub skip_on: Option<Vec<String>>,
    /// Override agent for this step (uses mode's default if None)
    #[serde(default)]
    pub agent: Option<String>,
    /// Additional context to inject into the prompt
    #[serde(default)]
    pub inject_context: Option<String>,
}

/// A chain of modes to execute sequentially
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeChain {
    /// Version number for versioned merging (internal configs only)
    /// Higher versions will override user customizations
    #[serde(default)]
    pub version: u32,
    /// Human-readable description of what this chain does
    pub description: Option<String>,
    /// State definitions for this chain - detected via pattern matching in output
    #[serde(default)]
    pub states: Vec<StateDefinition>,
    /// The steps to execute in order
    pub steps: Vec<ChainStep>,
    /// Whether to stop the chain on first failure
    #[serde(default = "default_stop_on_failure")]
    pub stop_on_failure: bool,
    /// Pass the full response text to the next step (default: true)
    /// When true, the complete output is passed; when false, only the summary
    #[serde(default = "default_pass_full_response")]
    pub pass_full_response: bool,

    /// Force running in a git worktree for this chain
    /// - None: Use global settings (default)
    /// - Some(true): Always run in worktree
    /// - Some(false): Never run in worktree (overrides global)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_worktree: Option<bool>,
}

fn default_stop_on_failure() -> bool {
    true
}

fn default_pass_full_response() -> bool {
    true
}

/// Either a single mode or a chain of modes
#[derive(Debug, Clone)]
pub enum ModeOrChain {
    Mode(ModeConfig),
    Chain(ModeChain),
}
