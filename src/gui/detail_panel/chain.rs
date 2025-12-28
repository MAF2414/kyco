//! Chain progress section rendering for the detail panel

use eframe::egui::{self, RichText};

use crate::gui::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_SECONDARY, STATUS_RUNNING, TEXT_DIM, TEXT_MUTED,
    TEXT_PRIMARY,
};
use crate::{ChainStepSummary, Job, JobStatus};

use super::markdown::apply_markdown_theme;

/// Render chain progress section inline (no inner scroll - parent handles scrolling)
pub(super) fn render_chain_progress_section_with_height(
    ui: &mut egui::Ui,
    job: &Job,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
    available_width: f32,
) {
    let chain_name = job.chain_name.as_deref().unwrap_or("Chain");
    let total_steps = job
        .chain_total_steps
        .unwrap_or(job.chain_step_history.len());
    let current_step = job.chain_current_step.unwrap_or(0);
    let completed_steps = job
        .chain_step_history
        .iter()
        .filter(|s| !s.skipped && s.success)
        .count();
    let is_running = job.status == JobStatus::Running;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("⛓ {}", chain_name))
                .monospace()
                .color(ACCENT_CYAN),
        );
        ui.add_space(8.0);

        if is_running && total_steps > 0 {
            ui.label(
                RichText::new(format!("Step {}/{}", current_step + 1, total_steps))
                    .color(STATUS_RUNNING)
                    .small(),
            );
        } else if !job.chain_step_history.is_empty() {
            let status_color = if job.status == JobStatus::Done {
                ACCENT_GREEN
            } else if job.status == JobStatus::Failed {
                ACCENT_RED
            } else {
                TEXT_MUTED
            };
            ui.label(
                RichText::new(format!("{}/{} completed", completed_steps, total_steps))
                    .color(status_color)
                    .small(),
            );
        }
    });

    ui.add_space(4.0);

    if total_steps > 0 {
        let progress = if is_running {
            current_step as f32 / total_steps as f32
        } else {
            completed_steps as f32 / total_steps as f32
        };

        let progress_bar = egui::ProgressBar::new(progress)
            .fill(if is_running {
                STATUS_RUNNING
            } else if job.status == JobStatus::Done {
                ACCENT_GREEN
            } else {
                ACCENT_RED
            })
            .animate(is_running);
        ui.add_sized([available_width - 16.0, 8.0], progress_bar);
    }

    ui.add_space(8.0);

    if !job.chain_step_history.is_empty() {
        egui::CollapsingHeader::new(
            RichText::new(format!("CHAIN STEPS ({})", job.chain_step_history.len()))
                .monospace()
                .color(TEXT_MUTED),
        )
        .id_salt(("chain_steps", job.id))
        .default_open(true)
        .show(ui, |ui| {
            ui.set_min_width(available_width - 16.0);
            for step in &job.chain_step_history {
                render_chain_step_full_width(
                    ui,
                    step,
                    job.id,
                    commonmark_cache,
                    available_width - 24.0,
                );
                ui.add_space(4.0);
            }
        });
    }
}

/// Render a single chain step with explicit width (no inner scroll)
fn render_chain_step_full_width(
    ui: &mut egui::Ui,
    step: &ChainStepSummary,
    job_id: u64,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
    width: f32,
) {
    let (status_icon, status_color) = if step.skipped {
        ("○", TEXT_MUTED)
    } else if step.success {
        ("✓", ACCENT_GREEN)
    } else {
        ("✗", ACCENT_RED)
    };

    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_min_width(width);

            ui.horizontal(|ui| {
                ui.label(RichText::new(status_icon).color(status_color));
                ui.label(
                    RichText::new(format!("{}. {}", step.step_index + 1, step.mode))
                        .monospace()
                        .color(TEXT_PRIMARY),
                );
                if step.skipped {
                    ui.label(RichText::new("(skipped)").color(TEXT_MUTED).small());
                } else if step.files_changed > 0 {
                    ui.label(
                        RichText::new(format!("{} files", step.files_changed))
                            .color(TEXT_MUTED)
                            .small(),
                    );
                }
            });

            if let Some(title) = &step.title {
                ui.label(RichText::new(title).color(TEXT_DIM));
            }

            if let Some(error) = &step.error {
                ui.label(
                    RichText::new(format!("Error: {}", error))
                        .color(ACCENT_RED)
                        .small(),
                );
            }

            if let Some(summary) = &step.summary {
                ui.add_space(4.0);
                ui.label(RichText::new("Summary:").small().color(TEXT_MUTED));
                // Truncate long summaries safely at character boundary
                let truncated = if summary.chars().count() > 200 {
                    let mut end = summary
                        .char_indices()
                        .nth(200)
                        .map(|(i, _)| i)
                        .unwrap_or(summary.len());
                    // Ensure we don't split in the middle of a grapheme cluster
                    while !summary.is_char_boundary(end) && end > 0 {
                        end -= 1;
                    }
                    format!("{}...", &summary[..end])
                } else {
                    summary.clone()
                };
                ui.label(RichText::new(truncated).color(TEXT_DIM).small());
            }

            if let Some(response) = &step.full_response {
                ui.add_space(4.0);
                egui::CollapsingHeader::new(
                    RichText::new("Full Response").small().color(TEXT_MUTED),
                )
                .id_salt(("step_response_fw", job_id, step.step_index))
                .default_open(false)
                .show(ui, |ui| {
                    ui.set_min_width(width - 16.0);
                    ui.scope(|ui| {
                        apply_markdown_theme(ui);
                        egui_commonmark::CommonMarkViewer::new().show(
                            ui,
                            commonmark_cache,
                            response,
                        );
                    });
                });
            }
        });
}
