//! Rendering helpers for job list items

use super::types::JobListAction;
use crate::gui::animations::{blocked_indicator, pending_indicator, queued_indicator};
use crate::gui::detail_panel::status_color;
use crate::gui::theme::{ACCENT_CYAN, ACCENT_PURPLE, ACCENT_RED, BG_SELECTED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};
use crate::{Job, JobStatus};
use chrono::{DateTime, Utc};
use eframe::egui::{self, Color32, RichText, Stroke};

/// Format a duration in milliseconds to human readable string
fn format_duration_ms(ms: i64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else if ms < 3_600_000 {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        if secs > 0 {
            format!("{}m {}s", mins, secs)
        } else {
            format!("{}m", mins)
        }
    } else {
        let hours = ms / 3_600_000;
        let mins = (ms % 3_600_000) / 60_000;
        format!("{}h {}m", hours, mins)
    }
}

/// Format relative time (e.g., "2m ago", "1h ago")
fn format_time_ago(time: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(time);
    let secs = diff.num_seconds();

    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

/// Generate a color from a string, optimized for dark theme visibility
fn color_from_string(s: &str) -> Color32 {
    // Simple hash function
    let hash: u32 = s.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));

    // Map hash to hue (0-360)
    let hue = (hash % 360) as f32;
    // Fixed saturation and lightness for dark theme readability
    let saturation: f32 = 0.65;
    let lightness: f32 = 0.65;

    // HSL to RGB conversion
    let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let m = lightness - c / 2.0;

    let (r, g, b) = match (hue as u32) / 60 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Color32::from_rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

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

                if let Some(wt_path) = &job.git_worktree_path {
                    let wt_name = wt_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("worktree");
                    ui.label(RichText::new("⎇").color(ACCENT_CYAN).small())
                        .on_hover_text(format!("Worktree: {}", wt_name));
                }

                render_blocked_info(ui, job);

                // Show state if available (for finished jobs)
                if let Some(ref result) = job.result {
                    if let Some(ref state) = result.state {
                        let state_color = color_from_string(state);
                        ui.label(RichText::new(format!("[{}]", state)).small().color(state_color));
                    }
                }

                // Right-align time info
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    render_time_info(ui, job);
                });
            });

            render_target_row(ui, job, available_width, action);
        })
        .response
}

/// Render time information for a job (started/finished timestamps)
fn render_time_info(ui: &mut egui::Ui, job: &Job) {
    match job.status {
        JobStatus::Running => {
            // Show how long it's been running
            if let Some(started) = job.started_at {
                let elapsed = Utc::now().signed_duration_since(started).num_milliseconds();
                let text = format_duration_ms(elapsed);
                ui.label(RichText::new(text).small().color(TEXT_DIM))
                    .on_hover_text(format!("Started: {}", started.format("%H:%M:%S")));
            }
        }
        JobStatus::Done | JobStatus::Failed | JobStatus::Rejected | JobStatus::Merged => {
            // Show when finished and duration
            if let Some(finished) = job.finished_at {
                let ago = format_time_ago(finished);
                let duration_text = if let Some(started) = job.started_at {
                    let duration = finished.signed_duration_since(started).num_milliseconds();
                    format!("{} ({})", ago, format_duration_ms(duration))
                } else {
                    ago
                };
                ui.label(RichText::new(duration_text).small().color(TEXT_DIM))
                    .on_hover_text(format!("Finished: {}", finished.format("%H:%M:%S")));
            }
        }
        JobStatus::Queued | JobStatus::Pending | JobStatus::Blocked => {
            // Show when created
            let ago = format_time_ago(job.created_at);
            ui.label(RichText::new(ago).small().color(TEXT_DIM))
                .on_hover_text(format!("Created: {}", job.created_at.format("%H:%M:%S")));
        }
    }
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
                        .small();

                if ui
                    .add(delete_btn)
                    .on_hover_text("Delete this job")
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    *action = JobListAction::DeleteJob(job.id);
                }
            });
        }
    });
}
