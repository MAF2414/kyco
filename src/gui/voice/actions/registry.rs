//! VoiceActionRegistry - Registry for voice actions
//!
//! This module defines the registry that manages voice actions and matches
//! spoken text against registered wakewords.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::voice_action::{VoiceAction, WakewordMatch};

/// Registry of voice actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceActionRegistry {
    /// All registered voice actions
    pub actions: Vec<VoiceAction>,

    /// Global wakeword prefix (e.g., "hey kyco")
    /// If set, all wakewords must be prefixed with this
    #[serde(default)]
    pub global_prefix: Option<String>,
}

impl Default for VoiceActionRegistry {
    fn default() -> Self {
        Self {
            actions: vec![
                VoiceAction::new("refactor", "refactor")
                    .with_alias("r")
                    .with_alias("überarbeite"),
                VoiceAction::new("fix", "fix")
                    .with_alias("f")
                    .with_alias("repariere")
                    .with_alias("fixen"),
                VoiceAction::new("tests", "tests")
                    .with_alias("test")
                    .with_alias("teste"),
                VoiceAction::new("docs", "docs")
                    .with_alias("documentation")
                    .with_alias("dokumentiere"),
                VoiceAction::new("review", "review")
                    .with_alias("überprüfe")
                    .with_alias("check"),
                VoiceAction::new("optimize", "optimize")
                    .with_alias("optimiere")
                    .with_alias("performance"),
                VoiceAction::new("implement", "implement")
                    .with_alias("implementiere")
                    .with_alias("create")
                    .with_alias("erstelle"),
                VoiceAction::new("explain", "explain")
                    .with_alias("erkläre")
                    .with_alias("was macht"),
            ],
            global_prefix: None,
        }
    }
}

impl VoiceActionRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            global_prefix: None,
        }
    }

    /// Create registry from config (modes, chains, agents)
    ///
    /// This dynamically builds voice actions from the available modes and chains
    /// in the configuration, using their aliases as additional wakewords.
    pub fn from_config(
        modes: &HashMap<String, crate::config::ModeConfig>,
        chains: &HashMap<String, crate::config::ModeChain>,
        _agents: &HashMap<String, crate::config::AgentConfigToml>,
    ) -> Self {
        let mut registry = Self::new();

        // Add actions for each mode
        for (mode_name, mode_config) in modes {
            let mut action = VoiceAction::new(mode_name.clone(), mode_name.clone());

            // Add aliases from mode config
            for alias in &mode_config.aliases {
                action = action.with_alias(alias.clone());
            }

            registry.add_action(action);
        }

        // Add actions for each chain
        for (chain_name, _chain_config) in chains {
            // Chains are triggered like modes
            let action = VoiceAction::new(chain_name.clone(), chain_name.clone());
            registry.add_action(action);
        }

        // If no modes/chains configured, use defaults
        if registry.actions.is_empty() {
            return Self::default();
        }

        registry
    }

    /// Add an action to the registry
    pub fn add_action(&mut self, action: VoiceAction) {
        self.actions.push(action);
    }

    /// Set the global prefix
    pub fn set_global_prefix(&mut self, prefix: impl Into<String>) {
        self.global_prefix = Some(prefix.into());
    }

    /// Match text against all registered actions
    pub fn match_text(&self, text: &str) -> Option<WakewordMatch> {
        let text_to_match = if let Some(ref prefix) = self.global_prefix {
            let prefix_lower = prefix.to_lowercase();
            let text_lower = text.to_lowercase();

            if text_lower.starts_with(&prefix_lower) {
                // Safe unicode slicing: count characters in the lowercased prefix,
                // then find the byte position after that many characters in the original text.
                // This handles cases where lowercase/uppercase have different byte lengths.
                let prefix_char_count = prefix_lower.chars().count();
                let rest_start_byte = text
                    .char_indices()
                    .nth(prefix_char_count)
                    .map(|(idx, _)| idx)
                    .unwrap_or(text.len());
                text[rest_start_byte..].trim()
            } else {
                return None;
            }
        } else {
            text
        };

        for action in &self.actions {
            if let Some(m) = action.matches(text_to_match) {
                return Some(m);
            }
        }

        None
    }

    /// Get all wakewords (including aliases) for configuration display
    pub fn get_all_wakewords(&self) -> Vec<String> {
        let mut wakewords = Vec::new();

        for action in &self.actions {
            wakewords.extend(action.wakewords.clone());
            wakewords.extend(action.aliases.clone());
        }

        wakewords
    }

    /// Get actions grouped by mode
    pub fn get_actions_by_mode(&self) -> HashMap<String, Vec<&VoiceAction>> {
        let mut map: HashMap<String, Vec<&VoiceAction>> = HashMap::new();

        for action in &self.actions {
            map.entry(action.mode.clone()).or_default().push(action);
        }

        map
    }
}
