use super::*;

#[test]
fn test_voice_action_match() {
    let action = VoiceAction::new("refactor", "refactor").with_alias("r");

    let m = action.matches("refactor this function").unwrap();
    assert_eq!(m.mode, "refactor");
    assert_eq!(m.prompt, "this function");

    let m = action.matches("r the code").unwrap();
    assert_eq!(m.mode, "refactor");
    assert_eq!(m.prompt, "the code");

    assert!(action.matches("fix something").is_none());
}

#[test]
fn test_registry_with_prefix() {
    let mut registry = VoiceActionRegistry::default();
    registry.set_global_prefix("hey kyco");

    // Should match with prefix
    let m = registry.match_text("hey kyco refactor this").unwrap();
    assert_eq!(m.mode, "refactor");
    assert_eq!(m.prompt, "this");

    // Should not match without prefix
    assert!(registry.match_text("refactor this").is_none());
}

#[test]
fn test_german_aliases() {
    let registry = VoiceActionRegistry::default();

    let m = registry.match_text("überarbeite diese Funktion").unwrap();
    assert_eq!(m.mode, "refactor");

    let m = registry.match_text("erkläre diesen Code").unwrap();
    assert_eq!(m.mode, "explain");
}

#[test]
fn test_prompt_template() {
    let action = VoiceAction::new("quick fix", "fix")
        .with_prompt_template("Quickly fix this issue: {prompt}");

    let m = action.matches("quick fix the bug").unwrap();
    assert_eq!(m.get_final_prompt(), "Quickly fix this issue: the bug");
}
