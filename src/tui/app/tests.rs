//! Tests for the TUI app module

use super::remove_tag_from_content;

#[test]
fn test_remove_standalone_tag_rust() {
    let content = r#"fn main() {
    // @@fix handle error
    println!("hello");
}
"#;
    let result = remove_tag_from_content(content, 2, "@@").unwrap();
    assert!(!result.contains("@@fix"), "Tag should be removed");
    assert!(result.contains("println"), "Code should remain");
    assert!(!result.contains("handle error"), "Description should be removed");
}

#[test]
fn test_remove_standalone_tag_python() {
    let content = r#"def foo():
    # @@docs add docstrings
    pass
"#;
    let result = remove_tag_from_content(content, 2, "@@").unwrap();
    assert!(!result.contains("@@docs"), "Tag should be removed");
    assert!(result.contains("pass"), "Code should remain");
}

#[test]
fn test_remove_standalone_tag_with_multiline_description() {
    let content = r#"// @@refactor clean up this code
// Make it more readable
// And add proper error handling
fn messy_function() {
    // some code
}
"#;
    let result = remove_tag_from_content(content, 1, "@@").unwrap();
    assert!(!result.contains("@@refactor"), "Tag should be removed");
    assert!(!result.contains("Make it more readable"), "Description line 1 should be removed");
    assert!(!result.contains("And add proper"), "Description line 2 should be removed");
    assert!(result.contains("fn messy_function"), "Code should remain");
    assert!(result.contains("// some code"), "Other comments should remain");
}

#[test]
fn test_remove_inline_tag_rust() {
    let content = r#"fn process() {
    let x = 42; // @@fix this is wrong
    println!("{}", x);
}
"#;
    let result = remove_tag_from_content(content, 2, "@@").unwrap();
    assert!(!result.contains("@@fix"), "Tag should be removed");
    assert!(result.contains("let x = 42;"), "Code should remain");
    assert!(!result.contains("this is wrong"), "Description should be removed");
}

#[test]
fn test_remove_inline_tag_python() {
    let content = r#"def bar():
    x = 1  # @@optimize make faster
    return x
"#;
    let result = remove_tag_from_content(content, 2, "@@").unwrap();
    assert!(!result.contains("@@optimize"), "Tag should be removed");
    assert!(result.contains("x = 1"), "Code should remain");
    assert!(!result.contains("make faster"), "Description should be removed");
}

#[test]
fn test_remove_tag_preserves_other_comments() {
    let content = r#"// This is a normal comment
// @@tests write unit tests
// Description of what to test
fn important_function() {
    // Implementation comment
}
"#;
    let result = remove_tag_from_content(content, 2, "@@").unwrap();
    assert!(!result.contains("@@tests"), "Tag should be removed");
    assert!(result.contains("This is a normal comment"), "Previous comment should remain");
    assert!(result.contains("Implementation comment"), "Other comments should remain");
}

#[test]
fn test_remove_tag_at_first_line() {
    let content = r#"# @@implement create new feature
# Add authentication support
def authenticate():
    pass
"#;
    let result = remove_tag_from_content(content, 1, "@@").unwrap();
    assert!(!result.contains("@@implement"), "Tag should be removed");
    assert!(!result.contains("Add authentication"), "Description should be removed");
    assert!(result.contains("def authenticate"), "Code should remain");
}

#[test]
fn test_tag_not_found_returns_none() {
    let content = "fn main() {}\n";
    let result = remove_tag_from_content(content, 1, "@@");
    assert!(result.is_none(), "Should return None when tag not found");
}

#[test]
fn test_line_out_of_bounds_returns_none() {
    let content = "fn main() {}\n";
    let result = remove_tag_from_content(content, 100, "@@");
    assert!(result.is_none(), "Should return None for invalid line number");
}

#[test]
fn test_custom_marker_prefix() {
    let content = r#"fn foo() {
    // ::docs add documentation
    bar();
}
"#;
    let result = remove_tag_from_content(content, 2, "::").unwrap();
    assert!(!result.contains("::docs"), "Custom marker should be removed");
    assert!(result.contains("bar()"), "Code should remain");
}

#[test]
fn test_preserves_trailing_newline() {
    let content = "# @@fix\ndef foo():\n    pass\n";
    let result = remove_tag_from_content(content, 1, "@@").unwrap();
    assert!(result.ends_with('\n'), "Should preserve trailing newline");
}

#[test]
fn test_no_trailing_newline() {
    let content = "# @@fix\ndef foo():\n    pass";
    let result = remove_tag_from_content(content, 1, "@@").unwrap();
    assert!(!result.ends_with('\n'), "Should not add trailing newline");
}

#[test]
fn test_sql_comment_style() {
    let content = r#"SELECT * FROM users
-- @@review check for SQL injection
WHERE name = 'test';
"#;
    let result = remove_tag_from_content(content, 2, "@@").unwrap();
    assert!(!result.contains("@@review"), "Tag should be removed");
    assert!(result.contains("SELECT"), "SQL should remain");
    assert!(result.contains("WHERE"), "SQL should remain");
}

#[test]
fn test_description_stops_at_code() {
    let content = r#"# @@implement new feature
# First do this
# Then do that
x = 1
# This is a different comment
"#;
    let result = remove_tag_from_content(content, 1, "@@").unwrap();
    assert!(!result.contains("@@implement"), "Tag should be removed");
    assert!(!result.contains("First do this"), "Description should be removed");
    assert!(!result.contains("Then do that"), "Description should be removed");
    assert!(result.contains("x = 1"), "Code should remain");
    assert!(result.contains("This is a different comment"), "Later comment should remain");
}
