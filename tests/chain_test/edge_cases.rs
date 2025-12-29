//! Edge cases and bug tests for chain configuration

use kyco::config::{ChainStep, ModeChain};
use std::collections::HashSet;

#[test]
fn test_chain_step_with_whitespace_in_mode() {
    // BUG TEST: Mode names with whitespace should be handled
    let step = ChainStep {
        mode: "  review  ".to_string(),
        trigger_on: None,
        skip_on: None,
        agent: None,
        inject_context: None,
    };

    // The mode field contains whitespace - this could cause issues
    // when looking up modes in config
    assert_eq!(step.mode, "  review  ");
    assert_ne!(step.mode.trim(), step.mode);
}

#[test]
fn test_chain_step_with_whitespace_in_agent() {
    // BUG TEST: Agent field with leading/trailing whitespace
    // Identified in review: persistence.rs:61 - agent field whitespace not trimmed
    let step = ChainStep {
        mode: "review".to_string(),
        trigger_on: None,
        skip_on: None,
        agent: Some("  claude  ".to_string()),
        inject_context: None,
    };

    // This tests that whitespace IS present - the bug is that it's not trimmed
    let agent = step.agent.unwrap();
    assert_eq!(agent, "  claude  ");
    assert_ne!(agent.trim(), agent);
}

#[test]
fn test_chain_step_with_duplicate_trigger_states() {
    // BUG TEST: Duplicate states in trigger_on should be handled
    let step = ChainStep {
        mode: "fix".to_string(),
        trigger_on: Some(vec![
            "issues_found".to_string(),
            "issues_found".to_string(), // duplicate
            "critical".to_string(),
        ]),
        skip_on: None,
        agent: None,
        inject_context: None,
    };

    // Without deduplication, we have 3 items
    assert_eq!(step.trigger_on.as_ref().unwrap().len(), 3);

    // After deduplication, we should have 2 unique items
    let unique: HashSet<_> = step.trigger_on.unwrap().into_iter().collect();
    assert_eq!(unique.len(), 2);
}

#[test]
fn test_chain_step_with_empty_string_in_triggers() {
    // Edge case: Empty strings in trigger list
    let step = ChainStep {
        mode: "fix".to_string(),
        trigger_on: Some(vec![
            "issues_found".to_string(),
            "".to_string(),   // empty string
            "  ".to_string(), // whitespace only
        ]),
        skip_on: None,
        agent: None,
        inject_context: None,
    };

    assert_eq!(step.trigger_on.as_ref().unwrap().len(), 3);
}

#[test]
fn test_mode_chain_toml_serialization() {
    let chain = ModeChain {
        version: 0,
        description: Some("TOML test chain".to_string()),
        steps: vec![ChainStep {
            mode: "review".to_string(),
            trigger_on: None,
            skip_on: None,
            agent: None,
            inject_context: None,
        }],
        stop_on_failure: true,
        states: vec![],
        pass_full_response: true,
        use_worktree: None,
    };

    let toml_str = toml::to_string(&chain).expect("Failed to serialize to TOML");
    let deserialized: ModeChain =
        toml::from_str(&toml_str).expect("Failed to deserialize from TOML");

    assert_eq!(deserialized.description, chain.description);
    assert_eq!(deserialized.steps.len(), 1);
}
