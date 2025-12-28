//! Git manager implementation

mod changes;
mod diff;
mod types;
mod worktree;

#[cfg(test)]
mod tests;

pub use types::{CommitMessage, DiffReport, DiffSettings, FileDiff, FileStatus, WorktreeInfo};

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Find the git repository root for a given path.
/// Returns None if the path is not inside a git repository.
pub fn find_git_root(path: &Path) -> Option<PathBuf> {
    let start_dir = if path.is_file() { path.parent()? } else { path };

    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(start_dir)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if root.is_empty() {
        None
    } else {
        Some(PathBuf::from(root))
    }
}

/// Manages Git operations for KYCo
#[derive(Clone)]
pub struct GitManager {
    /// Root directory of the repository
    root: PathBuf,

    /// Base directory for worktrees
    pub(super) worktrees_dir: PathBuf,
}

impl GitManager {
    /// Create a new Git manager
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();

        if !root.join(".git").exists() {
            bail!("Not a git repository: {}", root.display());
        }

        let worktrees_dir = root.join(".kyco").join("worktrees");

        Ok(Self {
            root,
            worktrees_dir,
        })
    }

    /// Get the current HEAD commit SHA
    pub fn head_sha(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.root)
            .output()
            .context("Failed to run git rev-parse")?;

        if !output.status.success() {
            bail!(
                "git rev-parse failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Check if the repository has at least one commit
    pub fn has_commits(&self) -> bool {
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.root)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get the current branch name
    pub fn current_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&self.root)
            .output()
            .context("Failed to get current branch")?;

        if !output.status.success() {
            bail!(
                "Failed to get current branch: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Check if the repo has uncommitted changes
    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.root)
            .output()
            .context("Failed to run git status")?;

        Ok(!output.stdout.is_empty())
    }

    /// Check if the repo has tracked/staged changes (ignores untracked files).
    pub fn has_tracked_uncommitted_changes(&self) -> Result<bool> {
        let output = Command::new("git")
            .args(["status", "--porcelain", "--untracked-files=no"])
            .current_dir(&self.root)
            .output()
            .context("Failed to run git status")?;

        Ok(!output.stdout.is_empty())
    }

    /// Get the root path
    pub fn root(&self) -> &Path {
        &self.root
    }
}
