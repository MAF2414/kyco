//! Rendering helpers for job list items

use super::types::JobListAction;
use crate::gui::animations::{blocked_indicator, pending_indicator, queued_indicator};
use crate::gui::detail_panel::status_color;
use crate::gui::theme::{ACCENT_PURPLE, ACCENT_RED, BG_SELECTED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};
use crate::{Job, JobStatus};
use eframe::egui::{self, Color32, RichText, Stroke};

/// Render the status indicator for a job
pub fn render_status_indicator(ui: &mut egui::Ui, job: &Job) {
    let status_col = status_color(job.status);
    match job.status {
        JobStatus::Running => {
            ui.add(egui::Spinner::new().size(12.0).color(status_col));
        }
        JobStatus::Blocked => {
            blocked_indicator(ui, status_col, 12.0);
        }
        JobStatus::Queued => {
            queued_indicator(ui, status_col, 12.0);
        }
        JobStatus::Pending => {
            pending_indicator(ui, status_col, 12.0);
        }
        JobStatus::Done => {
            ui.label(RichText::new("[+]").monospace().color(status_col));
        }
        JobStatus::Failed => {
            ui.label(RichText::new("[x]").monospace().color(status_col));
        }
        JobStatus::Rejected => {
            ui.label(RichText::new("[-]").monospace().color(status_col));
        }
        JobStatus::Merged => {
            ui.label(RichText::new("[>]").monospace().color(status_col));
        }
    }
}

/// Render blocked job info (waiting for another job)
pub fn render_blocked_info(ui: &mut egui::Ui, job: &Job) {
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
}

/// Render a single job row in the list
pub fn render_job_row(
    ui: &mut egui::Ui,
    job: &Job,
    is_selected: bool,
    available_width: f32,
    action: &mut JobListAction,
) -> egui::Response {
    let bg = if is_selected {
        BG_SELECTED
    } else {
        Color32::TRANSPARENT
    };

    egui::Frame::NONE
        .fill(bg)
        .inner_margin(egui::vec2(8.0, 4.0))
        .show(ui, |ui| {
            ui.set_min_width(available_width - 16.0);
            ui.horizontal(|ui| {
                render_status_indicator(ui, job);

                ui.label(
                    RichText::new(format!("#{}", job.id))
                        .monospace()
                        .color(TEXT_DIM),
                );
                ui.label(RichText::new(&job.mode).monospace().color(TEXT_PRIMARY));
                ui.label(RichText::new(format!("[{}]", job.agent_id)).color(TEXT_MUTED));

                if job.group_id.is_some() {
                    ui.label(RichText::new("||").color(ACCENT_PURPLE).small())
                        .on_hover_text("Part of multi-agent group");
                }

                render_blocked_info(ui, job);
            });

            render_target_row(ui, job, available_width, action);
        })
        .response
}

/// Render the target file row with delete button for finished jobs
fn render_target_row(
    ui: &mut egui::Ui,
    job: &Job,
    available_width: f32,
    action: &mut JobListAction,
) {
    let row_width = available_width - 16.0;
    ui.horizontal(|ui| {
        ui.set_width(row_width);

        let target = std::path::Path::new(&job.target)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(&job.target);

        let btn_space = if job.is_finished() { 32.0 } else { 0.0 };
        let max_filename_width = row_width - btn_space;

        let max_chars = ((max_filename_width / 6.5) as usize).saturating_sub(2);
        let display_target = if target.chars().count() > max_chars && max_chars > 3 {
            let truncate_byte_idx = target
                .char_indices()
                .nth(max_chars)
                .map(|(idx, _)| idx)
                .unwrap_or(target.len());
            format!("{}…", &target[..truncate_byte_idx])
        } else {
            target.to_string()
        };

        ui.label(RichText::new(&display_target).color(TEXT_DIM))
            .on_hover_text(&job.target);

        if job.is_finished() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let delete_btn =
                    egui::Button::new(RichText::new("✕").color(ACCENT_RED).size(12.0))
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(egui::vec2(20.0, 18.0));

                if ui
                    .add(delete_btn)
                    .on_hover_text("Delete this job")
                    .clicked()
                {
                    *action = JobListAction::DeleteJob(job.id);
                }
            });
        }
    });
}
