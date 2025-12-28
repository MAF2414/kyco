//! Git manager implementation

use anyhow::{Context, Result, anyhow, bail};
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
                if let Some(body) = r
                    .commit_body
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    return Some(body.to_string());
                }

                let mut paragraphs = Vec::new();
                if let Some(details) = r
                    .details
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    paragraphs.push(details.to_string());
                }
                if let Some(summary) = r
                    .summary
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
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
    let mut out: String = first_line
        .chars()
        .filter(|c| *c != '\r' && *c != '\n')
        .collect();
    if out.is_empty() {
        out = "kyco: update".to_string();
    }

    const MAX_LEN: usize = 72;
    if out.chars().count() > MAX_LEN {
        out = out.chars().take(MAX_LEN).collect();
    }

    out
}

/// Status of a file in a diff
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed { from: String },
    Copied { from: String },
    Untracked,
}

/// Diff information for a single file
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub status: FileStatus,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub is_binary: bool,
    pub patch: Option<String>,
}

/// Aggregated diff report
#[derive(Debug, Clone)]
pub struct DiffReport {
    pub files: Vec<FileDiff>,
    pub total_added: usize,
    pub total_removed: usize,
    pub files_changed: usize,
}

/// Options for diff generation
#[derive(Debug, Clone, Default)]
pub struct DiffSettings {
    pub ignore_whitespace: bool,
    pub context_lines: u32,
    pub include_untracked: bool,
}

/// Parse NUL-delimited output from git commands
fn parse_null_delimited(output: &[u8]) -> Vec<String> {
    output
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .filter_map(|s| std::str::from_utf8(s).ok())
        .map(|s| s.to_string())
        .collect()
}

