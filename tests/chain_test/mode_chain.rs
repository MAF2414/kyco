//! Tests for ModeChain struct

use kyco::config::{ChainStep, ModeChain};

#[test]
fn test_mode_chain_creation() {
    let chain = ModeChain {
        version: 0,
        description: Some("Test chain".to_string()),
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

    assert_eq!(chain.description.unwrap(), "Test chain");
    assert_eq!(chain.steps.len(), 1);
    assert!(chain.stop_on_failure);
}

#[test]
fn test_mode_chain_default_stop_on_failure() {
    // When deserializing without stop_on_failure, it should default to true
    let json = r#"{
        "description": "Test",
        "steps": [{"mode": "review"}]
    }"#;

    let chain: ModeChain = serde_json::from_str(json).expect("Failed to deserialize");
    assert!(chain.stop_on_failure);
}

#[test]
fn test_mode_chain_stop_on_failure_false() {
    let chain = ModeChain {
        version: 0,
        description: None,
        steps: vec![],
        stop_on_failure: false,
        states: vec![],
        pass_full_response: true,
        use_worktree: None,
    };

    assert!(!chain.stop_on_failure);
}

#[test]
fn test_mode_chain_with_empty_steps() {
    let chain = ModeChain {
        version: 0,
        description: Some("Empty chain".to_string()),
        steps: vec![],
        stop_on_failure: true,
        states: vec![],
        pass_full_response: true,
        use_worktree: None,
    };

    assert!(chain.steps.is_empty());
}

#[test]
fn test_mode_chain_with_multiple_steps() {
    let chain = ModeChain {
        version: 0,
        description: Some("Review and fix chain".to_string()),
        steps: vec![
            ChainStep {
                mode: "review".to_string(),
                trigger_on: None,
                skip_on: None,
                agent: None,
                inject_context: None,
            },
            ChainStep {
                mode: "fix".to_string(),
                trigger_on: Some(vec!["issues_found".to_string()]),
                skip_on: Some(vec!["no_issues".to_string()]),
                agent: None,
                inject_context: None,
            },
        ],
        stop_on_failure: true,
        states: vec![],
        pass_full_response: true,
        use_worktree: None,
    };

    assert_eq!(chain.steps.len(), 2);
    assert_eq!(chain.steps[0].mode, "review");
    assert_eq!(chain.steps[1].mode, "fix");
    assert!(chain.steps[1].trigger_on.is_some());
}

#[test]
fn test_mode_chain_serialization_roundtrip() {
    let chain = ModeChain {
        version: 0,
        description: Some("Test chain".to_string()),
        steps: vec![
            ChainStep {
                mode: "review".to_string(),
                trigger_on: None,
                skip_on: None,
                agent: Some("claude".to_string()),
                inject_context: None,
            },
            ChainStep {
                mode: "fix".to_string(),
                trigger_on: Some(vec!["issues_found".to_string()]),
                skip_on: None,
                agent: None,
                inject_context: Some("Fix the issues".to_string()),
            },
        ],
        stop_on_failure: false,
        states: vec![],
        pass_full_response: true,
        use_worktree: None,
    };

    let serialized = serde_json::to_string(&chain).expect("Failed to serialize");
    let deserialized: ModeChain = serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(deserialized.description, chain.description);
    assert_eq!(deserialized.steps.len(), chain.steps.len());
    assert_eq!(deserialized.stop_on_failure, chain.stop_on_failure);
}

#[test]
fn test_mode_chain_clone() {
    let original = ModeChain {
        version: 0,
        description: Some("Test".to_string()),
        steps: vec![ChainStep {
            mode: "review".to_string(),
            trigger_on: None,
            skip_on: None,
            agent: None,
            inject_context: None,
        }],
        stop_on_failure: false,
        states: vec![],
        pass_full_response: true,
        use_worktree: None,
    };

    let cloned = original.clone();

    assert_eq!(cloned.description, original.description);
    assert_eq!(cloned.steps.len(), original.steps.len());
    assert_eq!(cloned.stop_on_failure, original.stop_on_failure);
}
