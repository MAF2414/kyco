//! Integration tests for GitManager::new()

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

use kyco::git::GitManager;

use common::create_test_repo;

#[test]
fn test_git_manager_new_with_valid_repo() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path());

    assert!(
        manager.is_ok(),
        "GitManager::new should succeed for a valid git repository"
    );

    let manager = manager.unwrap();
    assert_eq!(
        manager.root(),
        temp_dir.path(),
        "Root path should match the provided path"
    );
}

#[test]
fn test_git_manager_new_with_non_git_directory() {
    // Create a regular directory (not a git repo)
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let result = GitManager::new(temp_dir.path());

    assert!(
        result.is_err(),
        "GitManager::new should fail for non-git directory"
    );
    let err_msg = result.err().unwrap().to_string();
    assert!(
        err_msg.contains("Not a git repository"),
        "Error message should indicate not a git repository, got: {}",
        err_msg
    );
}

#[test]
fn test_git_manager_new_with_nonexistent_path() {
    let nonexistent_path = PathBuf::from("/tmp/definitely_does_not_exist_12345");

    let result = GitManager::new(&nonexistent_path);

    assert!(
        result.is_err(),
        "GitManager::new should fail for nonexistent path"
    );
    let err_msg = result.err().unwrap().to_string();
    assert!(
        err_msg.contains("Not a git repository"),
        "Error message should indicate not a git repository, got: {}",
        err_msg
    );
}

#[test]
fn test_git_manager_new_accepts_pathbuf() {
    let temp_dir = create_test_repo();
    let path_buf: PathBuf = temp_dir.path().to_path_buf();

    let manager = GitManager::new(path_buf);

    assert!(manager.is_ok(), "GitManager::new should accept PathBuf");
}

#[test]
fn test_git_manager_new_accepts_path_reference() {
    let temp_dir = create_test_repo();
    let path: &Path = temp_dir.path();

    let manager = GitManager::new(path);

    assert!(manager.is_ok(), "GitManager::new should accept &Path");
}

#[test]
fn test_git_manager_new_accepts_string() {
    let temp_dir = create_test_repo();
    let path_string: String = temp_dir.path().to_str().unwrap().to_string();

    let manager = GitManager::new(path_string);

    assert!(manager.is_ok(), "GitManager::new should accept String");
}

#[test]
fn test_git_manager_new_with_bare_repo() {
    // A bare git repo has a different structure - no .git folder but git files directly
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Initialize a bare repo
    Command::new("git")
        .args(["init", "--bare"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init bare git repo");

    // Bare repos don't have .git directory, so this should fail
    let result = GitManager::new(repo_path);

    assert!(
        result.is_err(),
        "GitManager::new should fail for bare repositories (no .git directory)"
    );
}

#[test]
fn test_git_manager_new_with_nested_git_repo() {
    // Create a git repo with a nested git repo inside
    let temp_dir = create_test_repo();
    let nested_path = temp_dir.path().join("nested");

    // Create nested repo
    fs::create_dir_all(&nested_path).expect("Failed to create nested dir");
    Command::new("git")
        .args(["init"])
        .current_dir(&nested_path)
        .output()
        .expect("Failed to init nested git repo");

    // Configure git user for the nested repo
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&nested_path)
        .output()
        .expect("Failed to configure git email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&nested_path)
        .output()
        .expect("Failed to configure git name");

    // Create initial commit in nested repo
    fs::write(nested_path.join("nested.txt"), "nested content\n")
        .expect("Failed to write nested file");

    Command::new("git")
        .args(["add", "."])
        .current_dir(&nested_path)
        .output()
        .expect("Failed to git add in nested repo");

    Command::new("git")
        .args(["commit", "-m", "Initial nested commit"])
        .current_dir(&nested_path)
        .output()
        .expect("Failed to commit in nested repo");

    // Should be able to create GitManager for both repos independently
    let parent_manager = GitManager::new(temp_dir.path());
    let nested_manager = GitManager::new(&nested_path);

    assert!(
        parent_manager.is_ok(),
        "Should create manager for parent repo"
    );
    assert!(
        nested_manager.is_ok(),
        "Should create manager for nested repo"
    );

    // Verify they have different root paths
    let parent_manager = parent_manager.unwrap();
    let nested_manager = nested_manager.unwrap();

    assert_eq!(parent_manager.root(), temp_dir.path());
    assert_eq!(nested_manager.root(), nested_path.as_path());
}

#[test]
fn test_git_manager_new_preserves_trailing_slash() {
    let temp_dir = create_test_repo();
    let path_with_slash = format!("{}/", temp_dir.path().display());

    let manager = GitManager::new(path_with_slash);

    // Should still succeed even with trailing slash
    assert!(
        manager.is_ok(),
        "GitManager::new should handle paths with trailing slash"
    );
}

#[test]
fn test_git_manager_root_returns_correct_path() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    // The root() method should return the exact path we passed in
    let root = manager.root();
    assert_eq!(
        root,
        temp_dir.path(),
        "root() should return the path passed to new()"
    );
}

#[test]
fn test_git_manager_new_with_empty_repo_no_commits() {
    // Create a git repo without any commits
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repo");

    // GitManager::new should still succeed - the check for commits is in create_worktree
    let manager = GitManager::new(repo_path);

    assert!(
        manager.is_ok(),
        "GitManager::new should succeed even without commits"
    );

    // But has_commits should return false
    let manager = manager.unwrap();
    assert!(
        !manager.has_commits(),
        "has_commits should return false for empty repo"
    );
}

#[test]
fn test_git_manager_has_commits_returns_true_for_repo_with_commits() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    assert!(
        manager.has_commits(),
        "has_commits should return true for repo with commits"
    );
}

#[test]
fn test_git_manager_head_sha_returns_valid_sha() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let sha = manager.head_sha().expect("Failed to get HEAD sha");

    // SHA should be 40 hex characters
    assert_eq!(sha.len(), 40, "SHA should be 40 characters long");
    assert!(
        sha.chars().all(|c| c.is_ascii_hexdigit()),
        "SHA should only contain hex digits"
    );
}

#[test]
fn test_git_manager_head_sha_fails_for_empty_repo() {
    // Create a git repo without any commits
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repo");

    let manager = GitManager::new(repo_path).expect("Failed to create GitManager");

    let result = manager.head_sha();

    assert!(
        result.is_err(),
        "head_sha should fail for repo without commits"
    );
}
