//! Apply/merge popup types and logic
//!
//! This module contains the types and functions for the apply/merge confirmation popup.
//! The actual popup rendering is done in app.rs, but the supporting types and
//! the apply thread logic are extracted here for better organization.

mod apply;
mod render;
mod types;

pub(crate) use types::{
    ApplyTarget, ApplyThreadInput, ApplyThreadOutcome, GroupApplyInput, SingleApplyInput,
};

use super::app::KycoApp;
use apply::run_apply_thread;
use types::ApplyTarget as Target;

impl KycoApp {
    /// Build ApplyThreadInput from the current apply target
    pub(crate) fn build_apply_thread_input(
        &self,
        target: &Target,
    ) -> Result<ApplyThreadInput, String> {
        match target {
            Target::Single { job_id } => {
                let job = self
                    .job_manager
                    .lock()
                    .map_err(|_| "Failed to lock job manager".to_string())?
                    .get(*job_id)
                    .cloned()
                    .ok_or_else(|| format!("Job #{} not found", job_id))?;

                let workspace_root = self.workspace_root_for_job(&job);
                Ok(ApplyThreadInput::Single(SingleApplyInput {
                    job_id: *job_id,
                    workspace_root,
                    worktree_path: job.git_worktree_path.clone(),
                    base_branch: job.base_branch.clone(),
                    commit_message: crate::git::CommitMessage::from_job(&job),
                }))
            }
            Target::Group {
                group_id,
                selected_job_id,
            } => {
                let group = self
                    .group_manager
                    .lock()
                    .map_err(|_| "Failed to lock group manager".to_string())?
                    .get(*group_id)
                    .cloned()
                    .ok_or_else(|| format!("Group #{} not found", group_id))?;

                if !matches!(
                    group.status,
                    crate::GroupStatus::Comparing | crate::GroupStatus::Selected
                ) {
                    return Err(format!(
                        "Group #{} is not ready to merge yet (status: {})",
                        group_id, group.status
                    ));
                }

                if !group.job_ids.contains(selected_job_id) {
                    return Err(format!(
                        "Selected job #{} is not part of group #{}",
                        selected_job_id, group_id
                    ));
                }

                let manager = self
                    .job_manager
                    .lock()
                    .map_err(|_| "Failed to lock job manager".to_string())?;

                let selected_job = manager
                    .get(*selected_job_id)
                    .cloned()
                    .ok_or_else(|| format!("Selected job #{} not found", selected_job_id))?;

                let selected_worktree_path = selected_job
                    .git_worktree_path
                    .clone()
                    .ok_or_else(|| "Selected job has no worktree".to_string())?;

                let base_branch = selected_job
                    .base_branch
                    .clone()
                    .ok_or_else(|| "Selected job has no base branch recorded".to_string())?;

                let cleanup_worktrees: Vec<(crate::JobId, std::path::PathBuf)> = group
                    .job_ids
                    .iter()
                    .filter_map(|&job_id| {
                        manager
                            .get(job_id)
                            .and_then(|j| j.git_worktree_path.clone().map(|p| (job_id, p)))
                    })
                    .collect();

                let workspace_root = self.workspace_root_for_job(&selected_job);
                Ok(ApplyThreadInput::Group(GroupApplyInput {
                    group_id: *group_id,
                    selected_job_id: *selected_job_id,
                    selected_agent_id: selected_job.agent_id.clone(),
                    workspace_root,
                    selected_worktree_path,
                    base_branch,
                    commit_message: crate::git::CommitMessage::from_job(&selected_job),
                    cleanup_worktrees,
                    group_job_ids: group.job_ids.clone(),
                }))
            }
        }
    }

    /// Start the apply/merge operation in a background thread
    pub(crate) fn start_apply_confirm_merge(&mut self) {
        if self.apply_confirm_rx.is_some() {
            return;
        }

        let Some(target) = self.apply_confirm_target.clone() else {
            self.apply_confirm_error = Some("No merge target selected".to_string());
            return;
        };

        let input = match self.build_apply_thread_input(&target) {
            Ok(input) => input,
            Err(e) => {
                self.apply_confirm_error = Some(e);
                return;
            }
        };

        self.apply_confirm_error = None;
        let (tx, rx) = std::sync::mpsc::channel();
        self.apply_confirm_rx = Some(rx);

        std::thread::spawn(move || {
            let result = run_apply_thread(input);
            let _ = tx.send(result);
        });
    }
}
