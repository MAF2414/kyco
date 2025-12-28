//! Tests for GitManager

use super::types::{parse_numstat_output, DiffSettings, FileStatus};
use super::GitManager;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to run git {:?}: {}", args, e));
    assert!(
        output.status.success(),
        "git {:?} failed:\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn diff_uses_provided_base_branch() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();

    git(repo, &["init"]);
    git(repo, &["config", "user.email", "test@example.com"]);
    git(repo, &["config", "user.name", "Test User"]);

    fs::write(repo.join("README.md"), "hello\n").expect("write README");
    git(repo, &["add", "README.md"]);
    git(repo, &["commit", "-m", "init"]);
    git(repo, &["branch", "-m", "main"]);

    git(repo, &["checkout", "-b", "kyco/job-1"]);
    fs::write(repo.join("README.md"), "hello world\n").expect("write README");
    git(repo, &["add", "README.md"]);
    git(repo, &["commit", "-m", "change"]);

    let gm = GitManager::new(repo).expect("git manager");
    let diff = gm.diff(repo, Some("main")).expect("diff");
    assert!(
        diff.contains("hello world"),
        "expected diff to include changed content, got:\n{}",
        diff
    );
}

#[test]
fn parse_numstat_output_basic() {
    let output = b"10\t5\tfile.rs\n3\t0\tnew_file.txt\n";
    let results = parse_numstat_output(output);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0], ("file.rs".to_string(), 10, 5, false));
    assert_eq!(results[1], ("new_file.txt".to_string(), 3, 0, false));
}

#[test]
fn parse_numstat_output_binary() {
    let output = b"-\t-\timage.png\n";
    let results = parse_numstat_output(output);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0], ("image.png".to_string(), 0, 0, true));
}

#[test]
fn diff_report_basic() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();

    git(repo, &["init"]);
    git(repo, &["config", "user.email", "test@example.com"]);
    git(repo, &["config", "user.name", "Test User"]);

    fs::write(repo.join("README.md"), "hello\n").expect("write README");
    git(repo, &["add", "README.md"]);
    git(repo, &["commit", "-m", "init"]);
    git(repo, &["branch", "-m", "main"]);

    git(repo, &["checkout", "-b", "kyco/job-1"]);
    fs::write(repo.join("README.md"), "hello world\nline 2\n").expect("write README");
    git(repo, &["add", "README.md"]);
    git(repo, &["commit", "-m", "change"]);

    let gm = GitManager::new(repo).expect("git manager");
    let settings = DiffSettings::default();
    let report = gm
        .diff_report(repo, Some("main"), &settings)
        .expect("diff_report");

    assert_eq!(report.files_changed, 1);
    assert_eq!(report.files[0].path, "README.md");
    assert_eq!(report.files[0].status, FileStatus::Modified);
    assert!(report.total_added > 0 || report.total_removed > 0);
}

#[test]
fn diff_file_patch_basic() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();

    git(repo, &["init"]);
    git(repo, &["config", "user.email", "test@example.com"]);
    git(repo, &["config", "user.name", "Test User"]);

    fs::write(repo.join("test.txt"), "original\n").expect("write test.txt");
    git(repo, &["add", "test.txt"]);
    git(repo, &["commit", "-m", "init"]);

    fs::write(repo.join("test.txt"), "modified content\n").expect("write test.txt");

    let gm = GitManager::new(repo).expect("git manager");
    let settings = DiffSettings::default();
    let patch = gm
        .diff_file_patch(repo, "test.txt", None, &settings)
        .expect("diff_file_patch");

    assert!(
        patch.contains("modified content"),
        "expected patch to contain modified content, got:\n{}",
        patch
    );
}
