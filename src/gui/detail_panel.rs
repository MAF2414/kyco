//! Detail panel component for the GUI
//!
//! Renders the detail panel showing job information, prompt preview,
//! action buttons, and activity log for the selected job.

use eframe::egui::{self, RichText, ScrollArea};

use crate::config::Config;
use crate::{Job, JobId, JobStatus, LogEvent, LogEventKind};

use super::app::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_SECONDARY, STATUS_QUEUED, STATUS_RUNNING,
    TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};

/// Actions that can be triggered from the detail panel
#[derive(Debug, Clone, Copy)]
pub enum DetailPanelAction {
    Queue(JobId),
    Apply(JobId),
    Reject(JobId),
    ViewDiff(JobId),
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
pub fn render_detail_panel(ui: &mut egui::Ui, state: &DetailPanelState<'_>) -> Option<DetailPanelAction> {
    let mut action: Option<DetailPanelAction> = None;

    ui.vertical(|ui| {
        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("DETAILS").monospace().color(TEXT_PRIMARY));
        });
        ui.add_space(4.0);
        ui.separator();

        if let Some(job_id) = state.selected_job_id {
            if let Some(job) = state.cached_jobs.iter().find(|j| j.id == job_id) {
                // Job info
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Job #{}", job.id)).monospace().color(TEXT_PRIMARY));
                    let status_color = status_color(job.status);
                    ui.label(RichText::new(format!("[{}]", job.status)).monospace().color(status_color));
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
                    ui.label(RichText::new(desc).small().color(TEXT_DIM));
                }

                // Show result summary if available (from ---kyco block)
                if let Some(result) = &job.result {
                    ui.add_space(8.0);
                    egui::Frame::none()
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
                                ui.label(RichText::new(details).small().color(TEXT_DIM));
                            }

                            // Stats bar
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                // Status from result
                                if let Some(status) = &result.status {
                                    let result_status_color = match status.as_str() {
                                        "success" => ACCENT_GREEN,
                                        "partial" => STATUS_RUNNING,
                                        "failed" => ACCENT_RED,
                                        _ => TEXT_MUTED,
                                    };
                                    ui.label(RichText::new(format!("● {}", status)).small().color(result_status_color));
                                    ui.add_space(8.0);
                                }

                                // Files changed
                                if let Some(stats) = &job.stats {
                                    if stats.files_changed > 0 {
                                        ui.label(RichText::new(format!("{} files", stats.files_changed)).small().color(TEXT_MUTED));
                                        ui.add_space(8.0);
                                    }
                                    if stats.lines_added > 0 || stats.lines_removed > 0 {
                                        ui.label(RichText::new(format!("+{} -{}", stats.lines_added, stats.lines_removed)).small().color(TEXT_MUTED));
                                        ui.add_space(8.0);
                                    }
                                }

                                // Duration
                                if let Some(duration) = job.duration_string() {
                                    ui.label(RichText::new(duration).small().color(TEXT_MUTED));
                                }
                            });
                        });
                } else if let Some(stats) = &job.stats {
                    // Show just stats if no result block but we have timing/files
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if stats.files_changed > 0 {
                            ui.label(RichText::new(format!("{} files changed", stats.files_changed)).small().color(TEXT_MUTED));
                            ui.add_space(8.0);
                        }
                        if let Some(duration) = job.duration_string() {
                            ui.label(RichText::new(format!("⏱ {}", duration)).small().color(TEXT_MUTED));
                        }
                    });
                }

                ui.add_space(8.0);

                // Action buttons
                let current_job_id = job.id;
                let current_status = job.status;
                ui.horizontal(|ui| {
                    match current_status {
                        JobStatus::Pending => {
                            if ui.button(RichText::new("▶ Start").color(ACCENT_GREEN)).clicked() {
                                action = Some(DetailPanelAction::Queue(current_job_id));
                            }
                        }
                        JobStatus::Done => {
                            if ui.button(RichText::new("✓ Apply").color(ACCENT_GREEN)).clicked() {
                                action = Some(DetailPanelAction::Apply(current_job_id));
                            }
                            if ui.button(RichText::new("✗ Reject").color(ACCENT_RED)).clicked() {
                                action = Some(DetailPanelAction::Reject(current_job_id));
                            }
                            if ui.button(RichText::new("Δ Diff").color(TEXT_DIM)).clicked() {
                                action = Some(DetailPanelAction::ViewDiff(current_job_id));
                            }
                        }
                        JobStatus::Running => {
                            ui.label(RichText::new("⟳ Running...").color(STATUS_RUNNING));
                        }
                        JobStatus::Queued => {
                            ui.label(RichText::new("◎ Queued").color(STATUS_QUEUED));
                        }
                        _ => {}
                    }
                });

                ui.add_space(8.0);
                ui.separator();

                // Show prompt - either sent_prompt (if job ran) or preview (before running)
                let prompt_text = job.sent_prompt.clone().unwrap_or_else(|| {
                    build_prompt_preview(job, state.config)
                });

                ui.add_space(4.0);
                let prompt_label = if job.sent_prompt.is_some() {
                    "SENT PROMPT"
                } else {
                    "PROMPT PREVIEW"
                };
                ui.label(RichText::new(prompt_label).small().monospace().color(TEXT_MUTED));
                ui.add_space(2.0);

                ScrollArea::vertical()
                    .id_salt("prompt_scroll")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut prompt_text.as_str())
                                .font(egui::TextStyle::Monospace)
                                .text_color(TEXT_DIM)
                                .desired_width(f32::INFINITY)
                                .interactive(false)
                        );
                    });

                ui.add_space(8.0);
                ui.separator();

                // Logs for this job
                ui.label(RichText::new("ACTIVITY LOG").small().monospace().color(TEXT_MUTED));

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(state.log_scroll_to_bottom)
                    .show(ui, |ui| {
                        // Show job-specific logs first
                        for event in &job.log_events {
                            let color = log_color(&event.kind);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("[{}]", event.kind)).small().monospace().color(TEXT_MUTED));
                                ui.label(RichText::new(&event.summary).small().color(color));
                            });
                        }

                        // Then show global logs
                        for event in state.logs {
                            if event.job_id.is_none() || event.job_id == Some(job.id) {
                                let color = log_color(&event.kind);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(format!("[{}]", event.kind)).small().monospace().color(TEXT_MUTED));
                                    ui.label(RichText::new(&event.summary).small().color(color));
                                });
                            }
                        }
                    });
            } else {
                ui.label(RichText::new("Job not found").color(TEXT_MUTED));
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("Select a job to view details").color(TEXT_MUTED));
            });
        }
    });

    action
}

