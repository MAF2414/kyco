//! Main detail panel rendering

use eframe::egui::{self, RichText, ScrollArea};
use egui_extras::{Size, StripBuilder};

use crate::config::Config;
use crate::{Job, JobId, JobStatus, LogEvent};

use crate::gui::app::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_SECONDARY, STATUS_QUEUED, STATUS_RUNNING, TEXT_DIM,
    TEXT_MUTED, TEXT_PRIMARY,
};

use super::colors::{log_color, status_color};
use super::prompt::build_prompt_preview;

/// Actions that can be triggered from the detail panel
#[derive(Debug, Clone, Copy)]
pub enum DetailPanelAction {
    Queue(JobId),
    Apply(JobId),
    Reject(JobId),
    ViewDiff(JobId),
    /// Stop/kill a running job
    Kill(JobId),
    /// Mark a REPL job as complete (user confirms they finished in Terminal)
    MarkComplete(JobId),
}

/// State required for rendering the detail panel
pub struct DetailPanelState<'a> {
    pub selected_job_id: Option<u64>,
    pub cached_jobs: &'a [Job],
    pub logs: &'a [LogEvent],
    pub config: &'a Config,
    pub log_scroll_to_bottom: bool,
}

/// Render the detail panel and return any action triggered by the user
pub fn render_detail_panel(
    ui: &mut egui::Ui,
    state: &DetailPanelState<'_>,
) -> Option<DetailPanelAction> {
    let mut action: Option<DetailPanelAction> = None;

    render_header(ui);

    if let Some(job_id) = state.selected_job_id {
        if let Some(job) = state.cached_jobs.iter().find(|j| j.id == job_id) {
            // First render the fixed-height job info section
            render_job_info(ui, job);
            render_result_section(ui, job);

            ui.add_space(8.0);

            action = render_action_buttons(ui, job);

            ui.add_space(8.0);
            ui.separator();

            // Use StripBuilder to allocate remaining space between prompt and activity log
            // This ensures both sections are always visible and share the available space
            let available_height = ui.available_height();

            // Reserve minimum heights for each section
            let min_section_height = 100.0;
            let separator_height = 20.0; // spacing + separator

            // Calculate proportional heights (40% prompt, 60% activity log)
            let usable_height = (available_height - separator_height).max(min_section_height * 2.0);
            let prompt_height = (usable_height * 0.4).max(min_section_height);
            let log_height = (usable_height * 0.6).max(min_section_height);

            StripBuilder::new(ui)
                .size(Size::exact(prompt_height))
                .size(Size::exact(separator_height))
                .size(Size::remainder().at_least(log_height))
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        render_prompt_section(ui, job, state.config);
                    });
                    strip.cell(|ui| {
                        ui.add_space(8.0);
                        ui.separator();
                    });
                    strip.cell(|ui| {
                        render_activity_log(ui, job, state.logs, state.log_scroll_to_bottom);
                    });
                });
        } else {
            ui.label(RichText::new("Job not found").color(TEXT_MUTED));
        }
    } else {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("Select a job to view details").color(TEXT_MUTED));
        });
    }

    action
}

/// Render the panel header
fn render_header(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("DETAILS").monospace().color(TEXT_PRIMARY));
    });
    ui.add_space(4.0);
    ui.separator();
}

/// Render job information section
fn render_job_info(ui: &mut egui::Ui, job: &Job) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("Job #{}", job.id))
                .monospace()
                .color(TEXT_PRIMARY),
        );
        let color = status_color(job.status);
        ui.label(
            RichText::new(format!("[{}]", job.status))
                .monospace()
                .color(color),
        );
    });

    ui.horizontal(|ui| {
        ui.label(RichText::new("Mode:").color(TEXT_MUTED));
        ui.label(RichText::new(&job.mode).color(TEXT_PRIMARY));
        ui.label(RichText::new("Agent:").color(TEXT_MUTED));
        ui.label(RichText::new(&job.agent_id).color(ACCENT_CYAN));
    });

    ui.horizontal(|ui| {
        ui.label(RichText::new("Target:").color(TEXT_MUTED));
        ui.label(RichText::new(&job.target).color(TEXT_DIM));
    });

    if let Some(desc) = &job.description {
        ui.add_space(4.0);
        ui.label(RichText::new(desc).color(TEXT_DIM));
    }
}

/// Render result section (from ---kyco block)
fn render_result_section(ui: &mut egui::Ui, job: &Job) {
    if let Some(result) = &job.result {
        ui.add_space(8.0);
        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                // Title
                if let Some(title) = &result.title {
                    ui.label(RichText::new(title).monospace().color(TEXT_PRIMARY));
                }

                // Details
                if let Some(details) = &result.details {
                    ui.add_space(4.0);
                    ui.label(RichText::new(details).color(TEXT_DIM));
                }

                // Summary (if present)
                if let Some(summary) = &result.summary {
                    if !summary.is_empty() {
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui.label(RichText::new("Summary:").small().color(TEXT_MUTED));
                        ui.label(RichText::new(summary).color(TEXT_DIM).small());
                    }
                }

                // Stats bar
                ui.add_space(8.0);
                render_stats_bar(ui, job, result);
            });
    } else if let Some(stats) = &job.stats {
        // Show just stats if no result block but we have timing/files
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if stats.files_changed > 0 {
                ui.label(
                    RichText::new(format!("{} files changed", stats.files_changed))
                        .color(TEXT_MUTED),
                );
                ui.add_space(8.0);
            }
            if let Some(duration) = job.duration_string() {
                ui.label(RichText::new(format!("⏱ {}", duration)).color(TEXT_MUTED));
            }
        });
    }
}

