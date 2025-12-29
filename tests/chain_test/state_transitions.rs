//! State transition tests for chain step trigger/skip logic

use kyco::config::ChainStep;

#[test]
fn test_chain_step_trigger_logic() {
    let step = ChainStep {
        mode: "fix".to_string(),
        trigger_on: Some(vec!["issues_found".to_string(), "needs_fix".to_string()]),
        skip_on: None,
        agent: None,
        inject_context: None,
    };

    // Simulate trigger check logic
    let previous_state = "issues_found";
    let should_trigger = step
        .trigger_on
        .as_ref()
        .map(|triggers| triggers.contains(&previous_state.to_string()))
        .unwrap_or(true); // None means always run

    assert!(should_trigger);
}

#[test]
fn test_chain_step_skip_logic() {
    let step = ChainStep {
        mode: "fix".to_string(),
        trigger_on: None,
        skip_on: Some(vec!["no_issues".to_string(), "approved".to_string()]),
        agent: None,
        inject_context: None,
    };

    // Simulate skip check logic
    let previous_state = "no_issues";
    let should_skip = step
        .skip_on
        .as_ref()
        .map(|skips| skips.contains(&previous_state.to_string()))
        .unwrap_or(false);

    assert!(should_skip);
}

#[test]
fn test_chain_step_trigger_and_skip_conflict() {
    // Edge case: What happens when a state is in both trigger_on and skip_on?
    let step = ChainStep {
        mode: "fix".to_string(),
        trigger_on: Some(vec!["issues_found".to_string()]),
        skip_on: Some(vec!["issues_found".to_string()]), // Same state in both!
        agent: None,
        inject_context: None,
    };

    let previous_state = "issues_found";

    let should_trigger = step
        .trigger_on
        .as_ref()
        .map(|triggers| triggers.contains(&previous_state.to_string()))
        .unwrap_or(true);

    let should_skip = step
        .skip_on
        .as_ref()
        .map(|skips| skips.contains(&previous_state.to_string()))
        .unwrap_or(false);

    // Both are true - this is a config error
    assert!(should_trigger);
    assert!(should_skip);
}

#[test]
fn test_chain_step_no_trigger_no_skip_always_runs() {
    let step = ChainStep {
        mode: "review".to_string(),
        trigger_on: None,
        skip_on: None,
        agent: None,
        inject_context: None,
    };

    // With both None, the step should always run
    let should_trigger = step.trigger_on.is_none();
    let should_skip = step.skip_on.as_ref().map(|_| false).unwrap_or(false);

    assert!(should_trigger);
    assert!(!should_skip);
}
