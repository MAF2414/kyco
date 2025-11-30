//! Integration tests for Git worktree and diff functionality

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

use kyco::git::GitManager;

/// Creates a temporary git repository for testing
fn create_test_repo() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repo");

    // Configure git user for commits
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to configure git email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to configure git name");

    // Create initial file and commit
    fs::write(repo_path.join("test.txt"), "initial content\n")
        .expect("Failed to write initial file");

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git commit");

    temp_dir
}

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
    let worktree_path = manager.create_worktree(job_id).expect("Failed to create worktree");

    // Verify worktree exists
    assert!(worktree_path.exists(), "Worktree directory should exist");
    assert!(worktree_path.join("test.txt").exists(), "test.txt should exist in worktree");

    // Verify content is the same
    let content = fs::read_to_string(worktree_path.join("test.txt")).expect("Failed to read file");
    assert_eq!(content, "initial content\n");

    // Remove worktree
    manager.remove_worktree(job_id).expect("Failed to remove worktree");

    // Verify worktree is gone
    assert!(!worktree_path.exists(), "Worktree directory should be removed");
}

#[test]
fn test_diff_generation() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 2;

    // Create worktree
    let worktree_path = manager.create_worktree(job_id).expect("Failed to create worktree");

    // Modify file in worktree
    fs::write(worktree_path.join("test.txt"), "modified content\n")
        .expect("Failed to modify file");

    // Check changed files
    let changed = manager.changed_files(&worktree_path).expect("Failed to get changed files");
    assert_eq!(changed.len(), 1, "Should have 1 changed file");
    assert_eq!(changed[0], Path::new("test.txt"));

    // Get full diff
    let diff = manager.diff(&worktree_path).expect("Failed to get diff");
    assert!(diff.contains("-initial content"), "Diff should contain removed line");
    assert!(diff.contains("+modified content"), "Diff should contain added line");

    // Get file-specific diff
    let file_diff = manager.diff_file(&worktree_path, Path::new("test.txt"))
        .expect("Failed to get file diff");
    assert!(file_diff.contains("-initial content"), "File diff should contain removed line");
    assert!(file_diff.contains("+modified content"), "File diff should contain added line");

    // Cleanup
    manager.remove_worktree(job_id).expect("Failed to remove worktree");
}

#[test]
fn test_apply_changes() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 3;

    // Create worktree
    let worktree_path = manager.create_worktree(job_id).expect("Failed to create worktree");

    // Modify file in worktree
    fs::write(worktree_path.join("test.txt"), "applied content\n")
        .expect("Failed to modify file");

    // Apply changes to main repo
    manager.apply_changes(&worktree_path).expect("Failed to apply changes");

    // Verify changes were applied to main repo
    let main_content = fs::read_to_string(temp_dir.path().join("test.txt"))
        .expect("Failed to read main file");
    assert_eq!(main_content, "applied content\n", "Changes should be applied to main repo");

    // Cleanup
    manager.remove_worktree(job_id).expect("Failed to remove worktree");
}

#[test]
fn test_new_file_in_worktree() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 4;

    // Create worktree
    let worktree_path = manager.create_worktree(job_id).expect("Failed to create worktree");

    // Create new file in worktree
    fs::write(worktree_path.join("new_file.txt"), "new file content\n")
        .expect("Failed to create new file");

    // Check changed files - new untracked files won't show in git diff --name-only HEAD
    let diff = manager.diff(&worktree_path).expect("Failed to get diff");

    // Note: Untracked files don't show in git diff HEAD - this is empty
    assert!(diff.is_empty(), "Untracked files should not appear in diff");

    // Cleanup
    manager.remove_worktree(job_id).expect("Failed to remove worktree");
}

#[test]
fn test_new_file_staged_in_worktree() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 5;

    // Create worktree
    let worktree_path = manager.create_worktree(job_id).expect("Failed to create worktree");

    // Create new file in worktree
    fs::write(worktree_path.join("new_file.txt"), "new file content\n")
        .expect("Failed to create new file");

    // Stage the new file in the worktree
    Command::new("git")
        .args(["add", "new_file.txt"])
        .current_dir(&worktree_path)
        .output()
        .expect("Failed to stage new file");

    // Now the file should show in diff --cached
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(&worktree_path)
        .output()
        .expect("Failed to get staged diff");

    let staged_files = String::from_utf8_lossy(&output.stdout);
    assert!(staged_files.contains("new_file.txt"), "Staged file should appear in cached diff");

    // Cleanup
    manager.remove_worktree(job_id).expect("Failed to remove worktree");
}

#[test]
fn test_apply_new_file_from_worktree() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 6;

    // Create worktree
    let worktree_path = manager.create_worktree(job_id).expect("Failed to create worktree");

    // Create a new file in the worktree
    fs::write(worktree_path.join("brand_new_file.txt"), "brand new content\n")
        .expect("Failed to create new file");

    // Verify new file is in changed_files list
    let changed = manager.changed_files(&worktree_path).expect("Failed to get changed files");
    assert!(
        changed.contains(&PathBuf::from("brand_new_file.txt")),
        "New file should be in changed files list"
    );

    // Apply changes
    manager.apply_changes(&worktree_path).expect("Failed to apply changes");

    // Verify new file exists in main repo
    let main_file = temp_dir.path().join("brand_new_file.txt");
    assert!(main_file.exists(), "New file should exist in main repo after apply");

    let content = fs::read_to_string(&main_file).expect("Failed to read new file in main repo");
    assert_eq!(content, "brand new content\n");

    // Cleanup
    manager.remove_worktree(job_id).expect("Failed to remove worktree");
}

#[test]
fn test_apply_new_file_in_subdirectory() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 7;

    // Create worktree
    let worktree_path = manager.create_worktree(job_id).expect("Failed to create worktree");

    // Create a new file in a subdirectory in the worktree
    let subdir = worktree_path.join("subdir");
    fs::create_dir_all(&subdir).expect("Failed to create subdirectory");
    fs::write(subdir.join("nested_file.txt"), "nested content\n")
        .expect("Failed to create nested file");

    // Apply changes
    manager.apply_changes(&worktree_path).expect("Failed to apply changes");

    // Verify new file exists in main repo with directory structure
    let main_file = temp_dir.path().join("subdir").join("nested_file.txt");
    assert!(main_file.exists(), "Nested file should exist in main repo after apply");

    let content = fs::read_to_string(&main_file).expect("Failed to read nested file");
    assert_eq!(content, "nested content\n");

    // Cleanup
    manager.remove_worktree(job_id).expect("Failed to remove worktree");
}

#[test]
fn test_has_uncommitted_changes() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    // Initially, no uncommitted changes
    let has_changes = manager.has_uncommitted_changes().expect("Failed to check uncommitted changes");
    assert!(!has_changes, "Should have no uncommitted changes initially");

    // Modify a file
    fs::write(temp_dir.path().join("test.txt"), "changed\n")
        .expect("Failed to modify file");

    // Now should have uncommitted changes
    let has_changes = manager.has_uncommitted_changes().expect("Failed to check uncommitted changes");
    assert!(has_changes, "Should have uncommitted changes after modification");
}
