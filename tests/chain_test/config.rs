//! Tests for Config chain integration

use kyco::config::{ChainStep, Config, ModeChain};

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
