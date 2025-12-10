//! Agent registry for managing multiple agent adapters.
//!
//! This module provides a centralized registry pattern for managing AI coding agent
//! adapters. It supports two execution modes:
//!
//! - **Print mode**: Runs the agent CLI once and captures output (batch execution)
//! - **REPL mode**: Opens an interactive terminal session with the agent
//!
//! # Architecture
//!
//! The registry maintains two separate adapter maps:
//! - `adapters`: Print mode adapters (e.g., "claude", "codex", "gemini")
//! - `repl_adapters`: REPL mode adapters (e.g., "claude-terminal", "codex-terminal")
//!
//! # Usage
//!
//! ```rust,ignore
//! use coderail::agent::AgentRegistry;
//! use coderail::{AgentConfig, CliType};
//!
//! // Create a registry with all default adapters
//! let registry = AgentRegistry::with_defaults();
//!
//! // Get an adapter by CLI type
//! if let Some(adapter) = registry.get_for_cli_type(CliType::Claude) {
//!     if adapter.is_available() {
//!         // Run the adapter...
//!     }
//! }
//!
//! // Get an adapter for a specific config (respects mode setting)
//! let config = AgentConfig::default();
//! if let Some(adapter) = registry.get_for_config(&config) {
//!     // Execute...
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use crate::{AgentConfig, AgentMode, CliType};

use super::claude::ClaudeAdapter;
use super::codex::CodexAdapter;
use super::gemini::GeminiAdapter;
use super::terminal::TerminalAdapter;
use super::runner::AgentRunner;

/// Default suffix appended to adapter IDs for terminal/REPL mode.
///
/// Used to distinguish print mode adapters (e.g., `"claude"`) from
/// REPL mode adapters (e.g., `"claude-terminal"`).
///
/// # Example
///
/// ```rust,ignore
/// let print_id = "claude";
/// let repl_id = format!("{}{}", print_id, DEFAULT_TERMINAL_SUFFIX);
/// assert_eq!(repl_id, "claude-terminal");
/// ```
pub const DEFAULT_TERMINAL_SUFFIX: &str = "-terminal";

/// Maps a [`CliType`] to its corresponding adapter ID for print mode.
///
/// # Arguments
///
/// * `cli_type` - The CLI type to map
///
/// # Returns
///
/// * `Some(&str)` - The adapter ID (e.g., `"claude"`, `"codex"`, `"gemini"`)
/// * `None` - For [`CliType::Custom`], which has no predefined adapter
fn cli_type_to_id(cli_type: CliType) -> Option<&'static str> {
    match cli_type {
        CliType::Claude => Some("claude"),
        CliType::Codex => Some("codex"),
        CliType::Gemini => Some("gemini"),
        CliType::Custom => None,
    }
}

/// Maps a [`CliType`] to its corresponding adapter ID for REPL/terminal mode.
///
/// This appends [`DEFAULT_TERMINAL_SUFFIX`] to the base adapter ID.
///
/// # Arguments
///
/// * `cli_type` - The CLI type to map
///
/// # Returns
///
/// * `Some(String)` - The REPL adapter ID (e.g., `"claude-terminal"`)
/// * `None` - For [`CliType::Custom`], which has no predefined adapter
fn cli_type_to_terminal_id(cli_type: CliType) -> Option<String> {
    cli_type_to_id(cli_type).map(|id| format!("{}{}", id, DEFAULT_TERMINAL_SUFFIX))
}

