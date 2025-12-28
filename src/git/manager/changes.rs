//! Change application and commit operations for GitManager

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

use super::types::CommitMessage;
use super::GitManager;

impl GitManager {
    /// Merge a worktree branch into the specified base branch
    ///
    /// This performs a proper git merge of the worktree's branch into the base branch.
    /// If there are uncommitted changes in the worktree, they are committed first.
    /// The base_branch parameter specifies which branch to merge into.
    pub fn apply_changes(
        &self,
        worktree: &Path,
        base_branch: &str,
        commit_message: Option<&CommitMessage>,
    ) -> Result<()> {
        // Avoid merging into a dirty working tree.
        // We ignore untracked files here (e.g., `.kyco/` artifacts) and only block
        // on tracked/staged changes that would make the merge surprising or unsafe.
        if self.has_tracked_uncommitted_changes()? {
            bail!(
                "Cannot apply changes: repository has uncommitted changes. \
                 Please commit or stash them first."
            );
        }

        let status_output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(worktree)
            .output()
            .context("Failed to check worktree status")?;

        if !status_output.stdout.is_empty() {
            let fallback = CommitMessage {
                subject: "Auto-commit remaining changes before merge".to_string(),
                body: None,
            };
            let message = commit_message.unwrap_or(&fallback);
            let _ = self.commit_all_in_dir(worktree, message)?;
        }

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

        let current_branch = self.current_branch()?;
        let should_restore_branch = current_branch != base_branch && current_branch != "HEAD";

        if current_branch != base_branch {
            let checkout_output = Command::new("git")
                .args(["checkout", base_branch])
                .current_dir(self.root())
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

        let merge_output = Command::new("git")
            .args(["merge", &worktree_branch, "--no-edit"])
            .current_dir(self.root())
            .output()
            .context("Failed to merge branch")?;

        if !merge_output.status.success() {
            let stderr = String::from_utf8_lossy(&merge_output.stderr)
                .trim()
                .to_string();

            // Try to abort merge so we don't leave the user's repo in a conflicted "merge in progress" state.
            let aborted = Command::new("git")
                .args(["merge", "--abort"])
                .current_dir(self.root())
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            // If merge failed and we changed branches, try to restore original branch.
            if should_restore_branch {
                let _ = Command::new("git")
                    .args(["checkout", &current_branch])
                    .current_dir(self.root())
                    .output();
            }

            if aborted {
                bail!("git merge failed (merge was aborted): {}", stderr);
            }

            bail!(
                "git merge failed (could not abort merge; try `git merge --abort`): {}",
                stderr
            );
        }

        // Restore the original branch (avoid surprising the user by leaving the repo on base_branch).
        if should_restore_branch {
            let checkout_output = Command::new("git")
                .args(["checkout", &current_branch])
                .current_dir(self.root())
                .output()
                .context("Failed to restore original branch after merge")?;

            if !checkout_output.status.success() {
                tracing::warn!(
                    "Failed to restore branch '{}': {}",
                    current_branch,
                    String::from_utf8_lossy(&checkout_output.stderr)
                );
            }
        }

        Ok(())
    }

    /// Commit current changes in the repository root.
    ///
    /// Returns `true` if a commit was created.
    pub fn commit_root_changes(&self, commit_message: &CommitMessage) -> Result<bool> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(self.root())
            .output()
            .context("Failed to check repo status")?;

        if output.stdout.is_empty() {
            return Ok(false);
        }

        self.commit_all_in_dir(self.root(), commit_message)
    }

    pub(super) fn commit_all_in_dir(&self, dir: &Path, commit_message: &CommitMessage) -> Result<bool> {
        let add_output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(dir)
            .output()
            .context("Failed to stage changes")?;

        if !add_output.status.success() {
            bail!(
                "git add failed: {}",
                String::from_utf8_lossy(&add_output.stderr).trim()
            );
        }

        let mut commit_cmd = Command::new("git");
        commit_cmd
            .arg("commit")
            .arg("-m")
            .arg(&commit_message.subject);
        if let Some(body) = commit_message
            .body
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            commit_cmd.arg("-m").arg(body);
        }

        let commit_output = commit_cmd
            .current_dir(dir)
            .output()
            .context("Failed to commit changes")?;

        if commit_output.status.success() {
            return Ok(true);
        }

        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        if stderr.contains("nothing to commit") {
            tracing::debug!("git commit reported nothing to commit: {}", stderr);
            return Ok(false);
        }

        bail!("git commit failed: {}", stderr.trim());
    }
}
