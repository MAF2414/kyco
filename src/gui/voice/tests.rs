//! Unit tests for voice module.

use super::*;

#[test]
fn test_parse_voice_input_with_keyword() {
    let keywords = vec!["refactor".to_string(), "fix".to_string()];

    let (mode, prompt) = parse_voice_input("refactor this function", &keywords);
    assert_eq!(mode, Some("refactor".to_string()));
    assert_eq!(prompt, "this function");

    let (mode, prompt) = parse_voice_input("Fix the bug in auth", &keywords);
    assert_eq!(mode, Some("fix".to_string()));
    assert_eq!(prompt, "the bug in auth");
}

#[test]
fn test_parse_voice_input_without_keyword() {
    let keywords = vec!["refactor".to_string()];

    let (mode, prompt) = parse_voice_input("hello world", &keywords);
    assert_eq!(mode, None);
    assert_eq!(prompt, "hello world");
}

#[test]
fn test_parse_voice_input_preserves_case() {
    let keywords = vec!["fix".to_string()];

    // Input has mixed case - prompt should preserve original case
    let (mode, prompt) = parse_voice_input("FIX the AuthController Bug", &keywords);
    assert_eq!(mode, Some("fix".to_string()));
    assert_eq!(prompt, "the AuthController Bug");
}

#[test]
fn test_voice_state_display() {
    assert_eq!(VoiceState::Recording.to_string(), "Recording");
    assert_eq!(VoiceState::Listening.to_string(), "Listening");
}
