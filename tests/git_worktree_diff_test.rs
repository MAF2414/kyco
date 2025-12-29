//! Integration tests for Git worktree and diff functionality

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use kyco::git::{CommitMessage, GitManager};

use common::create_test_repo;

#[test]
fn test_git_manager_creation() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    // Should be able to get HEAD sha
    let sha = manager.head_sha().expect("Failed to get HEAD sha");
    assert!(!sha.is_empty(), "HEAD sha should not be empty");
}

#[test]
fn test_worktree_creation_and_removal() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 1;

    // Create worktree
    let worktree = manager
        .create_worktree(job_id)
        .expect("Failed to create worktree");

    // Verify worktree exists
    assert!(worktree.path.exists(), "Worktree directory should exist");
    assert!(
        worktree.path.join("test.txt").exists(),
        "test.txt should exist in worktree"
    );

    // Verify content is the same
    let content = fs::read_to_string(worktree.path.join("test.txt")).expect("Failed to read file");
    assert_eq!(content, "initial content\n");

    // Remove worktree
    manager
        .remove_worktree(job_id)
        .expect("Failed to remove worktree");

    // Verify worktree is gone
    assert!(
        !worktree.path.exists(),
        "Worktree directory should be removed"
    );
}

#[test]
fn test_diff_generation() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 2;

    // Create worktree
    let worktree = manager
        .create_worktree(job_id)
        .expect("Failed to create worktree");

    // Modify file in worktree
    fs::write(worktree.path.join("test.txt"), "modified content\n").expect("Failed to modify file");

    // Check changed files
    let changed = manager
        .changed_files(&worktree.path)
        .expect("Failed to get changed files");
    assert_eq!(changed.len(), 1, "Should have 1 changed file");
    assert_eq!(changed[0], Path::new("test.txt"));

    // Get full diff
    let diff = manager
        .diff(&worktree.path, Some(&worktree.base_branch))
        .expect("Failed to get diff");
    assert!(
        diff.contains("-initial content"),
        "Diff should contain removed line"
    );
    assert!(
        diff.contains("+modified content"),
        "Diff should contain added line"
    );

    // Get file-specific diff
    let file_diff = manager
        .diff_file(&worktree.path, Path::new("test.txt"))
        .expect("Failed to get file diff");
    assert!(
        file_diff.contains("-initial content"),
        "File diff should contain removed line"
    );
    assert!(
        file_diff.contains("+modified content"),
        "File diff should contain added line"
    );

    // Cleanup
    manager
        .remove_worktree(job_id)
        .expect("Failed to remove worktree");
}

#[test]
fn test_apply_changes() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 3;

    // Create worktree
    let worktree = manager
        .create_worktree(job_id)
        .expect("Failed to create worktree");

    // Modify file in worktree
    fs::write(worktree.path.join("test.txt"), "applied content\n").expect("Failed to modify file");

    // Apply changes to main repo
    manager
        .apply_changes(&worktree.path, &worktree.base_branch, None)
        .expect("Failed to apply changes");

    // Verify changes were applied to main repo
    let main_content =
        fs::read_to_string(temp_dir.path().join("test.txt")).expect("Failed to read main file");
    assert_eq!(
        main_content, "applied content\n",
        "Changes should be applied to main repo"
    );

    // Cleanup
    manager
        .remove_worktree(job_id)
        .expect("Failed to remove worktree");
}

#[test]
fn test_apply_changes_uses_commit_message_for_auto_commit() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 30;
    let worktree = manager
        .create_worktree(job_id)
        .expect("Failed to create worktree");

    // Modify file in worktree but do NOT commit.
    fs::write(worktree.path.join("test.txt"), "applied content\n").expect("Failed to modify file");

    let message = CommitMessage {
        subject: "Custom commit subject".to_string(),
        body: Some("Custom commit body".to_string()),
    };

    manager
        .apply_changes(&worktree.path, &worktree.base_branch, Some(&message))
        .expect("Failed to apply changes");

    // Verify commit message was used.
    let output = Command::new("git")
        .args(["log", "-1", "--pretty=%B"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run git log");

    let msg = String::from_utf8_lossy(&output.stdout);
    assert!(msg.contains("Custom commit subject"));
    assert!(msg.contains("Custom commit body"));

    manager
        .remove_worktree(job_id)
        .expect("Failed to remove worktree");
}

#[test]
fn test_has_uncommitted_changes() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    // Initially, no uncommitted changes
    let has_changes = manager
        .has_uncommitted_changes()
        .expect("Failed to check uncommitted changes");
    assert!(!has_changes, "Should have no uncommitted changes initially");

    // Modify a file
    fs::write(temp_dir.path().join("test.txt"), "changed\n").expect("Failed to modify file");

    // Now should have uncommitted changes
    let has_changes = manager
        .has_uncommitted_changes()
        .expect("Failed to check uncommitted changes");
    assert!(
        has_changes,
        "Should have uncommitted changes after modification"
    );
}
