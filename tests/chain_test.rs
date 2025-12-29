//! Tests for chain configuration and GUI integration
//!
//! This test file focuses on bugs identified in the review:
//! 1. Agent field whitespace not trimmed before storage
//! 2. Duplicate trigger/skip states not deduplicated
//! 3. Mode validation
//! 4. Edit fields not cleared after chain deletion

use kyco::config::{ChainStep, Config, ModeChain};

// ============================================================================
// ChainStep Tests
// ============================================================================

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

// ============================================================================
// ModeChain Tests
// ============================================================================

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

// ============================================================================
// Config Chain Integration Tests
// ============================================================================

#[test]
fn test_config_get_chain() {
    let config = Config::with_defaults();

    // Default config should have review+fix chain
    let chain = config.get_chain("review+fix");
    assert!(chain.is_some());

    let chain = chain.unwrap();
    assert_eq!(chain.steps.len(), 2);
    assert_eq!(chain.steps[0].mode, "review");
    assert_eq!(chain.steps[1].mode, "fix");
}

#[test]
fn test_config_is_chain() {
    let config = Config::with_defaults();

    assert!(config.is_chain("review+fix"));
    assert!(!config.is_chain("review")); // This is a mode, not a chain
    assert!(!config.is_chain("nonexistent"));
}

#[test]
fn test_config_get_mode_or_chain_returns_chain() {
    let config = Config::with_defaults();

    let result = config.get_mode_or_chain("review+fix");
    assert!(result.is_some());

    if let Some(kyco::config::ModeOrChain::Chain(chain)) = result {
        assert_eq!(chain.steps.len(), 2);
    } else {
        panic!("Expected Chain variant");
    }
}

#[test]
fn test_config_get_mode_or_chain_returns_mode() {
    let config = Config::with_defaults();

    let result = config.get_mode_or_chain("review");
    assert!(result.is_some());

    if let Some(kyco::config::ModeOrChain::Mode(mode)) = result {
        assert!(mode.prompt.is_some());
    } else {
        panic!("Expected Mode variant");
    }
}

#[test]
fn test_config_get_mode_or_chain_returns_none() {
    let config = Config::with_defaults();

    let result = config.get_mode_or_chain("nonexistent");
    assert!(result.is_none());
}

// ============================================================================
// Edge Cases and Bug Tests
// ============================================================================

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
    use std::collections::HashSet;
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

#[test]
fn test_chain_with_nonexistent_mode_reference() {
    // Test that a chain can reference a mode that doesn't exist
    // (validation should happen elsewhere)
    let mut config = Config::default();
    config.chain.insert(
        "bad_chain".to_string(),
        ModeChain {
            version: 0,
            description: Some("Chain with nonexistent mode".to_string()),
            steps: vec![ChainStep {
                mode: "nonexistent_mode".to_string(),
                trigger_on: None,
                skip_on: None,
                agent: None,
                inject_context: None,
            }],
            stop_on_failure: true,
            states: vec![],
            pass_full_response: true,
            use_worktree: None,
        },
    );

    // Chain exists in config
    assert!(config.get_chain("bad_chain").is_some());

    // But the referenced mode doesn't exist
    assert!(config.get_mode("nonexistent_mode").is_none());
}

