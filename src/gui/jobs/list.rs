//! Job list UI rendering
//!
//! This module contains the job list panel rendering logic.

use super::super::app::{BG_SELECTED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};
use super::super::detail_panel::status_color;
use crate::{Job, JobStatus};
use eframe::egui::{self, Color32, RichText, ScrollArea};

/// Render the job list panel
pub fn render_job_list(
    ui: &mut egui::Ui,
    cached_jobs: &[Job],
    selected_job_id: &mut Option<u64>,
) {
    // Request repaint for animation if any job is running
    let has_running_job = cached_jobs.iter().any(|j| j.status == JobStatus::Running);
    if has_running_job {
        ui.ctx().request_repaint();
    }

    ui.vertical(|ui| {
        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("JOBS").monospace().color(TEXT_PRIMARY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} total", cached_jobs.len()))
                        .small()
                        .color(TEXT_MUTED),
                );
            });
        });
        ui.add_space(4.0);
        ui.separator();

        // Job list
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Sort jobs: Running > Queued > Pending > Done/Failed/Merged
                let mut sorted_jobs = cached_jobs.to_vec();
                sorted_jobs.sort_by(|a, b| {
                    let priority = |s: JobStatus| match s {
                        JobStatus::Running => 0,
                        JobStatus::Queued => 1,
                        JobStatus::Pending => 2,
                        JobStatus::Done => 3,
                        JobStatus::Failed => 4,
                        JobStatus::Rejected => 5,
                        JobStatus::Merged => 6,
                    };
                    priority(a.status)
                        .cmp(&priority(b.status))
                        .then_with(|| b.created_at.cmp(&a.created_at))
                });

                for job in &sorted_jobs {
                    let is_selected = *selected_job_id == Some(job.id);
                    let bg = if is_selected {
                        BG_SELECTED
                    } else {
                        Color32::TRANSPARENT
                    };

                    let response = egui::Frame::none()
                        .fill(bg)
                        .inner_margin(egui::vec2(8.0, 4.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Status indicator with animation for running jobs
                                let status_col = status_color(job.status);
                                if job.status == JobStatus::Running {
                                    // Use egui's built-in Spinner for smooth animation
                                    ui.add(egui::Spinner::new().size(12.0).color(status_col));
                                } else {
                                    ui.label(RichText::new("●").color(status_col));
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
                                    RichText::new(format!("[{}]", job.agent_id))
                                        .color(TEXT_MUTED),
                                );
                            });

                            // Target (truncated)
                            let target = if job.target.len() > 40 {
                                format!("{}...", &job.target[..40])
                            } else {
                                job.target.clone()
                            };
                            ui.label(RichText::new(target).color(TEXT_DIM));
                        });

                    if response.response.interact(egui::Sense::click()).clicked() {
                        *selected_job_id = Some(job.id);
                    }
                }
            });
    });
}
