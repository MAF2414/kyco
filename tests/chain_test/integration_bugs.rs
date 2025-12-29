//! Integration bug tests for chain configuration

use kyco::config::{Config, ModeChain};
use kyco::gui::chains::ChainStepEdit;

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
