//! Worktree operations for GitManager

use anyhow::{Context, Result, anyhow, bail};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{types::WorktreeInfo, GitManager};
use crate::JobId;

impl GitManager {
    /// Create a worktree for a job with automatic retry on conflicts.
    /// Returns the worktree path and the base branch it was created from.
    pub fn create_worktree(&self, job_id: JobId) -> Result<WorktreeInfo> {
        self.create_worktree_with_retries(job_id, 10)
    }

    /// Create a worktree for a job with configurable retry count
    pub(super) fn create_worktree_with_retries(
        &self,
        job_id: JobId,
        max_retries: u32,
    ) -> Result<WorktreeInfo> {
        if !self.has_commits() {
            bail!(
                "Cannot create worktree: repository has no commits. \
                Please make an initial commit first, or disable use_worktree in config."
            );
        }

        #[cfg(unix)]
        {
            if unsafe { libc::geteuid() } == 0 {
                bail!(
                    "Cannot create worktree: running as root. \
                    This would create files owned by root that cannot be modified later. \
                    Please run KYCo as your normal user."
                );
            }
        }

        let base_branch = self.current_branch()?;

        if self.worktrees_dir.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if let Ok(metadata) = std::fs::metadata(&self.worktrees_dir) {
                    let dir_uid = metadata.uid();
                    let current_uid = unsafe { libc::geteuid() };
                    if dir_uid == 0 && current_uid != 0 {
                        bail!(
                            "Cannot create worktree: {} is owned by root. \n\
                            Please fix the permissions with:\n\
                            sudo chown -R $(whoami) {:?}",
                            self.worktrees_dir.display(),
                            self.worktrees_dir
                        );
                    }
                }
            }
        }

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
            .args([
                "for-each-ref",
                "--format=%(refname:short)",
                "refs/heads/kyco",
            ])
            .current_dir(self.root())
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

            if worktree_path.exists() {
                existing_worktree_names.insert(worktree_dir_name.clone());
                continue;
            }

            let branch_name = format!("kyco/{}", worktree_dir_name);

            if existing_branch_names.contains(&branch_name) {
                continue;
            }

            let output = Command::new("git")
                .args(["branch", &branch_name])
                .current_dir(self.root())
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

            let output = Command::new("git")
                .args(["worktree", "add", worktree_path_str, &branch_name])
                .current_dir(self.root())
                .output()
                .context("Failed to create worktree")?;

            if output.status.success() {
                return Ok(WorktreeInfo {
                    path: worktree_path,
                    base_branch: base_branch.clone(),
                    branch_name,
                });
            }

            let stderr = String::from_utf8_lossy(&output.stderr);

            // Always clean up the branch we created since worktree creation failed
            let _ = Command::new("git")
                .args(["branch", "-D", &branch_name])
                .current_dir(self.root())
                .output();

            if stderr.contains("already exists") || stderr.contains("is already checked out") {
                // Worktree conflict, try next suffix
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

    /// Remove a worktree by its path (extracts branch name from directory name).
    pub fn remove_worktree_by_path(&self, worktree_path: &Path) -> Result<()> {
        let dir_name = worktree_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Could not extract directory name from worktree path"))?;
        let branch_name = format!("kyco/{}", dir_name);
        self.remove_worktree_by_path_and_branch(worktree_path, &branch_name)
    }

    /// Remove a worktree by path and branch name (internal implementation)
    pub(super) fn remove_worktree_by_path_and_branch(
        &self,
        worktree_path: &Path,
        branch_name: &str,
    ) -> Result<()> {
        if worktree_path.exists() {
            let worktree_path_str = worktree_path
                .to_str()
                .ok_or_else(|| anyhow!("Worktree path contains invalid UTF-8"))?;
            let output = Command::new("git")
                .args(["worktree", "remove", "--force", worktree_path_str])
                .current_dir(self.root())
                .output()
                .context("Failed to remove worktree")?;

            if !output.status.success() {
                tracing::warn!(
                    "Failed to remove worktree: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        let output = Command::new("git")
            .args(["branch", "-D", &branch_name])
            .current_dir(self.root())
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

        let output = Command::new("git") // modified files
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

        let output = Command::new("git") // untracked files
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

    /// Get untracked files in a worktree/repo.
    pub fn untracked_files(&self, worktree: &Path) -> Result<Vec<PathBuf>> {
        let output = Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard"])
            .current_dir(worktree)
            .output()
            .context("Failed to run git ls-files")?;

        if !output.status.success() {
            bail!(
                "git ls-files failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)
            .collect())
    }
}
