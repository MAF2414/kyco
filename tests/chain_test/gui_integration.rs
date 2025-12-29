//! ChainStepEdit tests (GUI Integration)

use kyco::config::ChainStep;
use kyco::gui::chains::ChainStepEdit;

#[test]
fn test_chain_step_edit_default() {
    let edit = ChainStepEdit::default();

    assert!(edit.mode.is_empty());
    assert!(edit.trigger_on.is_empty());
    assert!(edit.skip_on.is_empty());
    assert!(edit.agent.is_empty());
    assert!(edit.inject_context.is_empty());
}

#[test]
fn test_chain_step_edit_from_chain_step() {
    let step = ChainStep {
        mode: "review".to_string(),
        trigger_on: Some(vec!["issues_found".to_string(), "needs_fix".to_string()]),
        skip_on: Some(vec!["no_issues".to_string()]),
        agent: Some("claude".to_string()),
        inject_context: Some("Extra context".to_string()),
    };

    let edit = ChainStepEdit::from(&step);

    assert_eq!(edit.mode, "review");
    assert_eq!(edit.trigger_on, "issues_found, needs_fix");
    assert_eq!(edit.skip_on, "no_issues");
    assert_eq!(edit.agent, "claude");
    assert_eq!(edit.inject_context, "Extra context");
}

#[test]
fn test_chain_step_edit_from_chain_step_with_none_optionals() {
    let step = ChainStep {
        mode: "fix".to_string(),
        trigger_on: None,
        skip_on: None,
        agent: None,
        inject_context: None,
    };

    let edit = ChainStepEdit::from(&step);

    assert_eq!(edit.mode, "fix");
    assert!(edit.trigger_on.is_empty());
    assert!(edit.skip_on.is_empty());
    assert!(edit.agent.is_empty());
    assert!(edit.inject_context.is_empty());
}

#[test]
fn test_chain_step_edit_to_chain_step_basic() {
    let mut edit = ChainStepEdit::default();
    edit.mode = "review".to_string();
    edit.trigger_on = "issues_found".to_string();
    edit.skip_on = "no_issues".to_string();
    edit.agent = "claude".to_string();
    edit.inject_context = "context".to_string();

    let step = edit.to_chain_step();

    assert_eq!(step.mode, "review");
    assert_eq!(step.trigger_on, Some(vec!["issues_found".to_string()]));
    assert_eq!(step.skip_on, Some(vec!["no_issues".to_string()]));
    assert_eq!(step.agent, Some("claude".to_string()));
    assert_eq!(step.inject_context, Some("context".to_string()));
}

#[test]
fn test_chain_step_edit_to_chain_step_empty_strings_become_none() {
    let mut edit = ChainStepEdit::default();
    edit.mode = "review".to_string();
    edit.trigger_on = "".to_string();
    edit.skip_on = "".to_string();
    edit.agent = "".to_string();
    edit.inject_context = "".to_string();

    let step = edit.to_chain_step();

    assert_eq!(step.mode, "review");
    assert!(step.trigger_on.is_none());
    assert!(step.skip_on.is_none());
    assert!(step.agent.is_none());
    assert!(step.inject_context.is_none());
}

#[test]
fn test_chain_step_edit_whitespace_agent_becomes_none() {
    // BUG TEST: Agent field with only whitespace should become None
    let mut edit = ChainStepEdit::default();
    edit.mode = "review".to_string();
    edit.agent = "   ".to_string(); // whitespace only

    let step = edit.to_chain_step();

    assert!(
        step.agent.is_none(),
        "Agent with only whitespace should become None"
    );
}

#[test]
fn test_chain_step_edit_agent_whitespace_trimmed() {
    // BUG TEST: Agent field whitespace should be trimmed
    // Identified in review: persistence.rs - agent field whitespace not trimmed before storage
    let mut edit = ChainStepEdit::default();
    edit.mode = "review".to_string();
    edit.agent = "  claude  ".to_string();

    let step = edit.to_chain_step();

    // The agent should be trimmed
    assert_eq!(step.agent, Some("claude".to_string()));
}

#[test]
fn test_chain_step_edit_inject_context_whitespace_trimmed() {
    let mut edit = ChainStepEdit::default();
    edit.mode = "review".to_string();
    edit.inject_context = "  some context  ".to_string();

    let step = edit.to_chain_step();

    assert_eq!(step.inject_context, Some("some context".to_string()));
}

