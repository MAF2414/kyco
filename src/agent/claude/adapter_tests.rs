use super::*;
use crate::{Job, ScopeDefinition};
use std::path::PathBuf;

fn create_test_job(
    mode: &str,
    description: Option<&str>,
    source_file: &str,
    source_line: usize,
) -> Job {
    // Target format matches what JobManager creates: file:line
    let target = format!("{}:{}", source_file, source_line);
    Job::new(
        1,
        mode.to_string(),
        ScopeDefinition::file(PathBuf::from(source_file)),
        target,
        description.map(|s| s.to_string()),
        "claude".to_string(),
        PathBuf::from(source_file),
        source_line,
        None, // raw_tag_line not needed for these tests
    )
}

#[test]
fn test_prompt_includes_file_and_line() {
    let adapter = ClaudeAdapter::new();
    let config = AgentConfig::default();
    let job = create_test_job("refactor", Some("fix the bug"), "src/main.rs", 42);

    let prompt = adapter.build_prompt(&job, &config);

    // Must contain file:line reference
    assert!(
        prompt.contains("src/main.rs:42"),
        "Prompt should contain file:line reference"
    );
    // Must contain description
    assert!(
        prompt.contains("fix the bug"),
        "Prompt should contain description"
    );
}

#[test]
fn test_prompt_without_description() {
    let adapter = ClaudeAdapter::new();
    let config = AgentConfig::default();
    let job = create_test_job("refactor", None, "lib/utils.py", 10);

    let prompt = adapter.build_prompt(&job, &config);

    // Must contain file:line reference
    assert!(
        prompt.contains("lib/utils.py:10"),
        "Prompt should contain file:line reference"
    );
    // Must mention the mode (case-insensitive, template uses "Refactor")
    assert!(
        prompt.to_lowercase().contains("refactor"),
        "Prompt should mention the mode"
    );
}

#[test]
fn test_prompt_different_files() {
    let adapter = ClaudeAdapter::new();
    let config = AgentConfig::default();

    // Test with different file paths
    let test_cases = vec![
        ("src/app.tsx", 1, "src/app.tsx:1"),
        ("./relative/path.rs", 100, "./relative/path.rs:100"),
        ("deep/nested/file.go", 50, "deep/nested/file.go:50"),
    ];

    for (file, line, expected) in test_cases {
        let job = create_test_job("implement", Some("do something"), file, line);
        let prompt = adapter.build_prompt(&job, &config);
        assert!(
            prompt.contains(expected),
            "Prompt should contain '{}', got: {}",
            expected,
            prompt
        );
    }
}

#[test]
fn test_prompt_format_with_description() {
    let adapter = ClaudeAdapter::new();
    let config = AgentConfig::default();
    let job = create_test_job("fix", Some("handle edge cases"), "test.rs", 5);

    let prompt = adapter.build_prompt(&job, &config);

    // Prompt should contain target (file:line) and description
    assert!(
        prompt.contains("test.rs:5"),
        "Prompt should contain file:line reference"
    );
    assert!(
        prompt.contains("handle edge cases"),
        "Prompt should contain description"
    );
}

#[test]
fn test_prompt_format_without_description() {
    let adapter = ClaudeAdapter::new();
    let config = AgentConfig::default();
    let job = create_test_job("tests", None, "code.py", 20);

    let prompt = adapter.build_prompt(&job, &config);

    // Format should mention file:line and mode
    assert!(
        prompt.contains("code.py:20"),
        "Prompt should contain file:line"
    );
    assert!(prompt.contains("code.py"), "Prompt should mention file");
    // "tests" mode has a template with "Write unit tests"
    assert!(prompt.contains("test"), "Prompt should mention tests");
}
