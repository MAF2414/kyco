//! Git manager implementation

use anyhow::{anyhow, bail, Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Job, JobId};

/// Result of creating a worktree
pub struct WorktreeInfo {
    /// Path to the created worktree
    pub path: PathBuf,
    /// The base branch from which the worktree was created
    pub base_branch: String,
}

/// Suggested git commit message (subject + optional body).
#[derive(Debug, Clone)]
pub struct CommitMessage {
    pub subject: String,
    pub body: Option<String>,
}

impl CommitMessage {
    pub fn from_job(job: &Job) -> Self {
        let subject = job
            .result
            .as_ref()
            .and_then(|r| {
                r.commit_subject
                    .as_deref()
                    .or(r.title.as_deref())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
            })
            .map(sanitize_commit_subject)
            .unwrap_or_else(|| sanitize_commit_subject(&format!("{}: {}", job.mode, job.target)));

        let body = job
            .result
            .as_ref()
            .and_then(|r| {
                if let Some(body) = r.commit_body.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    return Some(body.to_string());
                }

                let mut paragraphs = Vec::new();
                if let Some(details) = r.details.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    paragraphs.push(details.to_string());
                }
                if let Some(summary) = r.summary.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    paragraphs.push(summary.to_string());
                }

                if paragraphs.is_empty() {
                    None
                } else {
                    Some(paragraphs.join("\n\n"))
                }
            })
            .map(|mut body| {
                // Add lightweight traceability without spamming the subject.
                body.push_str(&format!("\n\nKYCO-Job: #{}", job.id));
                body
            });

        Self { subject, body }
    }
}

fn sanitize_commit_subject(raw: &str) -> String {
    // Keep the subject single-line and reasonably short.
    let first_line = raw.lines().next().unwrap_or("").trim();
    let mut out: String = first_line.chars().filter(|c| *c != '\r' && *c != '\n').collect();
    if out.is_empty() {
        out = "kyco: update".to_string();
    }

    const MAX_LEN: usize = 72;
    if out.chars().count() > MAX_LEN {
        out = out.chars().take(MAX_LEN).collect();
    }

    out
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

    /// Get the diff for a worktree (shows all changes vs base branch)
    ///
    /// This shows both committed and uncommitted changes in the worktree
    /// compared to the base branch (master/main).
    pub fn diff(&self, worktree: &Path, base_branch: Option<&str>) -> Result<String> {
        let mut result = String::new();

        // Get diff of committed changes vs base branch when available.
        if let Some(base_branch) = base_branch.map(str::trim).filter(|s| !s.is_empty()) {
            let range = format!("{}...HEAD", base_branch);
            let committed_output = Command::new("git")
                .args(["diff", "--no-color", &range])
                .current_dir(worktree)
                .output()
                .context("Failed to run git diff for committed changes")?;

            if committed_output.status.success() {
                let committed_diff = String::from_utf8_lossy(&committed_output.stdout);
                if !committed_diff.is_empty() {
                    result.push_str(&committed_diff);
                }
            } else {
                tracing::warn!(
                    "Failed to compute committed diff vs '{}': {}",
                    base_branch,
                    String::from_utf8_lossy(&committed_output.stderr)
                );
            }
        }

        // Also get uncommitted changes (in case agent didn't commit everything)
        let uncommitted_output = Command::new("git")
            .args(["diff", "--no-color", "HEAD"])
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

        // First, check if there are uncommitted changes and commit them
        let status_output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(worktree)
            .output()
            .context("Failed to check worktree status")?;

        if !status_output.stdout.is_empty() {
            // There are uncommitted changes - commit them so the merge is clean.
            let fallback = CommitMessage {
                subject: "Auto-commit remaining changes before merge".to_string(),
                body: None,
            };
            let message = commit_message.unwrap_or(&fallback);
            let _ = self.commit_all_in_dir(worktree, message)?;
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
        let should_restore_branch = current_branch != base_branch && current_branch != "HEAD";

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
            let stderr = String::from_utf8_lossy(&merge_output.stderr).trim().to_string();

            // Try to abort merge so we don't leave the user's repo in a conflicted "merge in progress" state.
            let aborted = Command::new("git")
                .args(["merge", "--abort"])
                .current_dir(&self.root)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            // If merge failed and we changed branches, try to restore original branch.
            if should_restore_branch {
                let _ = Command::new("git")
                    .args(["checkout", &current_branch])
                    .current_dir(&self.root)
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
                .current_dir(&self.root)
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
            .current_dir(&self.root)
            .output()
            .context("Failed to check repo status")?;

        if output.stdout.is_empty() {
            return Ok(false);
        }

        self.commit_all_in_dir(&self.root, commit_message)
    }

    fn commit_all_in_dir(&self, dir: &Path, commit_message: &CommitMessage) -> Result<bool> {
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
        commit_cmd.arg("commit").arg("-m").arg(&commit_message.subject);
        if let Some(body) = commit_message.body.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
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

#[cfg(test)]
mod tests {
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
}