/// Central registry for managing agent adapters.
///
/// The `AgentRegistry` provides a unified interface for accessing agent adapters
/// that implement the [`AgentRunner`] trait. It supports both print mode (batch
/// execution) and REPL mode (interactive terminal) adapters.
///
/// # Design
///
/// The registry uses a dual-map design:
/// - Print mode adapters are stored by their base ID (e.g., `"claude"`)
/// - REPL mode adapters are stored with a terminal suffix (e.g., `"claude-terminal"`)
///
/// This separation allows the same CLI type to have different execution strategies.
///
/// # Thread Safety
///
/// The registry is `Clone` and stores adapters as `Arc<dyn AgentRunner>`,
/// making it safe to share across threads.
///
/// # Example
///
/// ```rust,ignore
/// let registry = AgentRegistry::with_defaults();
///
/// // List all available adapters (installed on system)
/// for id in registry.list_available() {
///     println!("Available: {}", id);
/// }
///
/// // Check if a specific adapter is available
/// if registry.is_available("claude") {
///     let adapter = registry.get("claude").unwrap();
///     // Use adapter...
/// }
/// ```
#[derive(Clone)]
pub struct AgentRegistry {
    /// Print mode adapters indexed by agent ID (e.g., `"claude"` → `ClaudeAdapter`).
    adapters: HashMap<String, Arc<dyn AgentRunner>>,

    /// REPL mode adapters indexed by terminal ID (e.g., `"claude-terminal"` → `TerminalAdapter`).
    repl_adapters: HashMap<String, Arc<dyn AgentRunner>>,
}

