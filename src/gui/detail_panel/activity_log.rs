//! Activity log rendering for the detail panel

use eframe::egui::{self, RichText};

use crate::gui::theme::{BG_SECONDARY, TEXT_MUTED, TEXT_PRIMARY};
use crate::{Job, LogEvent};

use super::colors::log_color;
use super::markdown::render_markdown_inline_colored;
use super::types::ActivityLogFilters;

/// Get static string label for log event kind (avoids format! allocation per frame)
#[inline]
fn log_kind_label(kind: &crate::LogEventKind) -> &'static str {
    use crate::LogEventKind;
    match kind {
        LogEventKind::Thought => "[thought]",
        LogEventKind::ToolCall => "[tool]",
        LogEventKind::ToolOutput => "[output]",
        LogEventKind::Text => "[text]",
        LogEventKind::Error => "[error]",
        LogEventKind::System => "[system]",
        LogEventKind::Permission => "[permission]",
    }
}

/// Render activity log section inline (no inner scroll - parent handles scrolling)
pub(super) fn render_activity_log_inline(
    ui: &mut egui::Ui,
    job: &Job,
    logs: &[LogEvent],
    _scroll_to_bottom: bool,
    available_width: f32,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
    filters: &mut ActivityLogFilters,
) {
    let total_log_count = job.log_events.len()
        + logs
            .iter()
            .filter(|e| e.job_id.is_none() || e.job_id == Some(job.id))
            .count();

    let shown_log_count = job
        .log_events
        .iter()
        .filter(|e| filters.is_enabled(&e.kind))
        .count()
        + logs
            .iter()
            .filter(|e| {
                (e.job_id.is_none() || e.job_id == Some(job.id)) && filters.is_enabled(&e.kind)
            })
            .count();

    // Use stable id_salt based on job ID to prevent state reset when log count changes
    egui::CollapsingHeader::new(
        RichText::new(format!(
            "ACTIVITY LOG ({}/{})",
            shown_log_count, total_log_count
        ))
        .monospace()
        .color(TEXT_MUTED),
    )
    .id_salt(("activity_log", job.id))
    .default_open(false)
    .show(ui, |ui| {
        ui.set_min_width(available_width - 16.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Show:").small().color(TEXT_MUTED));
            egui::ComboBox::from_id_salt(("activity_log_filters", job.id))
                .selected_text(
                    RichText::new(filters.selected_summary())
                        .small()
                        .color(TEXT_PRIMARY),
                )
                .width(140.0)
                .show_ui(ui, |ui| {
                    ui.checkbox(&mut filters.show_text, "Text");
                    ui.checkbox(&mut filters.show_tool_call, "Tool calls");
                    ui.checkbox(&mut filters.show_tool_output, "Tool output");
                    ui.checkbox(&mut filters.show_thought, "Thought");
                    ui.checkbox(&mut filters.show_system, "System");
                    ui.checkbox(&mut filters.show_error, "Error");
                    ui.checkbox(&mut filters.show_permission, "Permission");

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Only text").clicked() {
                            *filters = ActivityLogFilters::default();
                        }
                        if ui.button("All").clicked() {
                            filters.show_text = true;
                            filters.show_tool_call = true;
                            filters.show_tool_output = true;
                            filters.show_thought = true;
                            filters.show_system = true;
                            filters.show_error = true;
                            filters.show_permission = true;
                        }
                    });
                });
        });

        ui.add_space(6.0);

        // Show job-specific logs first
        for event in &job.log_events {
            if !filters.is_enabled(&event.kind) {
                continue;
            }

            let color = log_color(&event.kind);
            render_activity_log_event(ui, event, commonmark_cache, color);
        }

        // Then show global logs filtered by job_id
        for event in logs {
            if (event.job_id.is_none() || event.job_id == Some(job.id))
                && filters.is_enabled(&event.kind)
            {
                let color = log_color(&event.kind);
                render_activity_log_event(ui, event, commonmark_cache, color);
            }
        }
    });
}

fn render_activity_log_event(
    ui: &mut egui::Ui,
    event: &LogEvent,
    commonmark_cache: &mut egui_commonmark::CommonMarkCache,
    color: egui::Color32,
) {
    let text = event.content.as_deref().unwrap_or(event.summary.as_str());
    let render_markdown = matches!(event.kind, crate::LogEventKind::Text);

    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(6.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(log_kind_label(&event.kind))
                        .monospace()
                        .color(TEXT_MUTED),
                );
                if let Some(tool) = event.tool_name.as_deref() {
                    ui.label(RichText::new(tool).monospace().small().color(TEXT_MUTED));
                }
            });

            ui.add_space(2.0);

            if render_markdown {
                render_markdown_inline_colored(ui, text, commonmark_cache, color);
            } else {
                ui.label(RichText::new(text).color(color));
            }
        });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);
}