/// Get status color for a job status
pub fn status_color(status: JobStatus) -> egui::Color32 {
    use super::app::{STATUS_DONE, STATUS_FAILED, STATUS_MERGED, STATUS_PENDING, STATUS_REJECTED};
    match status {
        JobStatus::Pending => STATUS_PENDING,
        JobStatus::Queued => STATUS_QUEUED,
        JobStatus::Running => STATUS_RUNNING,
        JobStatus::Done => STATUS_DONE,
        JobStatus::Failed => STATUS_FAILED,
        JobStatus::Rejected => STATUS_REJECTED,
        JobStatus::Merged => STATUS_MERGED,
    }
}

/// Get log event color
fn log_color(kind: &LogEventKind) -> egui::Color32 {
    match kind {
        LogEventKind::Thought => TEXT_DIM,
        LogEventKind::ToolCall => ACCENT_CYAN,
        LogEventKind::ToolOutput => TEXT_MUTED,
        LogEventKind::Text => TEXT_PRIMARY,
        LogEventKind::Error => ACCENT_RED,
        LogEventKind::System => ACCENT_GREEN,
    }
}

/// Build prompt preview for a job (before it runs)
fn build_prompt_preview(job: &Job, config: &Config) -> String {
    // Get agent config to access mode templates
    let agent_config = config.get_agent(&job.agent_id).unwrap_or_default();
    let template = agent_config.get_mode_template(&job.mode);

    let file_path = job.source_file.display().to_string();
    let line = job.source_line;
    let description = job.description.as_deref().unwrap_or("");

    // Build the main prompt
    let prompt = template
        .prompt_template
        .replace("{file}", &file_path)
        .replace("{line}", &line.to_string())
        .replace("{target}", &job.target)
        .replace("{mode}", &job.mode)
        .replace("{description}", description)
        .replace("{scope_type}", "file");

    // Build system prompt if available
    let mut full_prompt = String::new();

    if let Some(system_prompt) = &template.system_prompt {
        full_prompt.push_str("=== SYSTEM PROMPT ===\n");
        full_prompt.push_str(system_prompt);
        full_prompt.push_str("\n\n");
    }

    full_prompt.push_str("=== USER PROMPT ===\n");
    full_prompt.push_str(&prompt);

    full_prompt
}
