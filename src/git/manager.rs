//! Git manager implementation

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::JobId;

/// Manages Git operations for KYCo
pub struct GitManager {
    /// Root directory of the repository
    root: PathBuf,

    /// Base directory for worktrees
    worktrees_dir: PathBuf,
}

impl GitManager {
    /// Create a new Git manager
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();

        // Verify this is a git repository
        if !root.join(".git").exists() {
            bail!("Not a git repository: {}", root.display());
        }

        let worktrees_dir = root.join(".kyco").join("worktrees");

        Ok(Self { root, worktrees_dir })
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

    /// Create a worktree for a job
    pub fn create_worktree(&self, job_id: JobId) -> Result<PathBuf> {
        // Check if the repository has commits - worktrees require at least one commit
        if !self.has_commits() {
            bail!(
                "Cannot create worktree: repository has no commits. \
                Please make an initial commit first, or disable use_worktree in config."
            );
        }

        let worktree_path = self.worktrees_dir.join(format!("job-{}", job_id));
        let branch_name = format!("kyco/job-{}", job_id);

        // Ensure the worktrees directory exists
        std::fs::create_dir_all(&self.worktrees_dir)?;

        // Create a new branch for this job
        let output = Command::new("git")
            .args(["branch", &branch_name])
            .current_dir(&self.root)
            .output()
            .context("Failed to create branch")?;

        // Branch might already exist, that's okay
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("already exists") {
                bail!("Failed to create branch: {}", stderr);
            }
        }

        // Create the worktree
        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                worktree_path.to_str().unwrap(),
                &branch_name,
            ])
            .current_dir(&self.root)
            .output()
            .context("Failed to create worktree")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Worktree might already exist
            if !stderr.contains("already exists") {
                bail!("Failed to create worktree: {}", stderr);
            }
        }

        Ok(worktree_path)
    }

    /// Remove a worktree for a job
    pub fn remove_worktree(&self, job_id: JobId) -> Result<()> {
        let worktree_path = self.worktrees_dir.join(format!("job-{}", job_id));
        let branch_name = format!("kyco/job-{}", job_id);

        // Remove the worktree
        if worktree_path.exists() {
            let output = Command::new("git")
                .args(["worktree", "remove", "--force", worktree_path.to_str().unwrap()])
                .current_dir(&self.root)
                .output()
                .context("Failed to remove worktree")?;

            if !output.status.success() {
                tracing::warn!(
                    "Failed to remove worktree: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        // Delete the branch
        let output = Command::new("git")
            .args(["branch", "-D", &branch_name])
            .current_dir(&self.root)
            .output()
            .context("Failed to delete branch")?;

        if !output.status.success() {
            tracing::warn!(
                "Failed to delete branch: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Get the list of changed files in a worktree (including modified and new files)
    pub fn changed_files(&self, worktree: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        // Get modified files (tracked files with changes)
        let output = Command::new("git")
            .args(["diff", "--name-only", "HEAD"])
            .current_dir(worktree)
            .output()
            .context("Failed to run git diff")?;

        if !output.status.success() {
            bail!(
                "git diff failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        files.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(PathBuf::from),
        );

        // Get untracked files (new files)
        let output = Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard"])
            .current_dir(worktree)
            .output()
            .context("Failed to run git ls-files")?;

        if output.status.success() {
            files.extend(
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(PathBuf::from),
            );
        }

        Ok(files)
    }

    /// Get the diff for a worktree
    pub fn diff(&self, worktree: &Path) -> Result<String> {
        let output = Command::new("git")
            .args(["diff", "HEAD"])
            .current_dir(worktree)
            .output()
            .context("Failed to run git diff")?;

        if !output.status.success() {
            bail!(
                "git diff failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get the diff for a specific file in a worktree
    pub fn diff_file(&self, worktree: &Path, file: &Path) -> Result<String> {
        let output = Command::new("git")
            .args(["diff", "HEAD", "--", file.to_str().unwrap()])
            .current_dir(worktree)
            .output()
            .context("Failed to run git diff")?;

        if !output.status.success() {
            bail!(
                "git diff failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Apply changes from a worktree to the main repo
    pub fn apply_changes(&self, worktree: &Path) -> Result<()> {
        use std::io::Write;

        // First, copy new (untracked) files
        let output = Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard"])
            .current_dir(worktree)
            .output()
            .context("Failed to run git ls-files")?;

        if output.status.success() {
            let new_files: Vec<PathBuf> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(PathBuf::from)
                .collect();

            for file in new_files {
                let src = worktree.join(&file);
                let dst = self.root.join(&file);

                // Create parent directories if needed
                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                // Copy the new file
                std::fs::copy(&src, &dst).with_context(|| {
                    format!("Failed to copy new file: {}", file.display())
                })?;
            }
        }

        // Then apply the diff for modified files
        let diff = self.diff(worktree)?;

        if diff.is_empty() {
            return Ok(());
        }

        // Apply the diff to the main repo
        let mut child = Command::new("git")
            .args(["apply", "-"])
            .current_dir(&self.root)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn git apply")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(diff.as_bytes())?;
        }

        let status = child.wait()?;

        if !status.success() {
            bail!("git apply failed");
        }

        Ok(())
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

    /// Get the root path
    pub fn root(&self) -> &Path {
        &self.root
    }
}