/// Render stats bar within result section
fn render_stats_bar(ui: &mut egui::Ui, job: &Job, result: &crate::JobResult) {
    ui.horizontal(|ui| {
        // Status from result
        if let Some(status) = &result.status {
            let result_status_color = match status.as_str() {
                "success" => ACCENT_GREEN,
                "partial" => STATUS_RUNNING,
                "failed" => ACCENT_RED,
                _ => TEXT_MUTED,
            };
            ui.label(RichText::new(format!("● {}", status)).color(result_status_color));
            ui.add_space(8.0);
        }

        // Files changed
        if let Some(stats) = &job.stats {
            if stats.files_changed > 0 {
                ui.label(RichText::new(format!("{} files", stats.files_changed)).color(TEXT_MUTED));
                ui.add_space(8.0);
            }
            if stats.lines_added > 0 || stats.lines_removed > 0 {
                ui.label(
                    RichText::new(format!("+{} -{}", stats.lines_added, stats.lines_removed))
                        .color(TEXT_MUTED),
                );
                ui.add_space(8.0);
            }
        }

        // Duration
        if let Some(duration) = job.duration_string() {
            ui.label(RichText::new(duration).color(TEXT_MUTED));
        }
    });
}

/// Render action buttons and return any triggered action
fn render_action_buttons(ui: &mut egui::Ui, job: &Job) -> Option<DetailPanelAction> {
    let mut action = None;
    let current_job_id = job.id;
    let current_status = job.status;

    ui.horizontal(|ui| {
        match current_status {
            JobStatus::Pending => {
                if ui
                    .button(RichText::new("▶ Start").color(ACCENT_GREEN))
                    .clicked()
                {
                    action = Some(DetailPanelAction::Queue(current_job_id));
                }
            }
            JobStatus::Done => {
                if ui
                    .button(RichText::new("✓ Apply").color(ACCENT_GREEN))
                    .clicked()
                {
                    action = Some(DetailPanelAction::Apply(current_job_id));
                }
                if ui
                    .button(RichText::new("✗ Reject").color(ACCENT_RED))
                    .clicked()
                {
                    action = Some(DetailPanelAction::Reject(current_job_id));
                }
                if ui
                    .button(RichText::new("Δ Diff").color(TEXT_DIM))
                    .clicked()
                {
                    action = Some(DetailPanelAction::ViewDiff(current_job_id));
                }
            }
            JobStatus::Running => {
                ui.label(RichText::new("⟳ Running...").color(STATUS_RUNNING));
                ui.add_space(8.0);

                if job.is_repl {
                    // REPL jobs: user can mark as complete when they're done in Terminal
                    if ui
                        .button(RichText::new("✓ Mark Complete").color(ACCENT_GREEN))
                        .on_hover_text("Mark this Terminal session as complete")
                        .clicked()
                    {
                        action = Some(DetailPanelAction::MarkComplete(current_job_id));
                    }
                } else {
                    // Print mode jobs: can be stopped/killed
                    if ui
                        .button(RichText::new("■ Stop").color(ACCENT_RED))
                        .on_hover_text("Stop this job")
                        .clicked()
                    {
                        action = Some(DetailPanelAction::Kill(current_job_id));
                    }
                }
            }
            JobStatus::Queued => {
                ui.label(RichText::new("◎ Queued").color(STATUS_QUEUED));
            }
            _ => {}
        }
    });

    action
}

/// Render prompt section
fn render_prompt_section(ui: &mut egui::Ui, job: &Job, config: &Config) {
    // Show prompt - either sent_prompt (if job ran) or preview (before running)
    let prompt_text = job
        .sent_prompt
        .clone()
        .unwrap_or_else(|| build_prompt_preview(job, config));

    let prompt_label = if job.sent_prompt.is_some() {
        "SENT PROMPT"
    } else {
        "PROMPT PREVIEW"
    };
    ui.label(RichText::new(prompt_label).monospace().color(TEXT_MUTED));
    ui.add_space(2.0);

    // Use all available height in this section (space is allocated by StripBuilder)
    ScrollArea::vertical()
        .id_salt("prompt_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut prompt_text.as_str())
                    .font(egui::TextStyle::Monospace)
                    .text_color(TEXT_DIM)
                    .desired_width(f32::INFINITY)
                    .interactive(false),
            );
        });
}

/// Render activity log section
fn render_activity_log(ui: &mut egui::Ui, job: &Job, logs: &[LogEvent], scroll_to_bottom: bool) {
    ui.label(RichText::new("ACTIVITY LOG").monospace().color(TEXT_MUTED));

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(scroll_to_bottom)
        .show(ui, |ui| {
            // Show job-specific logs first
            for event in &job.log_events {
                let color = log_color(&event.kind);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("[{}]", event.kind))
                            .monospace()
                            .color(TEXT_MUTED),
                    );
                    ui.label(RichText::new(&event.summary).color(color));
                });
            }

            // Then show global logs
            for event in logs {
                if event.job_id.is_none() || event.job_id == Some(job.id) {
                    let color = log_color(&event.kind);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("[{}]", event.kind))
                                .monospace()
                                .color(TEXT_MUTED),
                        );
                        ui.label(RichText::new(&event.summary).color(color));
                    });
                }
            }
        });
}
