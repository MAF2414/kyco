//! Internal default configurations embedded at compile-time
//!
//! This module embeds the built-in chains and agents from
//! `assets/internal/defaults.toml` and provides versioned merging
//! into user configurations.
//!
//! Note: Skills are loaded from SKILL.md files in `.claude/skills/`,
//! `.codex/skills/`, or `~/.kyco/skills/` - not from this file.

use std::collections::HashMap;

use serde::Deserialize;

use super::{AgentConfigToml, ModeChain};

/// Embedded defaults TOML content (compile-time)
pub const INTERNAL_DEFAULTS_TOML: &str = include_str!("../../assets/internal/defaults.toml");

/// Internal defaults structure matching the TOML format
#[derive(Debug, Clone, Deserialize)]
pub struct InternalDefaults {
    #[serde(default)]
    pub agent: HashMap<String, AgentConfigToml>,
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
    /// For each internal chain/agent:
    /// - If it doesn't exist in the target config, add it
    /// - If it exists but the internal version is higher, replace it
    /// - If it exists with same or higher version, keep the user's version
    ///
    /// Note: Skills are not merged here - they are loaded from SKILL.md files.
    pub fn merge_into(
        &self,
        agents: &mut HashMap<String, AgentConfigToml>,
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

        // Built-in chains ship with KYCo (e.g. bugbounty/security pipelines).
        // Skills are loaded from SKILL.md files, not from defaults.
        assert!(defaults.chain.contains_key("audit-file"));
        assert!(defaults.chain.contains_key("audit-project"));
    }

    #[test]
    fn test_merge_respects_versions() {
        let defaults = InternalDefaults::load().expect("Failed to parse internal defaults");

        let mut agents = HashMap::new();
        let mut chains = HashMap::new();

        // First merge - should add agents + internal chains
        defaults.merge_into(&mut agents, &mut chains);
        assert!(!agents.is_empty());
        assert!(chains.contains_key("audit-file"));
        assert!(chains.contains_key("audit-project"));

        // User customizes an agent with higher version
        if let Some(claude) = agents.get_mut("claude") {
            claude.version = 999;
        }

        // Second merge - should NOT override user's higher version
        defaults.merge_into(&mut agents, &mut chains);
        let claude = agents.get("claude").unwrap();
        assert_eq!(claude.version, 999);
    }
}
