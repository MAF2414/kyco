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

        let default_agent = &config.settings.gui.default_agent;

        if input_trimmed.is_empty() {
            // Show agents first, then modes when empty
            for (agent_name, agent_config) in &config.agent {
                let backend = agent_config.sdk.default_name();
                let desc = format!("{} ({})", backend, agent_config.aliases.join(", "));
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

        // Check if we have "agent:" prefix - show modes after colon
        if let Some(colon_pos) = input_trimmed.find(':') {
            let agent_part = &input_trimmed[..colon_pos];
            let mode_part = &input_trimmed[colon_pos + 1..];

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

            for (chain_name, chain_config) in &config.chain {
                let chain_lower = chain_name.to_lowercase();
                let matches_chain = chain_lower.starts_with(mode_part) || mode_part.is_empty();

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
            // No colon yet - show matching agents and modes
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

/// Parse the popup input into agent(s), mode, and prompt
///
/// Supports both single-agent and multi-agent syntax:
/// - "claude:refactor" -> (["claude"], "refactor", "")
/// - "claude+codex:refactor optimize" -> (["claude", "codex"], "refactor", "optimize")
/// - "refactor" -> (["claude"], "refactor", "")
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

/// Parse the popup input into multiple agents, mode, and prompt
///
/// Returns (agents, mode, prompt) where agents is a Vec of agent names
pub fn parse_input_multi(input: &str) -> (Vec<String>, String, String) {
    let input = input.trim();

    let (command, prompt) = match input.find(' ') {
        Some(pos) => (&input[..pos], input[pos + 1..].trim()),
        None => (input, ""),
    };

    let (agents_str, mode) = match command.find(':') {
        Some(pos) => (&command[..pos], &command[pos + 1..]),
        None => ("claude", command),
    };

    // Parse agents (may be "claude" or "claude+codex")
    let agents: Vec<String> = agents_str
        .split('+')
        .map(|a| a.trim().to_lowercase())
        .filter(|a| !a.is_empty())
        // Legacy: map Gemini to Claude
        .map(|a| match a.as_str() {
            "g" | "gm" | "gemini" => "claude".to_string(),
            _ => a,
        })
        .collect();

    let agents = if agents.is_empty() {
        vec!["claude".to_string()]
    } else {
        agents
    };

    (agents, mode.to_string(), prompt.to_string())
}

/// Check if input is in multi-agent mode (contains + in agent part)
pub fn is_multi_agent_input(input: &str) -> bool {
    let input = input.trim();
    let command = match input.find(' ') {
        Some(pos) => &input[..pos],
        None => input,
    };

    // Check if there's a + before the : (or before end if no :)
    let agent_part = match command.find(':') {
        Some(pos) => &command[..pos],
        None => command,
    };

    agent_part.contains('+')
}

/// Count the number of agents in the input
pub fn count_agents(input: &str) -> usize {
    let (agents, _, _) = parse_input_multi(input);
    agents.len()
}
