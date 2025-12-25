//! Job list UI rendering
//!
//! This module contains the job list panel rendering logic.

use super::super::animations::{blocked_indicator, pending_indicator, queued_indicator};
use super::super::app::{
    ACCENT_CYAN, ACCENT_PURPLE, ACCENT_RED, BG_SELECTED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use super::super::detail_panel::status_color;
use crate::{Job, JobId, JobStatus};
use eframe::egui::{self, Color32, RichText, ScrollArea};

/// Filter options for job list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JobListFilter {
    /// Show all jobs
    #[default]
    All,
    /// Show only active jobs (Running, Blocked, Queued, Pending)
    Active,
    /// Show only finished jobs (Done, Failed, Rejected, Merged)
    Finished,
    /// Show only failed jobs
    Failed,
}

impl JobListFilter {
    /// Check if a job matches this filter
    pub fn matches(&self, job: &Job) -> bool {
        match self {
            JobListFilter::All => true,
            JobListFilter::Active => !job.is_finished(),
            JobListFilter::Finished => job.is_finished(),
            JobListFilter::Failed => job.status == JobStatus::Failed,
        }
    }

    /// Get display label for this filter
    pub fn label(&self) -> &'static str {
        match self {
            JobListFilter::All => "All",
            JobListFilter::Active => "Active",
            JobListFilter::Finished => "Done",
            JobListFilter::Failed => "Failed",
        }
    }
}

/// Action returned from job list rendering
#[derive(Debug, Clone)]
pub enum JobListAction {
    /// No action
    None,
    /// Delete the specified job
    DeleteJob(JobId),
}

