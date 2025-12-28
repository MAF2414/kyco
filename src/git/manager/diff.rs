//! Diff operations for GitManager

use anyhow::{Context, Result, anyhow, bail};
use std::path::Path;
use std::process::Command;

use super::types::{DiffReport, DiffSettings, FileDiff, FileStatus, parse_null_delimited, parse_numstat_output};
use super::GitManager;

impl GitManager {
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
