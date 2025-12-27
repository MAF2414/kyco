//! Job list UI rendering
//!
//! This module contains the job list panel rendering logic.

use super::super::animations::{blocked_indicator, pending_indicator, queued_indicator};
use super::super::app::{
    ACCENT_CYAN, ACCENT_PURPLE, ACCENT_RED, BG_HIGHLIGHT, BG_SECONDARY, BG_SELECTED, TEXT_DIM,
    TEXT_MUTED, TEXT_PRIMARY,
};
use super::super::detail_panel::status_color;
use crate::{Job, JobId, JobStatus};
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke};

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

    /// Get count of jobs matching this filter
    pub fn count(&self, jobs: &[Job]) -> usize {
        jobs.iter().filter(|j| self.matches(j)).count()
    }
}

/// Action returned from job list rendering
#[derive(Debug, Clone)]
pub enum JobListAction {
    /// No action
    None,
    /// Delete the specified job
    DeleteJob(JobId),
    /// Delete all finished jobs
    DeleteAllFinished,
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

    // Pre-calculate counts for each filter
    let count_all = cached_jobs.len();
    let count_active = JobListFilter::Active.count(cached_jobs);
    let count_finished = JobListFilter::Finished.count(cached_jobs);
    let count_failed = JobListFilter::Failed.count(cached_jobs);

    ui.vertical(|ui| {
        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("JOBS").monospace().color(TEXT_PRIMARY));

            // Spacer to push Clear All button to the right
            let remaining = ui.available_width();
            if count_finished > 0 {
                // Calculate button width (approximately)
                let btn_width = 60.0;
                if remaining > btn_width {
                    ui.add_space(remaining - btn_width);
                }

                let clear_btn = egui::Button::new(
                    RichText::new("Clear All").small().color(TEXT_DIM),
                )
                .fill(BG_SECONDARY)
                .stroke(Stroke::new(1.0, TEXT_MUTED));

                if ui
                    .add(clear_btn)
                    .on_hover_text(format!("Delete all {} finished jobs", count_finished))
                    .clicked()
                {
                    action = JobListAction::DeleteAllFinished;
                }
            }
        });

        ui.add_space(4.0);

        // Filter buttons as pill-shaped tabs with counts
        ui.horizontal(|ui| {
            for (filter_option, count) in [
                (JobListFilter::All, count_all),
                (JobListFilter::Active, count_active),
                (JobListFilter::Finished, count_finished),
                (JobListFilter::Failed, count_failed),
            ] {
                let is_selected = *filter == filter_option;
                let label = filter_option.label();

                // Format label with count
                let label_with_count = if count > 0 {
                    format!("{} ({})", label, count)
                } else {
                    label.to_string()
                };

                // Style based on selection and filter type
                let (text_color, bg_color) = if is_selected {
                    (ACCENT_CYAN, BG_HIGHLIGHT)
                } else if count > 0 {
                    (TEXT_DIM, BG_SECONDARY)
                } else {
                    (TEXT_MUTED, Color32::TRANSPARENT)
                };

                // Special highlight for failed jobs with count > 0 (but not when selected)
                let text_color = if filter_option == JobListFilter::Failed && count > 0 && !is_selected {
                    ACCENT_RED
                } else {
                    text_color
                };

                let btn = egui::Button::new(RichText::new(&label_with_count).small().color(text_color))
                    .fill(bg_color)
                    .corner_radius(4.0);

                if ui.add(btn).clicked() {
                    *filter = filter_option;
                }

                ui.add_space(2.0);
            }
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

                            // Second row: filename + delete button
                            // Use fixed width layout to prevent overflow
                            let row_width = available_width - 16.0; // Account for margins
                            ui.horizontal(|ui| {
                                ui.set_width(row_width);

                                // Target - show only filename (full path available in detail panel)
                                let target = std::path::Path::new(&job.target)
                                    .file_name()
                                    .and_then(|f| f.to_str())
                                    .unwrap_or(&job.target);

                                // Calculate max width for filename (row minus button space)
                                let btn_space = if job.is_finished() { 32.0 } else { 0.0 };
                                let max_filename_width = row_width - btn_space;

                                // Truncate filename to fit available space (roughly 6px per char)
                                let max_chars = ((max_filename_width / 6.5) as usize).saturating_sub(2);
                                let display_target = if target.len() > max_chars && max_chars > 3 {
                                    format!("{}…", &target[..max_chars])
                                } else {
                                    target.to_string()
                                };

                                ui.label(RichText::new(&display_target).color(TEXT_DIM))
                                    .on_hover_text(&job.target);

                                // Delete button (only for finished jobs)
                                if job.is_finished() {
                                    // Use right-to-left layout for delete button
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let delete_btn = egui::Button::new(
                                            RichText::new("✕").color(ACCENT_RED).size(12.0),
                                        )
                                        .fill(Color32::TRANSPARENT)
                                        .stroke(Stroke::NONE)
                                        .min_size(egui::vec2(20.0, 18.0));

                                        if ui
                                            .add(delete_btn)
                                            .on_hover_text("Delete this job")
                                            .clicked()
                                        {
                                            action = JobListAction::DeleteJob(job.id);
                                        }
                                    });
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
