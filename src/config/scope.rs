//! Scope configuration types

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Scope configuration - defines what code area to process
///
/// Scopes define WHERE to look. Built-in scopes can be extended.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeConfig {
    /// Human-readable description
    pub description: Option<String>,

    /// How to describe this scope in prompts
    pub prompt_text: Option<String>,

    /// Short aliases (e.g., ["f", "fn"] for function)
    #[serde(default)]
    pub aliases: Vec<String>,

    /// For language-specific scope detection (regex patterns)
    #[serde(default)]
    pub patterns: HashMap<String, Vec<String>>,
}
