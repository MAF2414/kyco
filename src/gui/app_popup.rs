//! Apply/merge popup types and logic
//!
//! This module contains the types and functions for the apply/merge confirmation popup.
//! The actual popup rendering is done in app.rs, but the supporting types and
//! the apply thread logic are extracted here for better organization.

use super::app::KycoApp;
use super::app_types::ViewMode;
use super::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_PRIMARY, BG_SECONDARY, TEXT_DIM, TEXT_MUTED,
    TEXT_PRIMARY,
};
use crate::{AgentGroupId, JobId};
use eframe::egui::{self, RichText, Stroke, Vec2};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) enum ApplyTarget {
    Single {
        job_id: JobId,
    },
    Group {
        group_id: AgentGroupId,
        selected_job_id: JobId,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct ApplyThreadOutcome {
    pub(super) target: ApplyTarget,
    /// For group merges: all job IDs in the group (empty for single jobs).
    pub(super) group_job_ids: Vec<JobId>,
    pub(super) message: String,
}

#[derive(Debug, Clone)]
pub(crate) enum ApplyThreadInput {
    Single(SingleApplyInput),
    Group(GroupApplyInput),
}

#[derive(Debug, Clone)]
pub(crate) struct SingleApplyInput {
    pub(super) job_id: JobId,
    pub(super) workspace_root: PathBuf,
    pub(super) worktree_path: Option<PathBuf>,
    pub(super) base_branch: Option<String>,
    pub(super) commit_message: crate::git::CommitMessage,
}

#[derive(Debug, Clone)]
pub(crate) struct GroupApplyInput {
    pub(super) group_id: AgentGroupId,
    pub(super) selected_job_id: JobId,
    pub(super) selected_agent_id: String,
    pub(super) workspace_root: PathBuf,
    pub(super) selected_worktree_path: PathBuf,
    pub(super) base_branch: String,
    pub(super) commit_message: crate::git::CommitMessage,
    pub(super) cleanup_worktrees: Vec<(JobId, PathBuf)>,
    pub(super) group_job_ids: Vec<JobId>,
}

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

// ═══════════════════════════════════════════════════════════════════════════
// KycoApp methods for apply/merge operations
// ═══════════════════════════════════════════════════════════════════════════

impl KycoApp {
    /// Build ApplyThreadInput from the current apply target
    pub(crate) fn build_apply_thread_input(
        &self,
        target: &ApplyTarget,
    ) -> Result<ApplyThreadInput, String> {
        match target {
            ApplyTarget::Single { job_id } => {
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
            ApplyTarget::Group {
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

                let cleanup_worktrees: Vec<(JobId, PathBuf)> = group
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

    /// Render the apply/merge confirmation popup
    pub(crate) fn render_apply_confirm_popup(&mut self, ctx: &egui::Context) {
        let Some(target) = self.apply_confirm_target.clone() else {
            self.view_mode = self.apply_confirm_return_view;
            return;
        };

        let in_progress = self.apply_confirm_rx.is_some();
        let validation_error = self.build_apply_thread_input(&target).err();

        let title = match &target {
            ApplyTarget::Single { job_id } => format!("Merge Job #{}", job_id),
            ApplyTarget::Group { group_id, .. } => format!("Merge Group #{}", group_id),
        };
        let mut description_lines: Vec<String> = Vec::new();
        let mut selected_job_id_for_diff: Option<JobId> = None;
        let mut warning: Option<String> = None;

        match &target {
            ApplyTarget::Single { job_id } => {
                let job = self.cached_jobs.iter().find(|j| j.id == *job_id);
                if let Some(job) = job {
                    selected_job_id_for_diff = Some(job.id);
                    let workspace_root = self.workspace_root_for_job(job);
                    description_lines.push(format!("Repo: {}", workspace_root.display()));
                    description_lines.push(format!("Agent: {}", job.agent_id));
                    description_lines.push(format!("Mode: {}", job.mode));
                    description_lines.push(format!("Target: {}", job.target));

                    let subject = crate::git::CommitMessage::from_job(job).subject;
                    description_lines.push(format!("Commit: {}", subject));

                    if let Some(worktree) = &job.git_worktree_path {
                        description_lines.push(format!("Worktree: {}", worktree.display()));
                        let base = job.base_branch.as_deref().unwrap_or("<unknown>");
                        description_lines.push(format!("Merge into: {}", base));
                    } else {
                        warning = Some(
                            "No worktree: this will commit ALL current changes in the repo."
                                .to_string(),
                        );
                    }
                } else {
                    warning = Some("Job not found".to_string());
                }
            }
            ApplyTarget::Group {
                group_id,
                selected_job_id,
            } => {
                selected_job_id_for_diff = Some(*selected_job_id);
                let group = self
                    .group_manager
                    .lock()
                    .ok()
                    .and_then(|gm| gm.get(*group_id).cloned());
                if let Some(group) = group {
                    description_lines.push(format!("Group status: {}", group.status));
                    description_lines.push(format!("Mode: {}", group.mode));
                    description_lines.push(format!("Target: {}", group.target));
                }

                let job = self.cached_jobs.iter().find(|j| j.id == *selected_job_id);
                if let Some(job) = job {
                    let workspace_root = self.workspace_root_for_job(job);
                    description_lines.push(format!("Repo: {}", workspace_root.display()));
                    description_lines
                        .push(format!("Selected result: #{} ({})", job.id, job.agent_id));

                    let subject = crate::git::CommitMessage::from_job(job).subject;
                    description_lines.push(format!("Commit: {}", subject));

                    if let Some(worktree) = &job.git_worktree_path {
                        description_lines.push(format!("Worktree: {}", worktree.display()));
                    }
                    if let Some(base) = job.base_branch.as_deref() {
                        description_lines.push(format!("Merge into: {}", base));
                    }

                    warning = Some(
                        "Other group jobs will be marked Rejected and all group worktrees will be deleted."
                            .to_string(),
                    );
                } else {
                    warning = Some("Selected job not found".to_string());
                }
            }
        }

        egui::Window::new("Merge Confirmation")
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(620.0, 360.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(
                egui::Frame::default()
                    .fill(BG_PRIMARY)
                    .stroke(Stroke::new(2.0, ACCENT_CYAN))
                    .inner_margin(16.0)
                    .corner_radius(8.0),
            )
            .show(ctx, |ui| {
                ui.label(RichText::new(title).size(18.0).strong().color(TEXT_PRIMARY));
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                for line in &description_lines {
                    ui.label(RichText::new(line).color(TEXT_DIM));
                }

                if let Some(w) = &warning {
                    ui.add_space(8.0);
                    ui.label(RichText::new(w).color(ACCENT_RED));
                }

                if let Some(err) = &validation_error {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Cannot merge yet: {}", err))
                            .color(ACCENT_RED),
                    );
                }

                if let Some(err) = &self.apply_confirm_error {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("Error: {}", err)).color(ACCENT_RED));
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(12.0);

                if in_progress {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Merging...").color(TEXT_DIM));
                    });
                } else {
                    ui.label(
                        RichText::new("Tip: If a merge conflict occurs, the merge is aborted to keep your repo clean.")
                            .small()
                            .color(TEXT_MUTED),
                    );
                }

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_merge = !in_progress && validation_error.is_none();
                        let merge_btn = egui::Button::new(
                            RichText::new("✓ Merge")
                                .color(if can_merge { BG_PRIMARY } else { TEXT_MUTED }),
                        )
                        .fill(if can_merge { ACCENT_GREEN } else { BG_SECONDARY });

                        if ui.add_enabled(can_merge, merge_btn).clicked() {
                            self.start_apply_confirm_merge();
                        }

                        ui.add_space(8.0);

                        if ui
                            .add_enabled(
                                !in_progress,
                                egui::Button::new(RichText::new("View Diff").color(TEXT_DIM)),
                            )
                            .clicked()
                        {
                            if let Some(job_id) = selected_job_id_for_diff {
                                self.open_job_diff(job_id, ViewMode::ApplyConfirmPopup);
                            }
                        }

                        ui.add_space(8.0);

                        if ui
                            .add_enabled(
                                !in_progress,
                                egui::Button::new(RichText::new("Cancel").color(TEXT_DIM)),
                            )
                            .clicked()
                        {
                            self.apply_confirm_target = None;
                            self.apply_confirm_error = None;
                            self.view_mode = self.apply_confirm_return_view;
                        }
                    });
                });
            });
    }
}
