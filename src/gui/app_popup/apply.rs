//! Background thread logic for apply/merge operations.

use super::types::{
    ApplyTarget, ApplyThreadInput, ApplyThreadOutcome,
};

pub(super) fn run_apply_thread(input: ApplyThreadInput) -> Result<ApplyThreadOutcome, String> {
    match input {
        ApplyThreadInput::Single(input) => {
            let git =
                crate::git::GitManager::new(&input.workspace_root).map_err(|e| e.to_string())?;

            if let Some(worktree_path) = input.worktree_path {
                let base_branch = input
                    .base_branch
                    .ok_or_else(|| "Job has no base branch recorded".to_string())?;

                git.apply_changes(&worktree_path, &base_branch, Some(&input.commit_message))
                    .map_err(|e| e.to_string())?;

                let mut message = format!("Merged job #{}", input.job_id);
                if let Err(e) = git.remove_worktree_by_path(&worktree_path) {
                    message.push_str(&format!(" (cleanup warning: {})", e));
                }

                Ok(ApplyThreadOutcome {
                    target: ApplyTarget::Single {
                        job_id: input.job_id,
                    },
                    group_job_ids: Vec::new(),
                    message,
                })
            } else {
                match git.commit_root_changes(&input.commit_message) {
                    Ok(true) => Ok(ApplyThreadOutcome {
                        target: ApplyTarget::Single {
                            job_id: input.job_id,
                        },
                        group_job_ids: Vec::new(),
                        message: format!("Committed and applied job #{}", input.job_id),
                    }),
                    Ok(false) => Ok(ApplyThreadOutcome {
                        target: ApplyTarget::Single {
                            job_id: input.job_id,
                        },
                        group_job_ids: Vec::new(),
                        message: format!("Applied job #{} (no changes to commit)", input.job_id),
                    }),
                    Err(e) => Err(e.to_string()),
                }
            }
        }
        ApplyThreadInput::Group(input) => {
            let git =
                crate::git::GitManager::new(&input.workspace_root).map_err(|e| e.to_string())?;

            git.apply_changes(
                &input.selected_worktree_path,
                &input.base_branch,
                Some(&input.commit_message),
            )
            .map_err(|e| e.to_string())?;

            let mut cleanup_warnings = Vec::new();
            for (job_id, worktree_path) in &input.cleanup_worktrees {
                if let Err(e) = git.remove_worktree_by_path(worktree_path) {
                    cleanup_warnings.push(format!("Job #{}: {}", job_id, e));
                }
            }

            let message = if cleanup_warnings.is_empty() {
                format!(
                    "Merged changes from {} and cleaned up {} worktrees",
                    input.selected_agent_id,
                    input.cleanup_worktrees.len()
                )
            } else {
                format!(
                    "Merged changes from {} (cleanup warnings: {})",
                    input.selected_agent_id,
                    cleanup_warnings.join(", ")
                )
            };

            Ok(ApplyThreadOutcome {
                target: ApplyTarget::Group {
                    group_id: input.group_id,
                    selected_job_id: input.selected_job_id,
                },
                group_job_ids: input.group_job_ids,
                message,
            })
        }
    }
}
