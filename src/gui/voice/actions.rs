//! Voice Actions - Mapping wakewords to modes and actions
//!
//! This module defines how spoken keywords trigger specific modes or actions.
//! Users can configure custom wakewords that map to different modes.
//!
//! Example configurations:
//! - "refactor" -> triggers refactor mode
//! - "hey kyco fix" -> triggers fix mode
//! - "make tests" -> triggers tests mode

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A voice action that maps a wakeword to a mode or custom action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceAction {
    /// The wakeword(s) that trigger this action (case-insensitive)
    /// Can be a single word or phrase
    pub wakewords: Vec<String>,

    /// The mode to trigger (e.g., "refactor", "fix", "tests")
    pub mode: String,

    /// Optional: Custom prompt template override
    /// Use {prompt} as placeholder for the rest of the voice input
    pub prompt_template: Option<String>,

    /// Whether this action requires additional prompt after the wakeword
    /// If false, the action triggers immediately on wakeword
    pub requires_prompt: bool,

    /// Optional: Aliases for this action (e.g., "r" for refactor)
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Whether this action is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl VoiceAction {
    /// Create a new voice action
    pub fn new(wakeword: impl Into<String>, mode: impl Into<String>) -> Self {
        Self {
            wakewords: vec![wakeword.into()],
            mode: mode.into(),
            prompt_template: None,
            requires_prompt: true,
            aliases: Vec::new(),
            enabled: true,
        }
    }

    /// Add an additional wakeword
    pub fn with_wakeword(mut self, wakeword: impl Into<String>) -> Self {
        self.wakewords.push(wakeword.into());
        self
    }

    /// Set a custom prompt template
    pub fn with_prompt_template(mut self, template: impl Into<String>) -> Self {
        self.prompt_template = Some(template.into());
        self
    }

    /// Set whether this action requires a prompt
    pub fn requires_prompt(mut self, requires: bool) -> Self {
        self.requires_prompt = requires;
        self
    }

    /// Add an alias
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }

    /// Check if a text matches any of this action's wakewords
    pub fn matches(&self, text: &str) -> Option<WakewordMatch> {
        if !self.enabled {
            return None;
        }

        let text_lower = text.to_lowercase();
        let text_trimmed = text_lower.trim();

        // Check all wakewords and aliases
        let all_triggers: Vec<&str> = self
            .wakewords
            .iter()
            .map(|s| s.as_str())
            .chain(self.aliases.iter().map(|s| s.as_str()))
            .collect();

        for trigger in all_triggers {
            let trigger_lower = trigger.to_lowercase();

            if text_trimmed.starts_with(&trigger_lower) {
                // Extract the rest of the text after the wakeword
                let rest = text[trigger_lower.len()..].trim();

                return Some(WakewordMatch {
                    wakeword: trigger.to_string(),
                    mode: self.mode.clone(),
                    prompt: rest.to_string(),
                    prompt_template: self.prompt_template.clone(),
                    requires_prompt: self.requires_prompt,
                });
            }
        }

        None
    }
}

/// Result of a wakeword match
#[derive(Debug, Clone)]
pub struct WakewordMatch {
    /// The wakeword that was matched
    pub wakeword: String,
    /// The mode to trigger
    pub mode: String,
    /// The rest of the text after the wakeword (the prompt)
    pub prompt: String,
    /// Optional custom prompt template
    pub prompt_template: Option<String>,
    /// Whether this action requires additional prompt
    pub requires_prompt: bool,
}

impl WakewordMatch {
    /// Get the final prompt text
    /// If a template is set, applies the template
    pub fn get_final_prompt(&self) -> String {
        if let Some(ref template) = self.prompt_template {
            template.replace("{prompt}", &self.prompt)
        } else {
            self.prompt.clone()
        }
    }

    /// Check if this match is complete (has prompt if required)
    pub fn is_complete(&self) -> bool {
        !self.requires_prompt || !self.prompt.is_empty()
    }
}

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
                // Remove prefix and trim
                text[prefix_lower.len()..].trim()
            } else {
                // Doesn't start with prefix, no match
                return None;
            }
        } else {
            text
        };

        // Try to match against all actions
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_action_match() {
        let action = VoiceAction::new("refactor", "refactor").with_alias("r");

        let m = action.matches("refactor this function").unwrap();
        assert_eq!(m.mode, "refactor");
        assert_eq!(m.prompt, "this function");

        let m = action.matches("r the code").unwrap();
        assert_eq!(m.mode, "refactor");
        assert_eq!(m.prompt, "the code");

        assert!(action.matches("fix something").is_none());
    }

    #[test]
    fn test_registry_with_prefix() {
        let mut registry = VoiceActionRegistry::default();
        registry.set_global_prefix("hey kyco");

        // Should match with prefix
        let m = registry.match_text("hey kyco refactor this").unwrap();
        assert_eq!(m.mode, "refactor");
        assert_eq!(m.prompt, "this");

        // Should not match without prefix
        assert!(registry.match_text("refactor this").is_none());
    }

    #[test]
    fn test_german_aliases() {
        let registry = VoiceActionRegistry::default();

        let m = registry.match_text("überarbeite diese Funktion").unwrap();
        assert_eq!(m.mode, "refactor");

        let m = registry.match_text("erkläre diesen Code").unwrap();
        assert_eq!(m.mode, "explain");
    }

    #[test]
    fn test_prompt_template() {
        let action = VoiceAction::new("quick fix", "fix")
            .with_prompt_template("Quickly fix this issue: {prompt}");

        let m = action.matches("quick fix the bug").unwrap();
        assert_eq!(m.get_final_prompt(), "Quickly fix this issue: the bug");
    }
}