/// Parse git diff --numstat -z output
/// Returns tuples of (path, lines_added, lines_removed, is_binary)
fn parse_numstat_output(output: &[u8]) -> Vec<(String, usize, usize, bool)> {
    let text = String::from_utf8_lossy(output);
    let mut results = Vec::new();

    for line in text.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }

        let (added, removed, is_binary) = if parts[0] == "-" && parts[1] == "-" {
            // Binary file
            (0, 0, true)
        } else {
            let added = parts[0].parse().unwrap_or(0);
            let removed = parts[1].parse().unwrap_or(0);
            (added, removed, false)
        };

        // Handle renames: "old_path\tnew_path" or just "path"
        let path = if parts.len() > 3 {
            // Rename format with NUL: the path after rename info
            parts[2].to_string()
        } else {
            parts[2].to_string()
        };

        // Skip empty paths
        if !path.is_empty() {
            results.push((path, added, removed, is_binary));
        }
    }

    results
}

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
    worktrees_dir: PathBuf,
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
    fn create_worktree_with_retries(
        &self,
        job_id: JobId,
        max_retries: u32,
    ) -> Result<WorktreeInfo> {
        // Check if the repository has commits - worktrees require at least one commit
        if !self.has_commits() {
            bail!(
                "Cannot create worktree: repository has no commits. \
                Please make an initial commit first, or disable use_worktree in config."
            );
        }

        // Refuse to create worktrees when running as root to avoid permission issues
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

        // Check if worktrees directory exists and has wrong ownership
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

            let output = Command::new("git")
                .args(["worktree", "add", worktree_path_str, &branch_name])
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

            // Always clean up the branch we created since worktree creation failed
            let _ = Command::new("git")
                .args(["branch", "-D", &branch_name])
                .current_dir(&self.root)
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
    fn remove_worktree_by_path_and_branch(
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

        let merge_output = Command::new("git")
            .args(["merge", &worktree_branch, "--no-edit"])
            .current_dir(&self.root)
            .output()
            .context("Failed to merge branch")?;

        if !merge_output.status.success() {
            let stderr = String::from_utf8_lossy(&merge_output.stderr)
                .trim()
                .to_string();

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

    /// Generate a diff report for a worktree compared to a base branch
    ///
    /// Returns structured information about all changed files including
    /// line counts, file status, and binary detection.
    pub fn diff_report(
        &self,
        worktree: &Path,
        base_branch: Option<&str>,
        settings: &DiffSettings,
    ) -> Result<DiffReport> {
        // Determine the base commit
        let base_commit = if let Some(base) = base_branch.map(str::trim).filter(|s| !s.is_empty()) {
            let output = Command::new("git")
                .args(["merge-base", base, "HEAD"])
                .current_dir(worktree)
                .output()
                .context("Failed to run git merge-base")?;

            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        } else {
            None
        };

        let mut files = Vec::new();
        let mut tracked_paths = std::collections::HashSet::new();

        // Get diff stats for tracked files
        let mut diff_args = vec!["diff", "--numstat"];
        if settings.ignore_whitespace {
            diff_args.push("-w");
        }

        let range = if let Some(ref base) = base_commit {
            format!("{}..HEAD", base)
        } else {
            "HEAD".to_string()
        };
        diff_args.push(&range);

        let output = Command::new("git")
            .args(&diff_args)
            .current_dir(worktree)
            .output()
            .context("Failed to run git diff --numstat")?;

        if output.status.success() {
            for (path, added, removed, is_binary) in parse_numstat_output(&output.stdout) {
                tracked_paths.insert(path.clone());
                files.push(FileDiff {
                    path,
                    status: FileStatus::Modified,
                    lines_added: added,
                    lines_removed: removed,
                    is_binary,
                    patch: None,
                });
            }
        }

        // Also check for uncommitted changes vs HEAD
        let uncommitted_output = Command::new("git")
            .args(["diff", "--numstat", "HEAD"])
            .current_dir(worktree)
            .output()
            .context("Failed to run git diff --numstat HEAD")?;

        if uncommitted_output.status.success() {
            for (path, added, removed, is_binary) in
                parse_numstat_output(&uncommitted_output.stdout)
            {
                if !tracked_paths.contains(&path) {
                    tracked_paths.insert(path.clone());
                    files.push(FileDiff {
                        path,
                        status: FileStatus::Modified,
                        lines_added: added,
                        lines_removed: removed,
                        is_binary,
                        patch: None,
                    });
                }
            }
        }

        // Get untracked files if requested
        if settings.include_untracked {
            let untracked_output = Command::new("git")
                .args(["ls-files", "--others", "--exclude-standard", "-z"])
                .current_dir(worktree)
                .output()
                .context("Failed to run git ls-files")?;

            if untracked_output.status.success() {
                for path in parse_null_delimited(&untracked_output.stdout) {
                    if !tracked_paths.contains(&path) {
                        // Count lines in untracked file
                        let file_path = worktree.join(&path);
                        let lines_added = if file_path.exists() {
                            std::fs::read_to_string(&file_path)
                                .map(|content| content.lines().count())
                                .unwrap_or(0)
                        } else {
                            0
                        };

                        files.push(FileDiff {
                            path,
                            status: FileStatus::Untracked,
                            lines_added,
                            lines_removed: 0,
                            is_binary: false,
                            patch: None,
                        });
                    }
                }
            }
        }

        // Calculate totals
        let total_added: usize = files.iter().map(|f| f.lines_added).sum();
        let total_removed: usize = files.iter().map(|f| f.lines_removed).sum();
        let files_changed = files.len();

        Ok(DiffReport {
            files,
            total_added,
            total_removed,
            files_changed,
        })
    }

    /// Get the patch for a specific file (lazy loading)
    ///
    /// This generates the full patch content for a single file.
    pub fn diff_file_patch(
        &self,
        worktree: &Path,
        file_path: &str,
        base_commit: Option<&str>,
        settings: &DiffSettings,
    ) -> Result<String> {
        let mut args = vec!["diff", "--no-color"];

        if settings.ignore_whitespace {
            args.push("-w");
        }

        if settings.context_lines > 0 {
            // We need to format this as a string that lives long enough
            let context_arg = format!("-U{}", settings.context_lines);
            let mut args_with_context = args.clone();
            args_with_context.push(&context_arg);

            if let Some(base) = base_commit {
                args_with_context.push(base);
            } else {
                args_with_context.push("HEAD");
            }

            args_with_context.push("--");
            args_with_context.push(file_path);

            let output = Command::new("git")
                .args(&args_with_context)
                .current_dir(worktree)
                .output()
                .context("Failed to run git diff for file")?;

            if !output.status.success() {
                bail!(
                    "git diff failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }

        if let Some(base) = base_commit {
            args.push(base);
        } else {
            args.push("HEAD");
        }

        args.push("--");
        args.push(file_path);

        let output = Command::new("git")
            .args(&args)
            .current_dir(worktree)
            .output()
            .context("Failed to run git diff for file")?;

        if !output.status.success() {
            bail!(
                "git diff failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
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

    #[test]
    fn parse_numstat_output_basic() {
        use super::parse_numstat_output;

        let output = b"10\t5\tfile.rs\n3\t0\tnew_file.txt\n";
        let results = parse_numstat_output(output);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0], ("file.rs".to_string(), 10, 5, false));
        assert_eq!(results[1], ("new_file.txt".to_string(), 3, 0, false));
    }

    #[test]
    fn parse_numstat_output_binary() {
        use super::parse_numstat_output;

        let output = b"-\t-\timage.png\n";
        let results = parse_numstat_output(output);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], ("image.png".to_string(), 0, 0, true));
    }

    #[test]
    fn diff_report_basic() {
        use super::{DiffSettings, FileStatus};

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
        use super::DiffSettings;

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
}
