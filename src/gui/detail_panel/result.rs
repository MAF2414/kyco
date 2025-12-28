//! Result section rendering for the detail panel

use eframe::egui::{self, RichText};

use crate::gui::theme::{
    ACCENT_GREEN, ACCENT_RED, BG_SECONDARY, STATUS_RUNNING, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::Job;

use super::markdown::render_markdown_scroll;

/// Render result section (from YAML block or raw text)
pub(super) fn render_result_section(
    ui: &mut egui::Ui,
    job: &Job,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
) {
    let response_text = job
        .full_response
        .as_deref()
        .or_else(|| job.result.as_ref().and_then(|r| r.raw_text.as_deref()));

    if let Some(result) = &job.result {
        ui.add_space(8.0);
        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                let has_structured =
                    result.title.is_some() || result.status.is_some() || result.details.is_some();

                if has_structured {
                    if let Some(title) = &result.title {
                        ui.label(RichText::new(title).monospace().color(TEXT_PRIMARY));
                    }

                    if let Some(details) = &result.details {
                        ui.add_space(4.0);
                        ui.label(RichText::new(details).color(TEXT_DIM));
                    }

                    if let Some(summary) = &result.summary {
                        if !summary.is_empty() {
                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(4.0);
                            ui.label(RichText::new("Summary:").small().color(TEXT_MUTED));
                            ui.label(RichText::new(summary).color(TEXT_DIM).small());
                        }
                    }

                    ui.add_space(8.0);
                    render_stats_bar(ui, job, result);
                }

                if let Some(text) = response_text {
                    if has_structured {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }

                    ui.label(RichText::new("Response:").small().color(TEXT_MUTED));
                    ui.add_space(4.0);

                    render_markdown_scroll(ui, text, commonmark_cache);

                    // Still show stats if we didn't render the structured stats bar
                    if !has_structured {
                        if let Some(stats) = &job.stats {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if stats.files_changed > 0 {
                                    ui.label(
                                        RichText::new(format!("{} files", stats.files_changed))
                                            .color(TEXT_MUTED),
                                    );
                                    ui.add_space(8.0);
                                }
                                if let Some(duration) = job.duration_string() {
                                    ui.label(RichText::new(duration).color(TEXT_MUTED));
                                }
                            });
                        }
                    }
                }
            });
    } else if let Some(text) = response_text {
        // No parsed result, but we have a full response to display.
        ui.add_space(8.0);
        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.label(RichText::new("Response:").small().color(TEXT_MUTED));
                ui.add_space(4.0);

                render_markdown_scroll(ui, text, commonmark_cache);
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

fn render_stats_bar(ui: &mut egui::Ui, job: &Job, result: &crate::JobResult) {
    ui.horizontal(|ui| {
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

        if let Some(duration) = job.duration_string() {
            ui.label(RichText::new(duration).color(TEXT_MUTED));
        }
    });
}
