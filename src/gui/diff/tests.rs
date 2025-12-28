//! Tests for diff module

use super::*;

#[test]
fn test_diff_state_new() {
    let state = DiffState::new();
    assert!(state.content.is_none());
    assert!(state.file_path.is_none());
    assert!(!state.has_content());
}

#[test]
fn test_diff_state_set_content() {
    let mut state = DiffState::new();
    state.set_content("test diff".to_string());
    assert!(state.has_content());
    assert_eq!(state.content.as_deref(), Some("test diff"));
}

#[test]
fn test_diff_state_clear() {
    let mut state = DiffState::new();
    state.set_content("test diff".to_string());
    state.clear();
    assert!(!state.has_content());
    assert!(state.file_path.is_none());
}

#[test]
fn test_extract_file_path() {
    let diff = "diff --git a/foo.rs b/foo.rs\nindex 123..456\n--- a/foo.rs\n+++ b/foo.rs\n@@ -1,2 +1,3 @@";
    assert_eq!(extract_file_path(diff), Some("foo.rs".to_string()));

    let diff_no_path = "@@ -1,2 +1,3 @@\n+added";
    assert_eq!(extract_file_path(diff_no_path), None);
}

#[test]
fn test_parse_hunk_header() {
    let hunk = "@@ -10,5 +12,7 @@ fn foo()";
    let info = parse_hunk_header(hunk).unwrap();
    assert_eq!(info.old_start, 10);
    assert_eq!(info.new_start, 12);

    let simple_hunk = "@@ -1 +1 @@";
    let info = parse_hunk_header(simple_hunk).unwrap();
    assert_eq!(info.old_start, 1);
    assert_eq!(info.new_start, 1);

    let invalid = "not a hunk";
    assert!(parse_hunk_header(invalid).is_none());
}
