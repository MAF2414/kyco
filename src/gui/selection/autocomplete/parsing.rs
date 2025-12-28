//! Input parsing utilities for autocomplete

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
