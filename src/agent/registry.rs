//! Agent registry for managing multiple agent adapters

use std::collections::HashMap;
use std::sync::Arc;

use crate::{AgentConfig, AgentMode, CliType};

use super::claude::ClaudeAdapter;
use super::codex::CodexAdapter;
use super::gemini::GeminiAdapter;
use super::terminal::TerminalAdapter;
use super::runner::AgentRunner;

/// Default suffix for terminal/REPL mode adapter IDs
/// Used to distinguish print mode adapters (e.g., "claude") from
/// REPL mode adapters (e.g., "claude-terminal")
pub const DEFAULT_TERMINAL_SUFFIX: &str = "-terminal";

/// Map a CLI type to its corresponding adapter ID for print mode
fn cli_type_to_id(cli_type: CliType) -> Option<&'static str> {
    match cli_type {
        CliType::Claude => Some("claude"),
        CliType::Codex => Some("codex"),
        CliType::Gemini => Some("gemini"),
        CliType::Custom => None,
    }
}

/// Map a CLI type to its corresponding adapter ID for REPL/terminal mode
fn cli_type_to_terminal_id(cli_type: CliType) -> Option<String> {
    cli_type_to_id(cli_type).map(|id| format!("{}{}", id, DEFAULT_TERMINAL_SUFFIX))
}

/// Registry for agent adapters
///
/// The registry manages all available agent adapters and provides
/// factory methods to create the appropriate adapter for a given CLI type.
#[derive(Clone)]
pub struct AgentRegistry {
    /// Registered adapters by agent ID (print mode)
    adapters: HashMap<String, Arc<dyn AgentRunner>>,
    /// Registered adapters for REPL mode
    repl_adapters: HashMap<String, Arc<dyn AgentRunner>>,
}

impl AgentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
            repl_adapters: HashMap::new(),
        }
    }

    /// Create a registry with all default adapters registered
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Register default print mode adapters
        registry.register(Arc::new(ClaudeAdapter::new()));
        registry.register(Arc::new(CodexAdapter::new()));
        registry.register(Arc::new(GeminiAdapter::new()));

        // Register REPL mode adapters (using Terminal.app on macOS)
        registry.register_repl(Arc::new(TerminalAdapter::claude()));
        registry.register_repl(Arc::new(TerminalAdapter::codex()));
        registry.register_repl(Arc::new(TerminalAdapter::gemini()));

        registry
    }

    /// Register a print mode adapter
    pub fn register(&mut self, adapter: Arc<dyn AgentRunner>) {
        self.adapters.insert(adapter.id().to_string(), adapter);
    }

    /// Register a REPL mode adapter
    pub fn register_repl(&mut self, adapter: Arc<dyn AgentRunner>) {
        self.repl_adapters.insert(adapter.id().to_string(), adapter);
    }

    /// Get a print mode adapter by ID
    pub fn get(&self, id: &str) -> Option<Arc<dyn AgentRunner>> {
        self.adapters.get(id).cloned()
    }

    /// Get a REPL mode adapter by ID
    pub fn get_repl(&self, id: &str) -> Option<Arc<dyn AgentRunner>> {
        self.repl_adapters.get(id).cloned()
    }

    /// Get an adapter for a CLI type (print mode)
    pub fn get_for_cli_type(&self, cli_type: CliType) -> Option<Arc<dyn AgentRunner>> {
        cli_type_to_id(cli_type).and_then(|id| self.get(id))
    }

    /// Get a REPL adapter for a CLI type
    pub fn get_repl_for_cli_type(&self, cli_type: CliType) -> Option<Arc<dyn AgentRunner>> {
        cli_type_to_terminal_id(cli_type).and_then(|id| self.get_repl(&id))
    }

    /// Get or create an adapter for a config
    ///
    /// This looks up an existing adapter by ID, or falls back to CLI type.
    /// Respects the agent's mode setting (print vs repl).
    pub fn get_for_config(&self, config: &AgentConfig) -> Option<Arc<dyn AgentRunner>> {
        match config.mode {
            AgentMode::Print => {
                // First try by ID
                if let Some(adapter) = self.get(&config.id) {
                    return Some(adapter);
                }
                // Fall back to CLI type
                self.get_for_cli_type(config.cli_type)
            }
            AgentMode::Repl => {
                // First try by ID with terminal suffix
                let terminal_id = format!("{}{}", config.id, DEFAULT_TERMINAL_SUFFIX);
                if let Some(adapter) = self.get_repl(&terminal_id) {
                    return Some(adapter);
                }
                // Fall back to CLI type
                self.get_repl_for_cli_type(config.cli_type)
            }
        }
    }

    /// List all available adapters
    pub fn list_available(&self) -> Vec<&str> {
        self.adapters
            .iter()
            .filter(|(_, adapter)| adapter.is_available())
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// List all registered adapter IDs
    pub fn list_all(&self) -> Vec<&str> {
        self.adapters.keys().map(|s| s.as_str()).collect()
    }

    /// Check if an adapter is available
    pub fn is_available(&self, id: &str) -> bool {
        self.adapters
            .get(id)
            .map(|a| a.is_available())
            .unwrap_or(false)
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = AgentRegistry::with_defaults();

        assert!(registry.get("claude").is_some());
        assert!(registry.get("codex").is_some());
        assert!(registry.get("gemini").is_some());
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn test_get_for_cli_type() {
        let registry = AgentRegistry::with_defaults();

        assert!(registry.get_for_cli_type(CliType::Claude).is_some());
        assert!(registry.get_for_cli_type(CliType::Codex).is_some());
        assert!(registry.get_for_cli_type(CliType::Gemini).is_some());
        assert!(registry.get_for_cli_type(CliType::Custom).is_none());
    }
}
