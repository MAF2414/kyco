//! Autocomplete suggestions for modes and agents

use crate::config::Config;

/// Autocomplete suggestion
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub text: String,
    pub description: String,
    pub category: &'static str,
}

/// State for autocomplete functionality
pub struct AutocompleteState {
    /// Current suggestions
    pub suggestions: Vec<Suggestion>,
    /// Selected suggestion index
    pub selected_suggestion: usize,
    /// Whether to show suggestions dropdown
    pub show_suggestions: bool,
    /// Request cursor to move to end of input
    pub cursor_to_end: bool,
}

impl Default for AutocompleteState {
    fn default() -> Self {
        Self {
            suggestions: Vec::new(),
            selected_suggestion: 0,
            show_suggestions: false,
            cursor_to_end: false,
        }
    }
}

impl AutocompleteState {
    /// Update autocomplete suggestions based on input
    pub fn update_suggestions(&mut self, input: &str, config: &Config) {
        self.suggestions.clear();
        self.selected_suggestion = 0;

        let input_lower = input.to_lowercase();
        let input_trimmed = input_lower.trim();

        // Get default agent from config
        let default_agent = &config.settings.gui.default_agent;

        if input_trimmed.is_empty() {
            // Show agents first, then modes when empty
            for (agent_name, agent_config) in &config.agent {
                let desc = format!(
                    "{} ({})",
                    agent_config.binary,
                    agent_config.aliases.join(", ")
                );
                self.suggestions.push(Suggestion {
                    text: format!("{}:", agent_name),
                    description: desc,
                    category: "agent",
                });
            }
            for (mode_name, mode_config) in &config.mode {
                let aliases = if mode_config.aliases.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", mode_config.aliases.join(", "))
                };
                let agent_hint = mode_config.agent.as_deref().unwrap_or(default_agent);
                self.suggestions.push(Suggestion {
                    text: mode_name.to_string(),
                    description: format!("default: {}{}", agent_hint, aliases),
                    category: "mode",
                });
            }
            self.show_suggestions = true;
            return;
        }

        // Check if we have "agent:" prefix - show modes after colon
        if let Some(colon_pos) = input_trimmed.find(':') {
            let agent_part = &input_trimmed[..colon_pos];
            let mode_part = &input_trimmed[colon_pos + 1..];

            // After colon, show matching modes
            for (mode_name, mode_config) in &config.mode {
                let mode_lower = mode_name.to_lowercase();
                let matches_mode = mode_lower.starts_with(mode_part) || mode_part.is_empty();
                let matches_alias = mode_config
                    .aliases
                    .iter()
                    .any(|a| a.to_lowercase().starts_with(mode_part));

                if matches_mode || matches_alias {
                    let aliases = if mode_config.aliases.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", mode_config.aliases.join(", "))
                    };
                    self.suggestions.push(Suggestion {
                        text: format!("{}:{}", agent_part, mode_name),
                        description: aliases,
                        category: "mode",
                    });
                }
            }
        } else {
            // No colon yet - show matching agents and modes
            // First show matching agents (by name or alias)
            for (agent_name, agent_config) in &config.agent {
                let name_lower = agent_name.to_lowercase();
                let matches_name = name_lower.starts_with(input_trimmed);
                let matches_alias = agent_config
                    .aliases
                    .iter()
                    .any(|a| a.to_lowercase().starts_with(input_trimmed));

                if matches_name || matches_alias {
                    let desc = format!(
                        "{} ({})",
                        agent_config.binary,
                        agent_config.aliases.join(", ")
                    );
                    self.suggestions.push(Suggestion {
                        text: format!("{}:", agent_name),
                        description: desc,
                        category: "agent",
                    });
                }
            }

            // Then show matching modes (uses default agent)
            for (mode_name, mode_config) in &config.mode {
                let mode_lower = mode_name.to_lowercase();
                let matches_mode = mode_lower.starts_with(input_trimmed);
                let matches_alias = mode_config
                    .aliases
                    .iter()
                    .any(|a| a.to_lowercase().starts_with(input_trimmed));

                if matches_mode || matches_alias {
                    let aliases = if mode_config.aliases.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", mode_config.aliases.join(", "))
                    };
                    let agent_hint = mode_config.agent.as_deref().unwrap_or(default_agent);
                    self.suggestions.push(Suggestion {
                        text: mode_name.to_string(),
                        description: format!("default: {}{}", agent_hint, aliases),
                        category: "mode",
                    });
                }
            }
        }

        self.show_suggestions = !self.suggestions.is_empty();
    }

    /// Apply selected suggestion to the input
    /// Returns the new input string
    pub fn apply_suggestion(&mut self, _current_input: &str) -> Option<String> {
        if let Some(suggestion) = self.suggestions.get(self.selected_suggestion) {
            let new_input = if suggestion.text.ends_with(':') {
                suggestion.text.clone()
            } else {
                format!("{} ", suggestion.text)
            };
            self.show_suggestions = false;
            self.cursor_to_end = true;
            Some(new_input)
        } else {
            None
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected_suggestion = (self.selected_suggestion + 1) % self.suggestions.len();
        }
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.selected_suggestion > 0 {
            self.selected_suggestion -= 1;
        }
    }
}

/// Parse the popup input into agent, mode, and prompt
pub fn parse_input(input: &str) -> (String, String, String) {
    let input = input.trim();

    let (command, prompt) = match input.find(' ') {
        Some(pos) => (&input[..pos], input[pos + 1..].trim()),
        None => (input, ""),
    };

    let (agent, mode) = match command.find(':') {
        Some(pos) => (&command[..pos], &command[pos + 1..]),
        None => ("claude", command),
    };

    (agent.to_string(), mode.to_string(), prompt.to_string())
}
