//! Main detail panel rendering

use eframe::egui::{self, RichText, ScrollArea};

use crate::gui::theme::{BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

use super::colors::status_color;
use super::prompt::build_prompt_preview;
use crate::gui::diff::render_diff_content;

use super::actions::render_action_buttons;
use super::activity_log::render_activity_log_inline;
use super::chain::render_chain_progress_section_with_height;
use super::result::render_result_section;
use super::types::{DetailPanelAction, DetailPanelState};

use crate::gui::theme::{ACCENT_GREEN, ACCENT_RED};
use crate::Job;

/// Render the detail panel and return any action triggered by the user
pub fn render_detail_panel(
    ui: &mut egui::Ui,
    state: &mut DetailPanelState<'_>,
) -> Option<DetailPanelAction> {
    let mut action: Option<DetailPanelAction> = None;

    let available_width = ui.available_width();
    let available_height = ui.available_height();

    render_header(ui);

    if let Some(job_id) = state.selected_job_id {
        if let Some(job) = state.cached_jobs.iter().find(|j| j.id == job_id) {
            // Wrap everything in a ScrollArea to ensure we never overflow
            ScrollArea::vertical()
                .id_salt(("detail_panel_scroll", job.id))
                .auto_shrink([false, false])
                .max_height(available_height - 30.0) // Reserve space for header
                .show(ui, |ui| {
                    ui.set_min_width(available_width - 16.0);

                    render_job_info(ui, job);
                    render_result_section(ui, job, state.commonmark_cache);

                    ui.add_space(8.0);

                    action = render_action_buttons(
                        ui,
                        job,
                        state.continuation_prompt,
                        state.config,
                        state.permission_mode_overrides,
                    );

                    ui.add_space(8.0);
                    ui.separator();

                    if job.chain_name.is_some() || !job.chain_step_history.is_empty() {
                        render_chain_progress_section_with_height(
                            ui,
                            job,
                            state.commonmark_cache,
                            available_width,
                        );
                        ui.add_space(4.0);
                    }

                    render_prompt_section_collapsible(ui, job, state.config);

                    ui.add_space(4.0);

                    if let Some(diff_content) = state.diff_content {
                        render_diff_section_inline(ui, diff_content, available_width);
                    } else {
                        ui.add_space(8.0);
                        egui::Frame::NONE
                            .fill(BG_SECONDARY)
                            .corner_radius(4.0)
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                ui.set_min_width(available_width - 48.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(RichText::new("No diff available").color(TEXT_MUTED));
                                    ui.label(
                                        RichText::new(
                                            "Diff will appear here when the job completes with changes",
                                        )
                                        .color(TEXT_DIM)
                                        .small(),
                                    );
                                });
                            });
                    }

                    ui.add_space(4.0);

                    render_activity_log_inline(
                        ui,
                        job,
                        state.logs,
                        state.log_scroll_to_bottom,
                        available_width,
                        state.commonmark_cache,
                        state.activity_log_filters,
                    );
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

fn render_header(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("DETAILS").monospace().color(TEXT_PRIMARY));
    });
    ui.add_space(4.0);
    ui.separator();
}

fn render_job_info(ui: &mut egui::Ui, job: &Job) {
    use crate::gui::theme::ACCENT_CYAN;

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

/// Render prompt section with collapsible header (collapsed by default)
fn render_prompt_section_collapsible(
    ui: &mut egui::Ui,
    job: &Job,
    config: &crate::config::Config,
) {
    use std::borrow::Cow;

    let (prompt_text, prompt_label): (Cow<'_, str>, &str) = match &job.sent_prompt {
        Some(prompt) => (Cow::Borrowed(prompt.as_str()), "SENT PROMPT"),
        None => (
            Cow::Owned(build_prompt_preview(job, config)),
            "PROMPT PREVIEW",
        ),
    };

    let line_count = prompt_text.lines().count();

    egui::CollapsingHeader::new(
        RichText::new(format!("{} ({} lines)", prompt_label, line_count))
            .monospace()
            .color(TEXT_MUTED),
    )
    .default_open(false)
    .show(ui, |ui| {
        ScrollArea::vertical()
            .id_salt("prompt_scroll")
            .auto_shrink([false, false])
            .max_height(200.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut prompt_text.as_ref())
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_DIM)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            });
    });
}

/// Render diff section inline (no inner scroll - parent handles scrolling)
fn render_diff_section_inline(ui: &mut egui::Ui, diff_content: &str, available_width: f32) {
    let added = diff_content
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .count();
    let removed = diff_content
        .lines()
        .filter(|l| l.starts_with('-') && !l.starts_with("---"))
        .count();

    egui::CollapsingHeader::new(RichText::new("DIFF").monospace().color(TEXT_PRIMARY))
        .id_salt("diff_section")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if added > 0 {
                    ui.label(
                        RichText::new(format!("+{}", added))
                            .color(ACCENT_GREEN)
                            .small(),
                    );
                }
                if removed > 0 {
                    ui.label(
                        RichText::new(format!("-{}", removed))
                            .color(ACCENT_RED)
                            .small(),
                    );
                }
            });
            ui.add_space(4.0);

            egui::Frame::NONE
                .fill(BG_SECONDARY)
                .corner_radius(4.0)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    ui.set_min_width(available_width - 24.0);
                    render_diff_content(ui, diff_content);
                });
        });
}
