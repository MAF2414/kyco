//! Autocomplete suggestions for modes and agents

mod parsing;

pub use parsing::{count_agents, is_multi_agent_input, parse_input, parse_input_multi};

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

        let default_agent = &config.settings.gui.default_agent;

        if input_trimmed.is_empty() {
            // Show agents first, then skills when empty
            for (agent_name, agent_config) in &config.agent {
                let backend = agent_config.sdk.default_name();
                let desc = format!("{} ({})", backend, agent_config.aliases.join(", "));
                self.suggestions.push(Suggestion {
                    text: format!("{}:", agent_name),
                    description: desc,
                    category: "agent",
                });
            }
            // Skills from filesystem (no legacy modes)
            for (skill_name, skill_config) in &config.skill {
                let aliases = if skill_config.kyco.aliases.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", skill_config.kyco.aliases.join(", "))
                };
                let agent_hint = skill_config.kyco.agent.as_deref().unwrap_or(default_agent);
                self.suggestions.push(Suggestion {
                    text: skill_name.to_string(),
                    description: format!("default: {}{}", agent_hint, aliases),
                    category: "skill",
                });
            }
            for (chain_name, chain_config) in &config.chain {
                let desc = chain_config
                    .description
                    .clone()
                    .unwrap_or_else(|| format!("{} steps", chain_config.steps.len()));
                self.suggestions.push(Suggestion {
                    text: chain_name.to_string(),
                    description: format!("[chain] {}", desc),
                    category: "chain",
                });
            }
            self.show_suggestions = true;
            return;
        }

        // Check for multi-agent mode: input ends with "+"
        if input_trimmed.ends_with('+') && !input_trimmed.contains(':') {
            let prefix = &input_trimmed[..input_trimmed.len() - 1];
            let existing_agents: Vec<&str> = prefix.split('+').collect();

            for (agent_name, _agent_config) in &config.agent {
                // Don't suggest agents already in the list
                if existing_agents
                    .iter()
                    .any(|a| a.eq_ignore_ascii_case(agent_name))
                {
                    continue;
                }

                let desc = format!("Add {} to parallel execution", agent_name);
                self.suggestions.push(Suggestion {
                    text: format!("{}+{}:", input_trimmed.trim_end_matches('+'), agent_name),
                    description: desc,
                    category: "agent",
                });
            }

            // Also suggest finishing with a colon to select mode
            let agent_count = existing_agents.len();
            self.suggestions.push(Suggestion {
                text: format!("{}:", prefix),
                description: format!("{} agents selected - choose mode", agent_count),
                category: "agent",
            });

            self.show_suggestions = !self.suggestions.is_empty();
            return;
        }

        // Check if we have "agent:" prefix - show skills after colon
        if let Some(colon_pos) = input_trimmed.find(':') {
            let agent_part = &input_trimmed[..colon_pos];
            let skill_part = &input_trimmed[colon_pos + 1..];

            // Skills from filesystem (no legacy modes)
            for (skill_name, skill_config) in &config.skill {
                let skill_lower = skill_name.to_lowercase();
                let matches_skill = skill_lower.starts_with(skill_part) || skill_part.is_empty();
                let matches_alias = skill_config
                    .kyco
                    .aliases
                    .iter()
                    .any(|a| a.to_lowercase().starts_with(skill_part));

                if matches_skill || matches_alias {
                    let aliases = if skill_config.kyco.aliases.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", skill_config.kyco.aliases.join(", "))
                    };
                    self.suggestions.push(Suggestion {
                        text: format!("{}:{}", agent_part, skill_name),
                        description: aliases,
                        category: "skill",
                    });
                }
            }

            for (chain_name, chain_config) in &config.chain {
                let chain_lower = chain_name.to_lowercase();
                let matches_chain = chain_lower.starts_with(skill_part) || skill_part.is_empty();

                if matches_chain {
                    let desc = chain_config
                        .description
                        .clone()
                        .unwrap_or_else(|| format!("{} steps", chain_config.steps.len()));
                    self.suggestions.push(Suggestion {
                        text: format!("{}:{}", agent_part, chain_name),
                        description: format!("[chain] {}", desc),
                        category: "chain",
                    });
                }
            }
        } else {
            // No colon yet - show matching agents and skills
            for (agent_name, agent_config) in &config.agent {
                let name_lower = agent_name.to_lowercase();
                let matches_name = name_lower.starts_with(input_trimmed);
                let matches_alias = agent_config
                    .aliases
                    .iter()
                    .any(|a| a.to_lowercase().starts_with(input_trimmed));

                if matches_name || matches_alias {
                    let backend = agent_config.sdk.default_name();
                    let desc = format!("{} ({})", backend, agent_config.aliases.join(", "));
                    self.suggestions.push(Suggestion {
                        text: format!("{}:", agent_name),
                        description: desc,
                        category: "agent",
                    });
                }
            }

            // Show matching skills (from filesystem only - no legacy modes)
            for (skill_name, skill_config) in &config.skill {
                let skill_lower = skill_name.to_lowercase();
                let matches_skill = skill_lower.starts_with(input_trimmed);
                let matches_alias = skill_config
                    .kyco
                    .aliases
                    .iter()
                    .any(|a| a.to_lowercase().starts_with(input_trimmed));

                if matches_skill || matches_alias {
                    let aliases = if skill_config.kyco.aliases.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", skill_config.kyco.aliases.join(", "))
                    };
                    let agent_hint = skill_config.kyco.agent.as_deref().unwrap_or(default_agent);
                    self.suggestions.push(Suggestion {
                        text: skill_name.to_string(),
                        description: format!("default: {}{}", agent_hint, aliases),
                        category: "skill",
                    });
                }
            }

            for (chain_name, chain_config) in &config.chain {
                let chain_lower = chain_name.to_lowercase();
                if chain_lower.starts_with(input_trimmed) || chain_lower.contains(input_trimmed) {
                    let desc = chain_config
                        .description
                        .clone()
                        .unwrap_or_else(|| format!("{} steps", chain_config.steps.len()));
                    self.suggestions.push(Suggestion {
                        text: chain_name.to_string(),
                        description: format!("[chain] {}", desc),
                        category: "chain",
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

    /// Maximum number of suggestions to display (must match popup.rs)
    const MAX_VISIBLE: usize = 5;

    /// Move selection down
    pub fn select_next(&mut self) {
        if !self.suggestions.is_empty() {
            let max_idx = self.suggestions.len().min(Self::MAX_VISIBLE) - 1;
            if self.selected_suggestion < max_idx {
                self.selected_suggestion += 1;
            } else {
                self.selected_suggestion = 0; // Wrap around
            }
        }
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if !self.suggestions.is_empty() {
            let max_idx = self.suggestions.len().min(Self::MAX_VISIBLE) - 1;
            if self.selected_suggestion > 0 {
                self.selected_suggestion -= 1;
            } else {
                self.selected_suggestion = max_idx; // Wrap around
            }
        }
    }
}
