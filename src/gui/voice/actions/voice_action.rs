//! VoiceAction and WakewordMatch types
//!
//! This module defines the voice action struct and wakeword matching logic.

use serde::{Deserialize, Serialize};

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

        let all_triggers: Vec<&str> = self
            .wakewords
            .iter()
            .map(|s| s.as_str())
            .chain(self.aliases.iter().map(|s| s.as_str()))
            .collect();

        for trigger in all_triggers {
            let trigger_lower = trigger.to_lowercase();

            if text_trimmed.starts_with(&trigger_lower) {
                // Safe unicode slicing: count characters in trigger, then find the byte
                // position after that many characters in the original text.
                // This handles cases where lowercase/uppercase have different byte lengths.
                let trigger_char_count = trigger_lower.chars().count();
                let rest_start_byte = text
                    .char_indices()
                    .nth(trigger_char_count)
                    .map(|(idx, _)| idx)
                    .unwrap_or(text.len());
                let rest = text[rest_start_byte..].trim();

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
