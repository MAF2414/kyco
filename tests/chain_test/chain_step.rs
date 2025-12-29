//! Tests for ChainStep struct

use kyco::config::ChainStep;

#[test]
fn test_chain_step_with_all_fields() {
    let step = ChainStep {
        mode: "review".to_string(),
        trigger_on: Some(vec!["issues_found".to_string()]),
        skip_on: Some(vec!["no_issues".to_string()]),
        agent: Some("claude".to_string()),
        inject_context: Some("Extra context".to_string()),
    };

    assert_eq!(step.mode, "review");
    assert_eq!(step.trigger_on.as_ref().unwrap().len(), 1);
    assert_eq!(step.skip_on.as_ref().unwrap().len(), 1);
    assert!(step.agent.is_some());
    assert!(step.inject_context.is_some());
}

#[test]
fn test_chain_step_with_none_optionals() {
    let step = ChainStep {
        mode: "fix".to_string(),
        trigger_on: None,
        skip_on: None,
        agent: None,
        inject_context: None,
    };

    assert_eq!(step.mode, "fix");
    assert!(step.trigger_on.is_none());
    assert!(step.skip_on.is_none());
    assert!(step.agent.is_none());
    assert!(step.inject_context.is_none());
}

#[test]
fn test_chain_step_with_empty_trigger_on() {
    let step = ChainStep {
        mode: "review".to_string(),
        trigger_on: Some(vec![]),
        skip_on: None,
        agent: None,
        inject_context: None,
    };

    // Empty vec is different from None
    assert!(step.trigger_on.is_some());
    assert!(step.trigger_on.unwrap().is_empty());
}

#[test]
fn test_chain_step_with_multiple_triggers() {
    let step = ChainStep {
        mode: "fix".to_string(),
        trigger_on: Some(vec![
            "issues_found".to_string(),
            "needs_fix".to_string(),
            "critical".to_string(),
        ]),
        skip_on: None,
        agent: None,
        inject_context: None,
    };

    let triggers = step.trigger_on.unwrap();
    assert_eq!(triggers.len(), 3);
    assert!(triggers.contains(&"issues_found".to_string()));
    assert!(triggers.contains(&"needs_fix".to_string()));
    assert!(triggers.contains(&"critical".to_string()));
}

#[test]
fn test_chain_step_serialization_roundtrip() {
    let step = ChainStep {
        mode: "review".to_string(),
        trigger_on: Some(vec!["state1".to_string()]),
        skip_on: Some(vec!["state2".to_string()]),
        agent: Some("codex".to_string()),
        inject_context: Some("context".to_string()),
    };

    let serialized = serde_json::to_string(&step).expect("Failed to serialize");
    let deserialized: ChainStep = serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(deserialized.mode, step.mode);
    assert_eq!(deserialized.trigger_on, step.trigger_on);
    assert_eq!(deserialized.skip_on, step.skip_on);
    assert_eq!(deserialized.agent, step.agent);
    assert_eq!(deserialized.inject_context, step.inject_context);
}

#[test]
fn test_chain_step_deserialization_with_defaults() {
    // JSON with only mode field - others should default
    let json = r#"{"mode": "implement"}"#;
    let step: ChainStep = serde_json::from_str(json).expect("Failed to deserialize");

    assert_eq!(step.mode, "implement");
    assert!(step.trigger_on.is_none());
    assert!(step.skip_on.is_none());
    assert!(step.agent.is_none());
    assert!(step.inject_context.is_none());
}

#[test]
fn test_chain_step_clone() {
    let original = ChainStep {
        mode: "review".to_string(),
        trigger_on: Some(vec!["state1".to_string()]),
        skip_on: Some(vec!["state2".to_string()]),
        agent: Some("claude".to_string()),
        inject_context: Some("context".to_string()),
    };

    let cloned = original.clone();

    assert_eq!(cloned.mode, original.mode);
    assert_eq!(cloned.trigger_on, original.trigger_on);
    assert_eq!(cloned.skip_on, original.skip_on);
    assert_eq!(cloned.agent, original.agent);
    assert_eq!(cloned.inject_context, original.inject_context);
}
