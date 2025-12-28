//! Internal default configurations embedded at compile-time
//!
//! This module embeds the built-in modes, chains, and agents from
//! `assets/internal/defaults.toml` and provides versioned merging
//! into user configurations.

use std::collections::HashMap;

use serde::Deserialize;

use super::{AgentConfigToml, ModeChain, ModeConfig};

/// Embedded defaults TOML content (compile-time)
pub const INTERNAL_DEFAULTS_TOML: &str = include_str!("../../assets/internal/defaults.toml");

/// Internal defaults structure matching the TOML format
#[derive(Debug, Clone, Deserialize)]
pub struct InternalDefaults {
    #[serde(default)]
    pub agent: HashMap<String, AgentConfigToml>,
    #[serde(default)]
    pub mode: HashMap<String, ModeConfig>,
    #[serde(default)]
    pub chain: HashMap<String, ModeChain>,
}

impl InternalDefaults {
    /// Parse the embedded defaults TOML
    pub fn load() -> Result<Self, toml::de::Error> {
        toml::from_str(INTERNAL_DEFAULTS_TOML)
    }

    /// Merge internal defaults into a config, respecting versions.
    ///
    /// For each internal mode/chain/agent:
    /// - If it doesn't exist in the target config, add it
    /// - If it exists but the internal version is higher, replace it
    /// - If it exists with same or higher version, keep the user's version
    pub fn merge_into(
        &self,
        agents: &mut HashMap<String, AgentConfigToml>,
        modes: &mut HashMap<String, ModeConfig>,
        chains: &mut HashMap<String, ModeChain>,
    ) {
        // Merge agents
        for (name, internal_agent) in &self.agent {
            match agents.get(name) {
                Some(existing) if existing.version >= internal_agent.version => {
                    // User has same or newer version, keep it
                }
                _ => {
                    // Add or upgrade
                    agents.insert(name.clone(), internal_agent.clone());
                }
            }
        }

        // Merge modes
        for (name, internal_mode) in &self.mode {
            match modes.get(name) {
                Some(existing) if existing.version >= internal_mode.version => {
                    // User has same or newer version, keep it
                }
                _ => {
                    // Add or upgrade
                    modes.insert(name.clone(), internal_mode.clone());
                }
            }
        }

        // Merge chains
        for (name, internal_chain) in &self.chain {
            match chains.get(name) {
                Some(existing) if existing.version >= internal_chain.version => {
                    // User has same or newer version, keep it
                }
                _ => {
                    // Add or upgrade
                    chains.insert(name.clone(), internal_chain.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_internal_defaults() {
        let defaults = InternalDefaults::load().expect("Failed to parse internal defaults");

        // Should have agents
        assert!(defaults.agent.contains_key("claude"));
        assert!(defaults.agent.contains_key("codex"));

        // Should have modes
        assert!(defaults.mode.contains_key("review"));
        assert!(defaults.mode.contains_key("fix"));
        assert!(defaults.mode.contains_key("implement"));

        // Should have chains
        assert!(defaults.chain.contains_key("review+fix"));
    }

    #[test]
    fn test_merge_respects_versions() {
        let defaults = InternalDefaults::load().expect("Failed to parse internal defaults");

        let mut agents = HashMap::new();
        let mut modes = HashMap::new();
        let mut chains = HashMap::new();

        // First merge - should add all
        defaults.merge_into(&mut agents, &mut modes, &mut chains);
        assert!(!agents.is_empty());
        assert!(!modes.is_empty());

        // User customizes a mode with higher version
        if let Some(review) = modes.get_mut("review") {
            review.version = 999;
            review.system_prompt = Some("Custom review prompt".to_string());
        }

        // Second merge - should NOT override user's higher version
        defaults.merge_into(&mut agents, &mut modes, &mut chains);
        let review = modes.get("review").unwrap();
        assert_eq!(review.version, 999);
        assert_eq!(
            review.system_prompt,
            Some("Custom review prompt".to_string())
        );
    }
}