/// Render the job list panel
pub fn render_job_list(
    ui: &mut egui::Ui,
    cached_jobs: &[Job],
    selected_job_id: &mut Option<u64>,
    filter: &mut JobListFilter,
) -> JobListAction {
    let mut action = JobListAction::None;
    // Request repaint for animation if any job has an animated status indicator
    let has_animated_job = cached_jobs.iter().any(|j| {
        matches!(
            j.status,
            JobStatus::Running | JobStatus::Blocked | JobStatus::Queued | JobStatus::Pending
        )
    });
    if has_animated_job {
        ui.ctx().request_repaint();
    }

    ui.vertical(|ui| {
        // Header with filter buttons
        ui.horizontal(|ui| {
            ui.label(RichText::new("JOBS").monospace().color(TEXT_PRIMARY));
            ui.add_space(8.0);

            // Filter buttons
            for filter_option in [
                JobListFilter::All,
                JobListFilter::Active,
                JobListFilter::Finished,
                JobListFilter::Failed,
            ] {
                let is_selected = *filter == filter_option;
                let label = filter_option.label();
                let text = if is_selected {
                    RichText::new(label).color(ACCENT_CYAN).small()
                } else {
                    RichText::new(label).color(TEXT_MUTED).small()
                };

                if ui
                    .add(egui::Button::new(text).frame(false))
                    .on_hover_text(format!("Filter: {}", label))
                    .clicked()
                {
                    *filter = filter_option;
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Count filtered jobs
                let filtered_count = cached_jobs.iter().filter(|j| filter.matches(j)).count();
                let total_count = cached_jobs.len();
                let count_text = if filtered_count == total_count {
                    format!("{} total", total_count)
                } else {
                    format!("{}/{}", filtered_count, total_count)
                };
                ui.label(RichText::new(count_text).small().color(TEXT_MUTED));
            });
        });
        ui.add_space(4.0);
        ui.separator();

        // Job list - get available width before ScrollArea
        let available_width = ui.available_width();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(available_width);

                // Filter and sort jobs
                let mut filtered_jobs: Vec<&Job> = cached_jobs
                    .iter()
                    .filter(|j| filter.matches(j))
                    .collect();

                // Sort by status priority, then by date
                filtered_jobs.sort_by(|a, b| {
                    let priority = |s: JobStatus| match s {
                        JobStatus::Running => 0,
                        JobStatus::Blocked => 1,
                        JobStatus::Queued => 2,
                        JobStatus::Pending => 3,
                        JobStatus::Done => 4,
                        JobStatus::Failed => 5,
                        JobStatus::Rejected => 6,
                        JobStatus::Merged => 7,
                    };
                    priority(a.status)
                        .cmp(&priority(b.status))
                        .then_with(|| b.updated_at.cmp(&a.updated_at))
                });

                for job in filtered_jobs {
                    let is_selected = *selected_job_id == Some(job.id);
                    let bg = if is_selected {
                        BG_SELECTED
                    } else {
                        Color32::TRANSPARENT
                    };

                    let response = egui::Frame::NONE
                        .fill(bg)
                        .inner_margin(egui::vec2(8.0, 4.0))
                        .show(ui, |ui| {
                            ui.set_min_width(available_width - 16.0); // Account for margins
                            ui.horizontal(|ui| {
                                // Status indicator with visual representation for all states
                                let status_col = status_color(job.status);
                                match job.status {
                                    JobStatus::Running => {
                                        // Animated spinner for running jobs
                                        ui.add(egui::Spinner::new().size(12.0).color(status_col));
                                    }
                                    JobStatus::Blocked => {
                                        // Pulsing lock symbol for blocked jobs waiting for file lock
                                        blocked_indicator(ui, status_col, 12.0);
                                    }
                                    JobStatus::Queued => {
                                        // Animated dots cycling for queued jobs
                                        queued_indicator(ui, status_col, 12.0);
                                    }
                                    JobStatus::Pending => {
                                        // Gentle breathing pulse for pending jobs
                                        pending_indicator(ui, status_col, 12.0);
                                    }
                                    JobStatus::Done => {
                                        // Checkmark for success
                                        ui.label(
                                            RichText::new("[+]").monospace().color(status_col),
                                        );
                                    }
                                    JobStatus::Failed => {
                                        // X for failure
                                        ui.label(
                                            RichText::new("[x]").monospace().color(status_col),
                                        );
                                    }
                                    JobStatus::Rejected => {
                                        // Minus for rejected
                                        ui.label(
                                            RichText::new("[-]").monospace().color(status_col),
                                        );
                                    }
                                    JobStatus::Merged => {
                                        // Arrow/merge symbol
                                        ui.label(
                                            RichText::new("[>]").monospace().color(status_col),
                                        );
                                    }
                                }

                                // Job ID
                                ui.label(
                                    RichText::new(format!("#{}", job.id))
                                        .monospace()
                                        .color(TEXT_DIM),
                                );

                                // Mode
                                ui.label(RichText::new(&job.mode).monospace().color(TEXT_PRIMARY));

                                // Agent
                                ui.label(
                                    RichText::new(format!("[{}]", job.agent_id)).color(TEXT_MUTED),
                                );

                                // Group indicator (if job is part of a multi-agent group)
                                if job.group_id.is_some() {
                                    ui.label(RichText::new("||").color(ACCENT_PURPLE).small())
                                        .on_hover_text("Part of multi-agent group");
                                }

                                // Blocked indicator (shows which job is blocking this one)
                                if job.status == JobStatus::Blocked {
                                    if let Some(blocked_by) = job.blocked_by {
                                        let hover_text = if let Some(ref file) = job.blocked_file {
                                            format!(
                                                "Waiting for Job #{} to release {}",
                                                blocked_by,
                                                file.file_name()
                                                    .map(|f| f.to_string_lossy().to_string())
                                                    .unwrap_or_else(|| file.display().to_string())
                                            )
                                        } else {
                                            format!("Waiting for Job #{}", blocked_by)
                                        };
                                        ui.label(
                                            RichText::new(format!("-> #{}", blocked_by))
                                                .small()
                                                .color(status_color(JobStatus::Blocked)),
                                        )
                                        .on_hover_text(hover_text);
                                    }
                                }
                            });

                            ui.horizontal(|ui| {
                                // Target - show only filename (full path available in detail panel)
                                let target = std::path::Path::new(&job.target)
                                    .file_name()
                                    .and_then(|f| f.to_str())
                                    .unwrap_or(&job.target);
                                ui.label(RichText::new(target).color(TEXT_DIM))
                                    .on_hover_text(&job.target);

                                // Delete button (only for finished jobs, on right side)
                                if job.is_finished() {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new("×")
                                                            .color(ACCENT_RED)
                                                            .small(),
                                                    )
                                                    .frame(false),
                                                )
                                                .on_hover_text("Delete job")
                                                .clicked()
                                            {
                                                action = JobListAction::DeleteJob(job.id);
                                            }
                                        },
                                    );
                                }
                            });
                        });

                    if response.response.interact(egui::Sense::click()).clicked() {
                        *selected_job_id = Some(job.id);
                    }
                }
            });
    });

    action
}
