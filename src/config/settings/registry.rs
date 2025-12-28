//! Registry settings for agent adapter configuration

use serde::{Deserialize, Serialize};

/// Registry settings for agent adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySettings {
    /// List of enabled adapter IDs
    /// If empty (default), all adapters are enabled
    /// Example: ["claude", "codex"] - only these adapters will be registered
    #[serde(default)]
    pub enabled_adapters: Vec<String>,

    /// List of disabled adapter IDs
    /// These adapters will not be registered even if available
    /// Example: ["claude-terminal"] - terminal adapter will be skipped
    #[serde(default)]
    pub disabled_adapters: Vec<String>,

    /// Suffix used for terminal/REPL mode adapter IDs
    /// Default: "-terminal" (e.g., "claude-terminal")
    #[serde(default = "default_terminal_suffix")]
    pub terminal_suffix: String,
}

fn default_terminal_suffix() -> String {
    "-terminal".to_string()
}

impl Default for RegistrySettings {
    fn default() -> Self {
        Self {
            enabled_adapters: Vec::new(),
            disabled_adapters: Vec::new(),
            terminal_suffix: default_terminal_suffix(),
        }
    }
}
