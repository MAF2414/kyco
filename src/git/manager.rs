//! Git manager implementation

use anyhow::{anyhow, bail, Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::JobId;

/// Result of creating a worktree
pub struct WorktreeInfo {
    /// Path to the created worktree
    pub path: PathBuf,
    /// The base branch from which the worktree was created
    pub base_branch: String,
}

/// Manages Git operations for KYCo
#[derive(Clone)]
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

    /// Create a worktree for a job with automatic retry on conflicts
    ///
    /// If the worktree or branch already exists, this will retry with incrementing
    /// suffixes (e.g., job-1, job-1-2, job-1-3) up to `max_retries` times.
    ///
    /// Returns the worktree path and the base branch it was created from.
    pub fn create_worktree(&self, job_id: JobId) -> Result<WorktreeInfo> {
        self.create_worktree_with_retries(job_id, 10)
    }

    /// Create a worktree for a job with configurable retry count
    fn create_worktree_with_retries(&self, job_id: JobId, max_retries: u32) -> Result<WorktreeInfo> {
        // Check if the repository has commits - worktrees require at least one commit
        if !self.has_commits() {
            bail!(
                "Cannot create worktree: repository has no commits. \
                Please make an initial commit first, or disable use_worktree in config."
            );
        }

        // Get the current branch name (base branch for the worktree)
        let base_branch = self.current_branch()?;

        // Ensure the worktrees directory exists
        std::fs::create_dir_all(&self.worktrees_dir)?;

        let mut existing_worktree_names = HashSet::new();
        if let Ok(entries) = std::fs::read_dir(&self.worktrees_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().into_string().ok() {
                    existing_worktree_names.insert(name);
                }
            }
        }

        let mut existing_branch_names = HashSet::new();
        if let Ok(output) = Command::new("git")
            .args(["for-each-ref", "--format=%(refname:short)", "refs/heads/kyco"])
            .current_dir(&self.root)
            .output()
        {
            if output.status.success() {
                existing_branch_names.extend(
                    String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .filter(|line| !line.is_empty())
                        .map(|line| line.to_string()),
                );
            }
        }

        let base_worktree_name = format!("job-{}", job_id);

        // Try creating with base name first, then with suffixes
        for attempt in 0..=max_retries {
            let worktree_dir_name = if attempt == 0 {
                base_worktree_name.clone()
            } else {
                format!("{}-{}", base_worktree_name, attempt)
            };

            if existing_worktree_names.contains(&worktree_dir_name) {
                continue;
            }

            let worktree_path = self.worktrees_dir.join(&worktree_dir_name);

            // Skip if worktree path already exists on disk
            if worktree_path.exists() {
                existing_worktree_names.insert(worktree_dir_name.clone());
                continue;
            }

            let branch_name = format!("kyco/{}", worktree_dir_name);

            if existing_branch_names.contains(&branch_name) {
                continue;
            }

            // Try to create the branch
            let output = Command::new("git")
                .args(["branch", &branch_name])
                .current_dir(&self.root)
                .output()
                .context("Failed to create branch")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("already exists") {
                    // Branch exists, try next suffix
                    continue;
                }
                bail!("Failed to create branch: {}", stderr);
            }

            let worktree_path_str = worktree_path
                .to_str()
                .ok_or_else(|| anyhow!("Worktree path contains invalid UTF-8"))?;

            // Try to create the worktree
            let output = Command::new("git")
                .args([
                    "worktree",
                    "add",
                    worktree_path_str,
                    &branch_name,
                ])
                .current_dir(&self.root)
                .output()
                .context("Failed to create worktree")?;

            if output.status.success() {
                return Ok(WorktreeInfo {
                    path: worktree_path,
                    base_branch: base_branch.clone(),
                });
            }

            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") || stderr.contains("is already checked out") {
                // Worktree conflict, clean up the branch we just created and try next suffix
                let _ = Command::new("git")
                    .args(["branch", "-D", &branch_name])
                    .current_dir(&self.root)
                    .output();
                existing_worktree_names.insert(worktree_dir_name);
                existing_branch_names.insert(branch_name);
                continue;
            }

            // Some other error, fail immediately
            bail!("Failed to create worktree: {}", stderr);
        }

        bail!(
            "Failed to create worktree for job {} after {} retries - all suffixes in use",
            job_id,
            max_retries
        );
    }

    /// Remove a worktree for a job (by job ID - legacy method)
    pub fn remove_worktree(&self, job_id: JobId) -> Result<()> {
        let worktree_path = self.worktrees_dir.join(format!("job-{}", job_id));
        let branch_name = format!("kyco/job-{}", job_id);
        self.remove_worktree_by_path_and_branch(&worktree_path, &branch_name)
    }

    /// Remove a worktree by its path
    ///
    /// This extracts the branch name from the worktree directory name.
    pub fn remove_worktree_by_path(&self, worktree_path: &Path) -> Result<()> {
        // Extract branch name from worktree directory name (e.g., "job-1" or "job-1-2")
        let dir_name = worktree_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Could not extract directory name from worktree path"))?;
        let branch_name = format!("kyco/{}", dir_name);
        self.remove_worktree_by_path_and_branch(worktree_path, &branch_name)
    }

    /// Remove a worktree by path and branch name (internal implementation)
    fn remove_worktree_by_path_and_branch(&self, worktree_path: &Path, branch_name: &str) -> Result<()> {

        // Remove the worktree
        if worktree_path.exists() {
            let worktree_path_str = worktree_path
                .to_str()
                .ok_or_else(|| anyhow!("Worktree path contains invalid UTF-8"))?;
            let output = Command::new("git")
                .args(["worktree", "remove", "--force", worktree_path_str])
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

    /// Get the diff for a worktree (shows all changes vs base branch)
    ///
    /// This shows both committed and uncommitted changes in the worktree
    /// compared to the base branch (master/main).
    pub fn diff(&self, worktree: &Path) -> Result<String> {
        let mut result = String::new();

        // First, get the merge base (common ancestor with master)
        let base_output = Command::new("git")
            .args(["merge-base", "HEAD", "master"])
            .current_dir(worktree)
            .output();

        let base_ref = match base_output {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            }
            _ => "master".to_string(), // Fallback to master if merge-base fails
        };

        // Get diff of committed changes vs base
        let committed_output = Command::new("git")
            .args(["diff", &base_ref, "HEAD"])
            .current_dir(worktree)
            .output()
            .context("Failed to run git diff for committed changes")?;

        if committed_output.status.success() {
            let committed_diff = String::from_utf8_lossy(&committed_output.stdout);
            if !committed_diff.is_empty() {
                result.push_str(&committed_diff);
            }
        }

        // Also get uncommitted changes (in case agent didn't commit everything)
        let uncommitted_output = Command::new("git")
            .args(["diff", "HEAD"])
            .current_dir(worktree)
            .output()
            .context("Failed to run git diff for uncommitted changes")?;

        if uncommitted_output.status.success() {
            let uncommitted_diff = String::from_utf8_lossy(&uncommitted_output.stdout);
            if !uncommitted_diff.is_empty() {
                if !result.is_empty() {
                    result.push_str("\n\n--- Uncommitted changes ---\n\n");
                }
                result.push_str(&uncommitted_diff);
            }
        }

        Ok(result)
    }

    /// Get the diff for a specific file in a worktree
    pub fn diff_file(&self, worktree: &Path, file: &Path) -> Result<String> {
        let file_str = file
            .to_str()
            .ok_or_else(|| anyhow!("File path contains invalid UTF-8"))?;
        let output = Command::new("git")
            .args(["diff", "HEAD", "--", file_str])
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

    /// Merge a worktree branch into the specified base branch
    ///
    /// This performs a proper git merge of the worktree's branch into the base branch.
    /// If there are uncommitted changes in the worktree, they are committed first.
    /// The base_branch parameter specifies which branch to merge into.
    pub fn apply_changes(&self, worktree: &Path, base_branch: &str) -> Result<()> {
        // First, check if there are uncommitted changes and commit them
        let status_output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(worktree)
            .output()
            .context("Failed to check worktree status")?;

        if !status_output.stdout.is_empty() {
            // There are uncommitted changes - commit them
            Command::new("git")
                .args(["add", "-A"])
                .current_dir(worktree)
                .output()
                .context("Failed to stage changes")?;

            let commit_output = Command::new("git")
                .args(["commit", "-m", "Auto-commit remaining changes before merge"])
                .current_dir(worktree)
                .output()
                .context("Failed to commit remaining changes")?;

            if !commit_output.status.success() {
                // Commit might fail if nothing to commit, that's OK
                tracing::debug!(
                    "Auto-commit output: {}",
                    String::from_utf8_lossy(&commit_output.stderr)
                );
            }
        }

        // Get the branch name of the worktree
        let branch_output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(worktree)
            .output()
            .context("Failed to get worktree branch name")?;

        if !branch_output.status.success() {
            bail!(
                "Failed to get branch name: {}",
                String::from_utf8_lossy(&branch_output.stderr)
            );
        }

        let worktree_branch = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        // Get the current branch in the main repo so we can restore it
        let current_branch = self.current_branch()?;

        // Checkout the base branch if we're not already on it
        if current_branch != base_branch {
            let checkout_output = Command::new("git")
                .args(["checkout", base_branch])
                .current_dir(&self.root)
                .output()
                .context("Failed to checkout base branch")?;

            if !checkout_output.status.success() {
                bail!(
                    "Failed to checkout base branch '{}': {}",
                    base_branch,
                    String::from_utf8_lossy(&checkout_output.stderr)
                );
            }
        }

        // Merge the worktree branch into the base branch
        let merge_output = Command::new("git")
            .args(["merge", &worktree_branch, "--no-edit"])
            .current_dir(&self.root)
            .output()
            .context("Failed to merge branch")?;

        if !merge_output.status.success() {
            // If merge failed and we changed branches, try to restore original branch
            if current_branch != base_branch {
                let _ = Command::new("git")
                    .args(["checkout", &current_branch])
                    .current_dir(&self.root)
                    .output();
            }
            bail!(
                "git merge failed: {}",
                String::from_utf8_lossy(&merge_output.stderr)
            );
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
