//! Target configuration types

use serde::{Deserialize, Serialize};

/// Target configuration - defines what to process within scope
///
/// Targets define WHAT within the scope to process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    /// Human-readable description
    pub description: Option<String>,

    /// How to describe this target in prompts
    pub prompt_text: Option<String>,

    /// Short aliases (e.g., ["b", "blk"] for block)
    #[serde(default)]
    pub aliases: Vec<String>,
}
