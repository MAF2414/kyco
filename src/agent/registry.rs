//! Agent registry for available agent adapters.
//!
//! Agents run via the SDK Bridge (Node.js sidecar) for full SDK control.

use std::collections::HashMap;
use std::sync::Arc;

use crate::{AgentConfig, SdkType};

use super::bridge::{ClaudeBridgeAdapter, CodexBridgeAdapter};
use super::runner::AgentRunner;

/// Central registry for managing agent adapters.
///
/// The `AgentRegistry` provides a unified interface for accessing agent adapters
/// that implement the [`AgentRunner`] trait.
///
/// # Thread Safety
///
/// The registry is `Clone` and stores adapters as `Arc<dyn AgentRunner>`,
/// making it safe to share across threads.
#[derive(Clone)]
pub struct AgentRegistry {
    /// Adapters indexed by agent ID (e.g., `"claude"` → `ClaudeAdapter`).
    adapters: HashMap<String, Arc<dyn AgentRunner>>,
}

impl AgentRegistry {
    /// Creates a new registry with SDK Bridge adapters.
    pub fn new() -> Self {
        let mut adapters: HashMap<String, Arc<dyn AgentRunner>> = HashMap::new();

        adapters.insert("claude".to_string(), Arc::new(ClaudeBridgeAdapter::new()));
        adapters.insert("codex".to_string(), Arc::new(CodexBridgeAdapter::new()));

        Self { adapters }
    }

    /// Creates a registry with custom adapters (for testing or extension).
    pub fn with_adapters(adapters: HashMap<String, Arc<dyn AgentRunner>>) -> Self {
        Self { adapters }
    }

    /// Retrieves an adapter by its ID (e.g., `"claude"`, `"codex"`).
    pub fn get(&self, id: &str) -> Option<Arc<dyn AgentRunner>> {
        self.adapters.get(id).cloned()
    }

    /// Retrieves an adapter for a given SDK type.
    pub fn get_for_sdk_type(&self, sdk_type: SdkType) -> Option<Arc<dyn AgentRunner>> {
        let id = match sdk_type {
            SdkType::Custom => "claude",
            _ => sdk_type.default_name(),
        };
        self.get(id)
    }

    /// Retrieves an adapter appropriate for the given agent configuration.
    ///
    /// Tries by ID first, then falls back to SDK type.
    pub fn get_for_config(&self, config: &AgentConfig) -> Option<Arc<dyn AgentRunner>> {
        if let Some(adapter) = self.get(&config.id) {
            return Some(adapter);
        }
        self.get_for_sdk_type(config.sdk_type)
    }

    /// Lists all available adapters.
    ///
    /// Availability is determined by each adapter (e.g., CLI binary present).
    pub fn list_available(&self) -> Vec<&str> {
        self.adapters
            .iter()
            .filter(|(_, a)| a.is_available())
            .map(|(k, _)| k.as_str())
            .collect()
    }

    /// Lists all registered adapter IDs.
    pub fn list_all(&self) -> Vec<&str> {
        self.adapters.keys().map(|s| s.as_str()).collect()
    }

    /// Checks if an adapter is registered.
    pub fn is_available(&self, id: &str) -> bool {
        self.adapters.contains_key(id)
    }

    // Legacy compatibility methods

    /// Legacy: Get adapter for CLI type (alias for get_for_sdk_type)
    pub fn get_for_cli_type(&self, cli_type: SdkType) -> Option<Arc<dyn AgentRunner>> {
        self.get_for_sdk_type(cli_type)
    }

    /// Legacy: Get REPL adapter (returns same as regular adapter for SDK)
    pub fn get_repl(&self, id: &str) -> Option<Arc<dyn AgentRunner>> {
        // For SDK adapters, there's no separate REPL mode - sessions handle continuation
        // Strip "-terminal" suffix if present for backwards compatibility
        let clean_id = id.strip_suffix("-terminal").unwrap_or(id);
        self.get(clean_id)
    }

    /// Legacy: Get REPL adapter for CLI type
    pub fn get_repl_for_cli_type(&self, cli_type: SdkType) -> Option<Arc<dyn AgentRunner>> {
        self.get_for_sdk_type(cli_type)
    }

    /// Legacy: Register adapter (for backwards compatibility)
    pub fn register(&mut self, adapter: Arc<dyn AgentRunner>) {
        self.adapters.insert(adapter.id().to_string(), adapter);
    }

    /// Legacy: Register REPL adapter (no-op for SDK adapters)
    pub fn register_repl(&mut self, _adapter: Arc<dyn AgentRunner>) {
        // No-op: SDK adapters don't have separate REPL mode
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Legacy function for backwards compatibility
#[allow(dead_code)]
pub fn cli_type_to_id(cli_type: SdkType) -> Option<&'static str> {
    Some(cli_type.default_name())
}

// Legacy constant
pub const DEFAULT_TERMINAL_SUFFIX: &str = "-terminal";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = AgentRegistry::new();

        assert!(registry.get("claude").is_some());
        assert!(registry.get("codex").is_some());
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn test_get_for_sdk_type() {
        let registry = AgentRegistry::new();

        assert!(registry.get_for_sdk_type(SdkType::Claude).is_some());
        assert!(registry.get_for_sdk_type(SdkType::Codex).is_some());
    }

    #[test]
    fn test_list_available() {
        let registry = AgentRegistry::new();
        let all = registry.list_all();

        assert!(all.contains(&"claude"));
        assert!(all.contains(&"codex"));
    }
}
