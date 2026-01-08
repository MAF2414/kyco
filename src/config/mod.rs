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
mod skill;
mod skill_discovery;
mod skill_parser;
mod skill_registry;
mod target;
mod token;

pub use agent::AgentConfigToml;
pub use alias::AliasConfig;
pub use chain::{ChainStep, ModeChain, ModeOrChain, ModeOrChainRef, StateDefinition};
pub use internal::{InternalDefaults, INTERNAL_DEFAULTS_TOML};
pub use mode::{ClaudeModeOptions, CodexModeOptions, ModeConfig, ModeSessionType};
pub use scope::ScopeConfig;
pub use skill::{
    ClaudeSkillOptions, CodexSkillOptions, SkillConfig, SkillKycoExtensions, SkillSessionType,
    SkillValidationError, validate_skill, validate_skill_description, validate_skill_name,
};
pub use skill_discovery::{
    delete_skill, delete_skill_global, save_skill, save_skill_global, SkillAgentType,
    SkillDiscovery,
};
pub use skill_parser::{create_skill_template, parse_skill_content, parse_skill_file, SkillParseError};
pub use skill_registry::{RegistrySkill, SkillRegistry};
pub use lookup::SkillOrChainRef;
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

    /// Mode configurations (prompt builders) - DEPRECATED, use skills instead
    #[serde(default)]
    pub mode: HashMap<String, ModeConfig>,

    /// Skill configurations (loaded from .claude/skills/ and .codex/skills/)
    /// This is computed at runtime from SKILL.md files, not stored in config.toml
    #[serde(skip)]
    pub skill: HashMap<String, SkillConfig>,

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
            skill: HashMap::new(),
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
    ///
    /// Note: Skills are loaded from SKILL.md files, not merged from internal defaults.
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
            for (name, internal_chain) in &internal.chain {
                if let Some(existing) = self.chain.get(name) {
                    if internal_chain.version > existing.version {
                        version_changes = true;
                        break;
                    }
                }
            }
        }

        // Perform the merge (skills are loaded from SKILL.md files, not here)
        internal.merge_into(&mut self.agent, &mut self.chain);

        // Check if anything changed
        let size_changes =
            self.agent.len() != agents_before || self.chain.len() != chains_before;

        size_changes || version_changes
    }
}
