//! Agent card rendering for the comparison popup

use eframe::egui::{self, RichText};

use crate::gui::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_HIGHLIGHT, BG_PRIMARY, BG_SECONDARY, BG_SELECTED,
    STATUS_DONE, STATUS_FAILED, STATUS_RUNNING, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::{Job, JobStatus};

pub(super) enum CardAction {
    Select,
    ViewDiff,
}

/// Render a single agent card
pub(super) fn render_agent_card(
    ui: &mut egui::Ui,
    agent_name: &str,
    job: Option<&Job>,
    is_selected: bool,
) -> Option<CardAction> {
    let mut action = None;

    let bg_color = if is_selected {
        BG_SELECTED
    } else {
        BG_SECONDARY
    };
    let border_color = if is_selected {
        ACCENT_CYAN
    } else {
        BG_HIGHLIGHT
    };

    egui::Frame::default()
        .fill(bg_color)
        .stroke(egui::Stroke::new(2.0, border_color))
        .inner_margin(12.0)
        .corner_radius(6.0)
        .show(ui, |ui| {
            ui.set_min_width(160.0);
            ui.set_max_width(180.0);
            ui.set_min_height(220.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(agent_name)
                        .color(TEXT_PRIMARY)
                        .size(14.0)
                        .strong(),
                );
                if is_selected {
                    ui.label(RichText::new("★").color(ACCENT_CYAN));
                }
            });

            ui.add_space(8.0);

            if let Some(job) = job {
                let (status_text, status_color) = match job.status {
                    JobStatus::Running => ("⟳ Running...", STATUS_RUNNING),
                    JobStatus::Done => ("✓ Done", STATUS_DONE),
                    JobStatus::Failed => ("✗ Failed", STATUS_FAILED),
                    JobStatus::Pending => ("○ Pending", TEXT_MUTED),
                    JobStatus::Queued => ("~ Queued", TEXT_DIM),
                    JobStatus::Blocked => ("⏸ Blocked", TEXT_DIM),
                    JobStatus::Rejected => ("- Rejected", ACCENT_RED),
                    JobStatus::Merged => ("> Merged", ACCENT_GREEN),
                };
                ui.label(RichText::new(status_text).color(status_color));

                ui.add_space(8.0);

                if let Some(stats) = &job.stats {
                    ui.label(
                        RichText::new(format!("{} files", stats.files_changed))
                            .color(TEXT_DIM)
                            .small(),
                    );
                    ui.label(
                        RichText::new(format!("+{} -{}", stats.lines_added, stats.lines_removed))
                            .color(TEXT_DIM)
                            .small(),
                    );
                }

                if let Some(duration_str) = job.duration_string() {
                    ui.label(RichText::new(duration_str).color(TEXT_MUTED).small());
                }

                ui.add_space(8.0);

                if let Some(result) = &job.result {
                    if let Some(title) = &result.title {
                        ui.label(RichText::new(truncate(title, 25)).color(TEXT_DIM).small());
                    }
                }

                if job.status == JobStatus::Failed {
                    if let Some(error) = &job.error_message {
                        ui.label(RichText::new(truncate(error, 30)).color(ACCENT_RED).small());
                    }
                }

                ui.add_space(8.0);

                if job.status == JobStatus::Done {
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("View Diff").color(TEXT_DIM).small(),
                                )
                                .small(),
                            )
                            .clicked()
                        {
                            action = Some(CardAction::ViewDiff);
                        }
                    });

                    if !is_selected {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Select").color(BG_PRIMARY).small(),
                                )
                                .fill(ACCENT_CYAN)
                                .small(),
                            )
                            .clicked()
                        {
                            action = Some(CardAction::Select);
                        }
                    } else {
                        ui.label(RichText::new("★ Selected").color(ACCENT_CYAN).small());
                    }
                }
            } else {
                ui.label(RichText::new("No data").color(TEXT_MUTED));
            }
        });

    action
}

/// Truncate a string to a maximum number of characters (UTF-8 safe)
fn truncate(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}
