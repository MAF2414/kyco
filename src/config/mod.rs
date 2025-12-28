//! Configuration loading and management

mod agent;
mod alias;
mod chain;
mod internal;
mod io;
mod lookup;
mod mode;
mod scope;
mod settings;
mod target;
mod token;

pub use agent::AgentConfigToml;
pub use alias::AliasConfig;
pub use chain::{ChainStep, ModeChain, ModeOrChain, StateDefinition};
pub use internal::{InternalDefaults, INTERNAL_DEFAULTS_TOML};
pub use mode::{ClaudeModeOptions, CodexModeOptions, ModeConfig, ModeSessionType};
pub use scope::ScopeConfig;
pub use settings::{
    default_orchestrator_system_prompt, GuiSettings, OrchestratorSettings, RegistrySettings,
    Settings, VoiceSettings,
};
pub use target::TargetConfig;
pub use token::generate_http_token;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Agent configurations
    #[serde(default)]
    pub agent: HashMap<String, AgentConfigToml>,

    /// Mode configurations (prompt builders)
    #[serde(default)]
    pub mode: HashMap<String, ModeConfig>,

    /// Mode chain configurations (sequential mode execution)
    #[serde(default)]
    pub chain: HashMap<String, ModeChain>,

    /// Scope configurations
    #[serde(default)]
    pub scope: HashMap<String, ScopeConfig>,

    /// Target configurations
    #[serde(default)]
    pub target: HashMap<String, TargetConfig>,

    /// Alias configurations
    #[serde(default)]
    pub alias: AliasConfig,

    /// General settings
    #[serde(default)]
    pub settings: Settings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent: HashMap::new(),
            mode: HashMap::new(),
            chain: HashMap::new(),
            scope: HashMap::new(),
            target: HashMap::new(),
            alias: AliasConfig::default(),
            settings: Settings::default(),
        }
    }
}

impl Config {
    /// Merge internal defaults into this config using versioned merging.
    ///
    /// Returns true if any changes were made (new items added or upgraded).
    pub fn merge_internal_defaults(&mut self) -> bool {
        let internal = match internal::InternalDefaults::load() {
            Ok(defaults) => defaults,
            Err(e) => {
                tracing::error!("Failed to parse internal defaults: {}", e);
                return false;
            }
        };

        // Track sizes before merge to detect changes
        let agents_before = self.agent.len();
        let modes_before = self.mode.len();
        let chains_before = self.chain.len();

        // Also need to check if any versions were upgraded
        let mut version_changes = false;

        // Check for version upgrades before merge
        for (name, internal_agent) in &internal.agent {
            if let Some(existing) = self.agent.get(name) {
                if internal_agent.version > existing.version {
                    version_changes = true;
                    break;
                }
            }
        }
        if !version_changes {
            for (name, internal_mode) in &internal.mode {
                if let Some(existing) = self.mode.get(name) {
                    if internal_mode.version > existing.version {
                        version_changes = true;
                        break;
                    }
                }
            }
        }
        if !version_changes {
            for (name, internal_chain) in &internal.chain {
                if let Some(existing) = self.chain.get(name) {
                    if internal_chain.version > existing.version {
                        version_changes = true;
                        break;
                    }
                }
            }
        }

        // Perform the merge
        internal.merge_into(&mut self.agent, &mut self.mode, &mut self.chain);

        // Check if anything changed
        let size_changes = self.agent.len() != agents_before
            || self.mode.len() != modes_before
            || self.chain.len() != chains_before;

        size_changes || version_changes
    }
}
