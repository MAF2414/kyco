//! Render logic for the apply/merge confirmation popup.

use super::types::ApplyTarget;
use crate::gui::app::KycoApp;
use crate::gui::app_types::ViewMode;
use crate::gui::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_PRIMARY, BG_SECONDARY, TEXT_DIM, TEXT_MUTED,
    TEXT_PRIMARY,
};
use crate::JobId;
use eframe::egui::{self, RichText, Stroke, Vec2};

impl KycoApp {
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
                    description_lines.push(format!("Skill: {}", job.skill));
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
                    description_lines.push(format!("Skill: {}", group.skill));
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
