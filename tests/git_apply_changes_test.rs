//! Integration tests for Git apply changes with new files

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use kyco::git::GitManager;

use common::create_test_repo;

#[test]
fn test_new_file_in_worktree() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 4;

    // Create worktree
    let worktree = manager
        .create_worktree(job_id)
        .expect("Failed to create worktree");

    // Create new file in worktree
    fs::write(worktree.path.join("new_file.txt"), "new file content\n")
        .expect("Failed to create new file");

    // Check changed files - new untracked files won't show in git diff --name-only HEAD
    let diff = manager
        .diff(&worktree.path, Some(&worktree.base_branch))
        .expect("Failed to get diff");

    // Note: Untracked files don't show in git diff HEAD - this is empty
    assert!(diff.is_empty(), "Untracked files should not appear in diff");

    // Cleanup
    manager
        .remove_worktree(job_id)
        .expect("Failed to remove worktree");
}

#[test]
fn test_new_file_staged_in_worktree() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 5;

    // Create worktree
    let worktree = manager
        .create_worktree(job_id)
        .expect("Failed to create worktree");

    // Create new file in worktree
    fs::write(worktree.path.join("new_file.txt"), "new file content\n")
        .expect("Failed to create new file");

    // Stage the new file in the worktree
    Command::new("git")
        .args(["add", "new_file.txt"])
        .current_dir(&worktree.path)
        .output()
        .expect("Failed to stage new file");

    // Now the file should show in diff --cached
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(&worktree.path)
        .output()
        .expect("Failed to get staged diff");

    let staged_files = String::from_utf8_lossy(&output.stdout);
    assert!(
        staged_files.contains("new_file.txt"),
        "Staged file should appear in cached diff"
    );

    // Cleanup
    manager
        .remove_worktree(job_id)
        .expect("Failed to remove worktree");
}

#[test]
fn test_apply_new_file_from_worktree() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 6;

    // Create worktree
    let worktree = manager
        .create_worktree(job_id)
        .expect("Failed to create worktree");

    // Create a new file in the worktree
    fs::write(
        worktree.path.join("brand_new_file.txt"),
        "brand new content\n",
    )
    .expect("Failed to create new file");

    // Verify new file is in changed_files list
    let changed = manager
        .changed_files(&worktree.path)
        .expect("Failed to get changed files");
    assert!(
        changed.contains(&PathBuf::from("brand_new_file.txt")),
        "New file should be in changed files list"
    );

    // Apply changes
    manager
        .apply_changes(&worktree.path, &worktree.base_branch, None)
        .expect("Failed to apply changes");

    // Verify new file exists in main repo
    let main_file = temp_dir.path().join("brand_new_file.txt");
    assert!(
        main_file.exists(),
        "New file should exist in main repo after apply"
    );

    let content = fs::read_to_string(&main_file).expect("Failed to read new file in main repo");
    assert_eq!(content, "brand new content\n");

    // Cleanup
    manager
        .remove_worktree(job_id)
        .expect("Failed to remove worktree");
}

#[test]
fn test_apply_new_file_in_subdirectory() {
    let temp_dir = create_test_repo();
    let manager = GitManager::new(temp_dir.path()).expect("Failed to create GitManager");

    let job_id: u64 = 7;

    // Create worktree
    let worktree = manager
        .create_worktree(job_id)
        .expect("Failed to create worktree");

    // Create a new file in a subdirectory in the worktree
    let subdir = worktree.path.join("subdir");
    fs::create_dir_all(&subdir).expect("Failed to create subdirectory");
    fs::write(subdir.join("nested_file.txt"), "nested content\n")
        .expect("Failed to create nested file");

    // Apply changes
    manager
        .apply_changes(&worktree.path, &worktree.base_branch, None)
        .expect("Failed to apply changes");

    // Verify new file exists in main repo with directory structure
    let main_file = temp_dir.path().join("subdir").join("nested_file.txt");
    assert!(
        main_file.exists(),
        "Nested file should exist in main repo after apply"
    );

    let content = fs::read_to_string(&main_file).expect("Failed to read nested file");
    assert_eq!(content, "nested content\n");

    // Cleanup
    manager
        .remove_worktree(job_id)
        .expect("Failed to remove worktree");
}