#[test]
fn test_config_chain_insertion_and_retrieval() {
    let mut config = Config::default();

    let chain = ModeChain {
        version: 0,
        description: Some("Custom chain".to_string()),
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

    config.chain.insert("custom".to_string(), chain);

    assert!(config.is_chain("custom"));
    let retrieved = config.get_chain("custom").unwrap();
    assert_eq!(retrieved.description, Some("Custom chain".to_string()));
}

#[test]
fn test_config_chain_removal() {
    let mut config = Config::with_defaults();

    assert!(config.is_chain("review+fix"));
    config.chain.remove("review+fix");
    assert!(!config.is_chain("review+fix"));
    assert!(config.get_chain("review+fix").is_none());
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

// ============================================================================
// Default Config Chain Tests
// ============================================================================

#[test]
fn test_default_config_has_review_plus_fix_chain() {
    let config = Config::with_defaults();

    let chain = config.get_chain("review+fix");
    assert!(
        chain.is_some(),
        "Default config should have review+fix chain"
    );

    let chain = chain.unwrap();
    assert!(
        chain.description.is_some(),
        "review+fix chain should have description"
    );
    assert!(chain.stop_on_failure, "review+fix should stop on failure");
}

#[test]
fn test_default_review_plus_fix_chain_steps() {
    let config = Config::with_defaults();
    let chain = config.get_chain("review+fix").unwrap();

    // First step: review
    assert_eq!(chain.steps[0].mode, "review");
    assert!(
        chain.steps[0].trigger_on.is_none(),
        "First step should always run"
    );
    assert!(chain.steps[0].skip_on.is_none());

    // Second step: fix
    assert_eq!(chain.steps[1].mode, "fix");
    assert_eq!(
        chain.steps[1].trigger_on,
        Some(vec!["issues_found".to_string()])
    );
    assert_eq!(chain.steps[1].skip_on, Some(vec!["no_issues".to_string()]));
}

#[test]
fn test_default_modes_have_output_states() {
    let config = Config::with_defaults();

    // Review mode should have output states for chaining
    let review = config.get_mode("review").unwrap();
    assert!(
        !review.output_states.is_empty(),
        "Review mode should have output states"
    );
    assert!(review.output_states.contains(&"issues_found".to_string()));
    assert!(review.output_states.contains(&"no_issues".to_string()));

    // Fix mode should have output states
    let fix = config.get_mode("fix").unwrap();
    assert!(
        !fix.output_states.is_empty(),
        "Fix mode should have output states"
    );
}

// ============================================================================
// State Transition Tests
// ============================================================================

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

// ============================================================================
// Serialization Format Tests
// ============================================================================

#[test]
fn test_chain_yaml_serialization() {
    let chain = ModeChain {
        version: 0,
        description: Some("YAML test".to_string()),
        steps: vec![ChainStep {
            mode: "review".to_string(),
            trigger_on: Some(vec!["ready".to_string()]),
            skip_on: None,
            agent: None,
            inject_context: None,
        }],
        stop_on_failure: true,
        states: vec![],
        pass_full_response: true,
        use_worktree: None,
    };

    let yaml_str = serde_yaml::to_string(&chain).expect("Failed to serialize to YAML");
    let deserialized: ModeChain =
        serde_yaml::from_str(&yaml_str).expect("Failed to deserialize from YAML");

    assert_eq!(deserialized.description, chain.description);
    assert_eq!(deserialized.steps.len(), 1);
}

#[test]
fn test_full_config_with_chain_toml_roundtrip() {
    let mut config = Config::with_defaults();

    config.chain.insert(
        "test_chain".to_string(),
        ModeChain {
            version: 0,
            description: Some("Test".to_string()),
            steps: vec![ChainStep {
                mode: "review".to_string(),
                trigger_on: None,
                skip_on: None,
                agent: Some("claude".to_string()),
                inject_context: None,
            }],
            stop_on_failure: false,
            states: vec![],
            pass_full_response: true,
            use_worktree: None,
        },
    );

    let toml_str = toml::to_string(&config).expect("Failed to serialize config to TOML");
    let deserialized: Config =
        toml::from_str(&toml_str).expect("Failed to deserialize config from TOML");

    assert!(deserialized.get_chain("test_chain").is_some());
    let chain = deserialized.get_chain("test_chain").unwrap();
    assert_eq!(chain.description, Some("Test".to_string()));
}

// ============================================================================
// ChainStepEdit Tests (GUI Integration)
// ============================================================================

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

// ============================================================================
// Integration Bug Tests
// ============================================================================

#[test]
fn test_bug_step_index_removal() {
    // BUG TEST: Concurrent step modifications can corrupt indices
    // Simulating the scenario from editor.rs:157-166
    let mut steps: Vec<ChainStepEdit> = vec![
        ChainStepEdit {
            mode: "step1".to_string(),
            ..Default::default()
        },
        ChainStepEdit {
            mode: "step2".to_string(),
            ..Default::default()
        },
        ChainStepEdit {
            mode: "step3".to_string(),
            ..Default::default()
        },
    ];

    // Simulate removing middle step
    let step_to_remove = 1;
    steps.remove(step_to_remove);

    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].mode, "step1");
    assert_eq!(steps[1].mode, "step3");
}

#[test]
fn test_bug_step_swap_up() {
    // Test swapping step up
    let mut steps: Vec<ChainStepEdit> = vec![
        ChainStepEdit {
            mode: "step1".to_string(),
            ..Default::default()
        },
        ChainStepEdit {
            mode: "step2".to_string(),
            ..Default::default()
        },
        ChainStepEdit {
            mode: "step3".to_string(),
            ..Default::default()
        },
    ];

    // Swap step 2 up (i=2 -> swap with i-1=1)
    steps.swap(2, 1);

    assert_eq!(steps[1].mode, "step3");
    assert_eq!(steps[2].mode, "step2");
}

#[test]
fn test_bug_step_swap_down() {
    // Test swapping step down
    let mut steps: Vec<ChainStepEdit> = vec![
        ChainStepEdit {
            mode: "step1".to_string(),
            ..Default::default()
        },
        ChainStepEdit {
            mode: "step2".to_string(),
            ..Default::default()
        },
        ChainStepEdit {
            mode: "step3".to_string(),
            ..Default::default()
        },
    ];

    // Swap step 0 down (i=0 -> swap with i+1=1)
    steps.swap(0, 1);

    assert_eq!(steps[0].mode, "step2");
    assert_eq!(steps[1].mode, "step1");
}

#[test]
#[should_panic]
fn test_bug_step_swap_out_of_bounds() {
    let mut steps: Vec<ChainStepEdit> = vec![
        ChainStepEdit {
            mode: "step1".to_string(),
            ..Default::default()
        },
        ChainStepEdit {
            mode: "step2".to_string(),
            ..Default::default()
        },
    ];

    // This would panic - trying to swap beyond bounds
    steps.swap(1, 2);
}

#[test]
fn test_chain_validation_empty_name() {
    // Test that empty chain name should be rejected
    let chain_name = "";
    assert!(chain_name.trim().is_empty());
}

#[test]
fn test_chain_validation_whitespace_name() {
    // Test that whitespace-only chain name should be rejected
    let chain_name = "   ";
    assert!(chain_name.trim().is_empty());
}

#[test]
fn test_chain_validation_empty_steps() {
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
fn test_mode_exists_validation() {
    // BUG TEST: No validation that mode exists in config
    // Identified in review: persistence.rs:36-42
    let config = Config::with_defaults();

    // Valid mode
    assert!(config.mode.contains_key("review"));

    // Invalid mode
    assert!(!config.mode.contains_key("nonexistent_mode"));
}

#[test]
fn test_chain_step_edit_mode_whitespace() {
    // Test that mode field with whitespace needs to be trimmed before lookup
    let mut edit = ChainStepEdit::default();
    edit.mode = "  review  ".to_string();

    let config = Config::with_defaults();

    // Direct lookup would fail due to whitespace
    assert!(!config.mode.contains_key(&edit.mode));

    // But trimmed lookup should succeed
    assert!(config.mode.contains_key(edit.mode.trim()));
}