impl AgentRegistry {
    /// Creates a new empty registry with no adapters registered.
    ///
    /// Use [`with_defaults()`](Self::with_defaults) to create a registry
    /// pre-populated with the standard adapters.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut registry = AgentRegistry::new();
    /// // Registry is empty, register adapters manually
    /// registry.register(Arc::new(ClaudeAdapter::new()));
    /// ```
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
            repl_adapters: HashMap::new(),
        }
    }

    /// Creates a registry pre-populated with all default adapters.
    ///
    /// # Registered Adapters
    ///
    /// **Print mode:**
    /// - `"claude"` → [`ClaudeAdapter`]
    /// - `"codex"` → [`CodexAdapter`]
    /// - `"gemini"` → [`GeminiAdapter`]
    ///
    /// **REPL mode:**
    /// - `"claude-terminal"` → [`TerminalAdapter`] (Claude)
    /// - `"codex-terminal"` → [`TerminalAdapter`] (Codex)
    /// - `"gemini-terminal"` → [`TerminalAdapter`] (Gemini)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let registry = AgentRegistry::with_defaults();
    /// assert!(registry.get("claude").is_some());
    /// assert!(registry.get_repl("claude-terminal").is_some());
    /// ```
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

    /// Registers a print mode adapter.
    ///
    /// The adapter is stored using the ID returned by [`AgentRunner::id()`].
    /// If an adapter with the same ID already exists, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `adapter` - The adapter to register, wrapped in `Arc` for shared ownership
    pub fn register(&mut self, adapter: Arc<dyn AgentRunner>) {
        self.adapters.insert(adapter.id().to_string(), adapter);
    }

    /// Registers a REPL mode adapter.
    ///
    /// The adapter is stored using the ID returned by [`AgentRunner::id()`].
    /// By convention, REPL adapter IDs should end with [`DEFAULT_TERMINAL_SUFFIX`].
    ///
    /// # Arguments
    ///
    /// * `adapter` - The adapter to register, wrapped in `Arc` for shared ownership
    pub fn register_repl(&mut self, adapter: Arc<dyn AgentRunner>) {
        self.repl_adapters.insert(adapter.id().to_string(), adapter);
    }

    /// Retrieves a print mode adapter by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The adapter ID (e.g., `"claude"`, `"codex"`, `"gemini"`)
    ///
    /// # Returns
    ///
    /// * `Some(Arc<dyn AgentRunner>)` - The adapter if found
    /// * `None` - If no adapter with that ID is registered
    pub fn get(&self, id: &str) -> Option<Arc<dyn AgentRunner>> {
        self.adapters.get(id).cloned()
    }

    /// Retrieves a REPL mode adapter by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The terminal adapter ID (e.g., `"claude-terminal"`)
    ///
    /// # Returns
    ///
    /// * `Some(Arc<dyn AgentRunner>)` - The adapter if found
    /// * `None` - If no adapter with that ID is registered
    pub fn get_repl(&self, id: &str) -> Option<Arc<dyn AgentRunner>> {
        self.repl_adapters.get(id).cloned()
    }

    /// Retrieves a print mode adapter for a given CLI type.
    ///
    /// # Arguments
    ///
    /// * `cli_type` - The CLI type to look up
    ///
    /// # Returns
    ///
    /// * `Some(Arc<dyn AgentRunner>)` - The adapter for the CLI type
    /// * `None` - For [`CliType::Custom`] or if not registered
    pub fn get_for_cli_type(&self, cli_type: CliType) -> Option<Arc<dyn AgentRunner>> {
        cli_type_to_id(cli_type).and_then(|id| self.get(id))
    }

    /// Retrieves a REPL mode adapter for a given CLI type.
    ///
    /// # Arguments
    ///
    /// * `cli_type` - The CLI type to look up
    ///
    /// # Returns
    ///
    /// * `Some(Arc<dyn AgentRunner>)` - The terminal adapter for the CLI type
    /// * `None` - For [`CliType::Custom`] or if not registered
    pub fn get_repl_for_cli_type(&self, cli_type: CliType) -> Option<Arc<dyn AgentRunner>> {
        cli_type_to_terminal_id(cli_type).and_then(|id| self.get_repl(&id))
    }

    /// Retrieves an adapter appropriate for the given agent configuration.
    ///
    /// This is the primary method for obtaining an adapter, as it respects
    /// the agent's mode setting ([`AgentMode::Print`] vs [`AgentMode::Repl`]).
    ///
    /// # Lookup Strategy
    ///
    /// 1. **Print mode**: First tries the config's `id`, then falls back to `cli_type`
    /// 2. **REPL mode**: First tries `{id}-terminal`, then falls back to `cli_type` terminal
    ///
    /// # Arguments
    ///
    /// * `config` - The agent configuration containing ID, CLI type, and mode
    ///
    /// # Returns
    ///
    /// * `Some(Arc<dyn AgentRunner>)` - The appropriate adapter
    /// * `None` - If no matching adapter is found
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let registry = AgentRegistry::with_defaults();
    /// let config = AgentConfig {
    ///     id: "claude".to_string(),
    ///     cli_type: CliType::Claude,
    ///     mode: AgentMode::Print,
    ///     ..Default::default()
    /// };
    ///
    /// let adapter = registry.get_for_config(&config).unwrap();
    /// assert_eq!(adapter.id(), "claude");
    /// ```
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

    /// Lists all available print mode adapters.
    ///
    /// An adapter is considered "available" if its underlying CLI binary
    /// is installed and accessible on the system (as determined by
    /// [`AgentRunner::is_available()`]).
    ///
    /// # Returns
    ///
    /// A vector of adapter IDs that are both registered and available.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let registry = AgentRegistry::with_defaults();
    /// let available = registry.list_available();
    /// // Might return ["claude", "gemini"] if only those are installed
    /// ```
    pub fn list_available(&self) -> Vec<&str> {
        self.adapters
            .iter()
            .filter(|(_, adapter)| adapter.is_available())
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Lists all registered print mode adapter IDs.
    ///
    /// Unlike [`list_available()`](Self::list_available), this returns all
    /// registered adapters regardless of whether their CLI is installed.
    ///
    /// # Returns
    ///
    /// A vector of all registered adapter IDs.
    pub fn list_all(&self) -> Vec<&str> {
        self.adapters.keys().map(|s| s.as_str()).collect()
    }

    /// Checks if a print mode adapter is available.
    ///
    /// # Arguments
    ///
    /// * `id` - The adapter ID to check
    ///
    /// # Returns
    ///
    /// * `true` - If the adapter is registered AND its CLI is installed
    /// * `false` - If the adapter is not registered or CLI is not found
    pub fn is_available(&self, id: &str) -> bool {
        self.adapters
            .get(id)
            .map(|a| a.is_available())
            .unwrap_or(false)
    }
}

impl Default for AgentRegistry {
    /// Creates a default registry using [`with_defaults()`](Self::with_defaults).
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
