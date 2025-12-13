//! Job list UI rendering
//!
//! This module contains the job list panel rendering logic.

use super::super::app::{ACCENT_PURPLE, BG_SELECTED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};
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
                // Sort jobs: Running > Blocked > Queued > Pending > Done/Failed/Merged
                let mut sorted_jobs = cached_jobs.to_vec();
                sorted_jobs.sort_by(|a, b| {
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
                        .then_with(|| b.created_at.cmp(&a.created_at))
                });

                for job in &sorted_jobs {
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
                            ui.horizontal(|ui| {
                                // Status indicator with visual representation for all states
                                let status_col = status_color(job.status);
                                match job.status {
                                    JobStatus::Running => {
                                        // Animated spinner for running jobs
                                        ui.add(egui::Spinner::new().size(12.0).color(status_col));
                                    }
                                    JobStatus::Blocked => {
                                        // Lock symbol for blocked jobs waiting for file lock
                                        ui.label(RichText::new("[L]").monospace().color(status_col));
                                    }
                                    JobStatus::Queued => {
                                        // Clock/hourglass symbol for queued
                                        ui.label(RichText::new("[~]").monospace().color(status_col));
                                    }
                                    JobStatus::Pending => {
                                        // Pause/waiting symbol
                                        ui.label(RichText::new("[.]").monospace().color(status_col));
                                    }
                                    JobStatus::Done => {
                                        // Checkmark for success
                                        ui.label(RichText::new("[+]").monospace().color(status_col));
                                    }
                                    JobStatus::Failed => {
                                        // X for failure
                                        ui.label(RichText::new("[x]").monospace().color(status_col));
                                    }
                                    JobStatus::Rejected => {
                                        // Minus for rejected
                                        ui.label(RichText::new("[-]").monospace().color(status_col));
                                    }
                                    JobStatus::Merged => {
                                        // Arrow/merge symbol
                                        ui.label(RichText::new("[>]").monospace().color(status_col));
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
                                    RichText::new(format!("[{}]", job.agent_id))
                                        .color(TEXT_MUTED),
                                );

                                // Group indicator (if job is part of a multi-agent group)
                                if job.group_id.is_some() {
                                    ui.label(
                                        RichText::new("||")
                                            .color(ACCENT_PURPLE)
                                            .small(),
                                    ).on_hover_text("Part of multi-agent group");
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
                                        ).on_hover_text(hover_text);
                                    }
                                }
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
