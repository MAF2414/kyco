//! Mode chain configuration types

use serde::{Deserialize, Serialize};

use super::ModeConfig;

/// A step in a mode chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStep {
    /// The mode to execute in this step
    pub mode: String,
    /// States that trigger this step (if None, always runs)
    /// If the previous step's state is in this list, this step executes
    #[serde(default)]
    pub trigger_on: Option<Vec<String>>,
    /// States that skip this step
    /// If the previous step's state is in this list, this step is skipped
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
    /// Human-readable description of what this chain does
    pub description: Option<String>,
    /// The steps to execute in order
    pub steps: Vec<ChainStep>,
    /// Whether to stop the chain on first failure
    #[serde(default = "default_stop_on_failure")]
    pub stop_on_failure: bool,
}

fn default_stop_on_failure() -> bool {
    true
}

/// Either a single mode or a chain of modes
#[derive(Debug, Clone)]
pub enum ModeOrChain {
    Mode(ModeConfig),
    Chain(ModeChain),
}