#[test]
fn test_chain_step_edit_trigger_on_comma_separated() {
    let mut edit = ChainStepEdit::default();
    edit.mode = "fix".to_string();
    edit.trigger_on = "issues_found, needs_fix, critical".to_string();

    let step = edit.to_chain_step();

    let triggers = step.trigger_on.unwrap();
    assert_eq!(triggers.len(), 3);
    assert!(triggers.contains(&"issues_found".to_string()));
    assert!(triggers.contains(&"needs_fix".to_string()));
    assert!(triggers.contains(&"critical".to_string()));
}

#[test]
fn test_chain_step_edit_trigger_on_whitespace_trimmed() {
    let mut edit = ChainStepEdit::default();
    edit.mode = "fix".to_string();
    edit.trigger_on = "  issues_found  ,  needs_fix  ".to_string();

    let step = edit.to_chain_step();

    let triggers = step.trigger_on.unwrap();
    // Each state should be trimmed
    assert!(triggers.contains(&"issues_found".to_string()));
    assert!(triggers.contains(&"needs_fix".to_string()));
}

#[test]
fn test_chain_step_edit_trigger_on_duplicates_deduplicated() {
    // BUG TEST: Duplicate states should be deduplicated
    // Identified in review: state.rs:51-59 - duplicate states not deduplicated
    let mut edit = ChainStepEdit::default();
    edit.mode = "fix".to_string();
    edit.trigger_on = "issues_found, issues_found, critical, issues_found".to_string();

    let step = edit.to_chain_step();

    let triggers = step.trigger_on.unwrap();
    // Should be deduplicated to 2 unique states
    assert_eq!(triggers.len(), 2);
    assert!(triggers.contains(&"issues_found".to_string()));
    assert!(triggers.contains(&"critical".to_string()));
}

#[test]
fn test_chain_step_edit_skip_on_duplicates_deduplicated() {
    let mut edit = ChainStepEdit::default();
    edit.mode = "fix".to_string();
    edit.skip_on = "no_issues, approved, no_issues".to_string();

    let step = edit.to_chain_step();

    let skips = step.skip_on.unwrap();
    assert_eq!(skips.len(), 2);
}

#[test]
fn test_chain_step_edit_empty_items_filtered_out() {
    let mut edit = ChainStepEdit::default();
    edit.mode = "fix".to_string();
    edit.trigger_on = "issues_found, , , critical".to_string();

    let step = edit.to_chain_step();

    let triggers = step.trigger_on.unwrap();
    // Empty items should be filtered out
    assert_eq!(triggers.len(), 2);
    assert!(!triggers.contains(&"".to_string()));
}

#[test]
fn test_chain_step_edit_whitespace_only_items_filtered_out() {
    let mut edit = ChainStepEdit::default();
    edit.mode = "fix".to_string();
    edit.trigger_on = "issues_found,   , critical".to_string();

    let step = edit.to_chain_step();

    let triggers = step.trigger_on.unwrap();
    assert_eq!(triggers.len(), 2);
}

#[test]
fn test_chain_step_edit_roundtrip() {
    // Test converting from ChainStep -> ChainStepEdit -> ChainStep
    let original = ChainStep {
        mode: "review".to_string(),
        trigger_on: Some(vec!["state1".to_string(), "state2".to_string()]),
        skip_on: Some(vec!["skip1".to_string()]),
        agent: Some("claude".to_string()),
        inject_context: Some("context".to_string()),
    };

    let edit = ChainStepEdit::from(&original);
    let restored = edit.to_chain_step();

    assert_eq!(restored.mode, original.mode);
    assert_eq!(restored.trigger_on, original.trigger_on);
    assert_eq!(restored.skip_on, original.skip_on);
    assert_eq!(restored.agent, original.agent);
    assert_eq!(restored.inject_context, original.inject_context);
}

#[test]
fn test_chain_step_edit_clone() {
    let mut edit = ChainStepEdit::default();
    edit.mode = "review".to_string();
    edit.trigger_on = "state1".to_string();

    let cloned = edit.clone();

    assert_eq!(cloned.mode, edit.mode);
    assert_eq!(cloned.trigger_on, edit.trigger_on);
}
