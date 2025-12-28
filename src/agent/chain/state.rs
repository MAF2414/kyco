//! State detection for chain step transitions.
//!
//! This module handles detecting states from agent output and evaluating
//! trigger conditions for chain steps.

use regex::Regex;

use crate::config::{ChainStep, Config, StateDefinition};

/// Detects states from output text using the chain's state definitions.
///
/// Iterates through each state definition and checks if any of its patterns
/// match the output text. Returns all matching state IDs.
///
/// # Arguments
///
/// * `states` - State definitions from the chain configuration
/// * `output` - Output text from the previous step
///
/// # Returns
///
/// Vector of state IDs that were detected in the output.
pub fn detect_states(states: &[StateDefinition], output: &Option<String>) -> Vec<String> {
    let Some(output_text) = output else {
        return Vec::new();
    };

    let mut detected = Vec::new();

    for state in states {
        let matched = state.patterns.iter().any(|pattern| {
            if state.is_regex {
                // Regex matching
                let regex_pattern = if state.case_insensitive {
                    format!("(?i){}", pattern)
                } else {
                    pattern.clone()
                };
                match Regex::new(&regex_pattern) {
                    Ok(re) => re.is_match(output_text),
                    Err(_) => {
                        // Invalid regex - fall back to text search
                        if state.case_insensitive {
                            output_text.to_lowercase().contains(&pattern.to_lowercase())
                        } else {
                            output_text.contains(pattern)
                        }
                    }
                }
            } else {
                // Plain text search
                if state.case_insensitive {
                    output_text.to_lowercase().contains(&pattern.to_lowercase())
                } else {
                    output_text.contains(pattern)
                }
            }
        });

        if matched {
            detected.push(state.id.clone());
        }
    }

    detected
}

/// Auto-detects states from output text using a mode's output_states.
///
/// This enables chains to work without explicit state definitions by
/// automatically generating patterns from the mode's output_states.
/// For example, if a mode has `output_states = ["issues_found", "no_issues"]`,
/// this will look for patterns like:
/// - `state to "issues_found"` or `state: issues_found`
/// - `issues_found` (the state name itself, case-insensitive)
///
/// # Arguments
///
/// * `config` - Application configuration with mode definitions
/// * `mode_name` - The name of the mode to look up
/// * `output` - The output text to search for state patterns
///
/// # Returns
///
/// A vector of detected state IDs from the mode's output_states.
pub fn detect_states_from_mode(
    config: &Config,
    mode_name: &str,
    output: &Option<String>,
) -> Vec<String> {
    let Some(output_text) = output else {
        return Vec::new();
    };

    // Look up the mode to get its output_states
    let Some(mode) = config.mode.get(mode_name) else {
        return Vec::new();
    };

    if mode.output_states.is_empty() {
        return Vec::new();
    }

    let output_lower = output_text.to_lowercase();
    let mut detected = Vec::new();

    for state_id in &mode.output_states {
        let state_lower = state_id.to_lowercase();

        // Check for various patterns that indicate this state
        let patterns = [
            format!("state to \"{}\"", state_lower),
            format!("state: {}", state_lower),
            format!("set state to {}", state_lower),
            format!("setting state to {}", state_lower),
            state_lower.clone(), // The state name itself
        ];

        let matched = patterns.iter().any(|p| output_lower.contains(p));

        if matched {
            detected.push(state_id.clone());
        }
    }

    detected
}

/// Evaluates whether a step should execute based on trigger conditions.
///
/// The evaluation order is:
/// 1. If `skip_on` contains any detected state → skip (return `false`)
/// 2. If `trigger_on` is specified and no detected state matches → skip
/// 3. Otherwise → run (return `true`)
///
/// # Arguments
///
/// * `step` - The step configuration with optional `trigger_on`/`skip_on`
/// * `detected_states` - States detected from the previous step's output
///
/// # Returns
///
/// `true` if the step should execute, `false` if it should be skipped.
pub fn should_step_run(step: &ChainStep, detected_states: &[String]) -> bool {
    // Check skip_on first - if any detected state matches, don't run
    if let Some(skip_states) = &step.skip_on {
        for detected in detected_states {
            if skip_states.contains(detected) {
                return false;
            }
        }
    }

    // Check trigger_on - if specified, at least one detected state must match
    if let Some(trigger_states) = &step.trigger_on {
        if detected_states.is_empty() {
            return false; // No states detected, can't trigger
        }
        // Check if any detected state is in trigger_on
        detected_states.iter().any(|d| trigger_states.contains(d))
    } else {
        // No trigger condition = always run
        true
    }
}
