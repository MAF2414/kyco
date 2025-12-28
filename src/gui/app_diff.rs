//! Diff-related methods for KycoApp
//!
//! Contains methods for loading and displaying git diffs.

use super::app::KycoApp;
use super::app_types::ViewMode;
use crate::{JobId, LogEvent};

impl KycoApp {
    /// Open the diff view for a job
    pub(crate) fn open_job_diff(&mut self, job_id: JobId, return_view: ViewMode) {
        let Some(job) = self.cached_jobs.iter().find(|j| j.id == job_id).cloned() else {
            self.logs
                .push(LogEvent::error(format!("Job #{} not found", job_id)));
            return;
        };

        let workspace_root = self.workspace_root_for_job(&job);
        let gm = match crate::git::GitManager::new(&workspace_root) {
            Ok(gm) => gm,
            Err(e) => {
                self.logs.push(LogEvent::error(format!(
                    "Failed to initialize git manager for {}: {}",
                    workspace_root.display(),
                    e
                )));
                return;
            }
        };

        let diff_result =
            if let Some(worktree) = job.git_worktree_path.as_ref().filter(|p| p.exists()) {
                gm.diff(worktree, job.base_branch.as_deref())
                    .map(|mut diff| {
                        if let Ok(untracked) = gm.untracked_files(worktree) {
                            if !untracked.is_empty() {
                                if !diff.is_empty() {
                                    diff.push_str("\n\n");
                                }
                                diff.push_str("--- Untracked files ---\n");
                                for file in untracked {
                                    diff.push_str(&file.display().to_string());
                                    diff.push('\n');
                                }
                            }
                        }
                        diff
                    })
            } else {
                gm.diff(&workspace_root, None).map(|mut diff| {
                    let mut header = "--- Workspace changes (no worktree) ---\n\n".to_string();
                    if let Ok(untracked) = gm.untracked_files(&workspace_root) {
                        if !untracked.is_empty() {
                            if !diff.is_empty() {
                                diff.push_str("\n\n");
                            }
                            diff.push_str("--- Untracked files ---\n");
                            for file in untracked {
                                diff.push_str(&file.display().to_string());
                                diff.push('\n');
                            }
                        }
                    }

                    if diff.is_empty() {
                        header.push_str("No changes in workspace.");
                        header
                    } else {
                        format!("{}{}", header, diff)
                    }
                })
            };

        match diff_result {
            Ok(content) => {
                self.diff_state.set_content(content);
                self.diff_return_view = return_view;
                self.view_mode = ViewMode::DiffView;
            }
            Err(e) => {
                self.logs
                    .push(LogEvent::error(format!("Failed to load diff: {}", e)));
            }
        }
    }

    /// Load inline diff for the currently selected job (for detail panel display)
    pub(crate) fn load_inline_diff_for_selected(&mut self) {
        let Some(job_id) = self.selected_job_id else {
            self.inline_diff_content = None;
            return;
        };

        let Some(job) = self.cached_jobs.iter().find(|j| j.id == job_id).cloned() else {
            self.inline_diff_content = None;
            return;
        };

        // Only load diff for completed jobs with changes
        if job.status != crate::JobStatus::Done {
            self.inline_diff_content = None;
            return;
        }

        let workspace_root = self.workspace_root_for_job(&job);
        let gm = match crate::git::GitManager::new(&workspace_root) {
            Ok(gm) => gm,
            Err(_) => {
                self.inline_diff_content = None;
                return;
            }
        };

        let diff_result: Option<String> =
            if let Some(worktree) = job.git_worktree_path.as_ref().filter(|p| p.exists()) {
                gm.diff(worktree, job.base_branch.as_deref())
                    .ok()
                    .map(|mut diff| {
                        if let Ok(untracked) = gm.untracked_files(worktree) {
                            if !untracked.is_empty() {
                                if !diff.is_empty() {
                                    diff.push_str("\n\n");
                                }
                                diff.push_str("--- Untracked files ---\n");
                                for file in untracked {
                                    diff.push_str(&file.display().to_string());
                                    diff.push('\n');
                                }
                            }
                        }
                        diff
                    })
            } else {
                // No worktree - show workspace diff
                gm.diff(&workspace_root, None).ok()
            };

        self.inline_diff_content = diff_result.filter(|d| !d.is_empty());
    }
}
