//! Job list UI rendering

mod render;
mod types;

pub use types::{JobListAction, JobListFilter};

use render::render_job_row;
use types::JobListAction as Action;

use super::super::theme::{
    ACCENT_CYAN, ACCENT_RED, BG_HIGHLIGHT, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::{Job, JobStatus};
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke};

/// Render the job list panel
pub fn render_job_list(
    ui: &mut egui::Ui,
    cached_jobs: &[Job],
    selected_job_id: &mut Option<u64>,
    filter: &mut JobListFilter,
) -> JobListAction {
    let mut action = JobListAction::None;

    if has_animated_jobs(cached_jobs) {
        ui.ctx().request_repaint();
    }

    let count_all = cached_jobs.len();
    let count_active = JobListFilter::Active.count(cached_jobs);
    let count_finished = JobListFilter::Finished.count(cached_jobs);
    let count_failed = JobListFilter::Failed.count(cached_jobs);

    ui.vertical(|ui| {
        render_header(ui, count_finished, &mut action);
        ui.add_space(4.0);
        render_filter_tabs(ui, filter, count_all, count_active, count_finished, count_failed);
        ui.add_space(4.0);
        ui.separator();

        render_job_scroll_area(
            ui,
            cached_jobs,
            selected_job_id,
            filter,
            &mut action,
        );
    });

    action
}

fn has_animated_jobs(jobs: &[Job]) -> bool {
    jobs.iter().any(|j| {
        matches!(
            j.status,
            JobStatus::Running | JobStatus::Blocked | JobStatus::Queued | JobStatus::Pending
        )
    })
}

fn render_header(ui: &mut egui::Ui, count_finished: usize, action: &mut JobListAction) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("JOBS").monospace().color(TEXT_PRIMARY));

        let remaining = ui.available_width();
        if count_finished > 0 {
            let btn_width = 60.0;
            if remaining > btn_width {
                ui.add_space(remaining - btn_width);
            }

            let clear_btn = egui::Button::new(RichText::new("Clear All").small().color(TEXT_DIM))
                .fill(BG_SECONDARY)
                .stroke(Stroke::new(1.0, TEXT_MUTED));

            if ui
                .add(clear_btn)
                .on_hover_text(format!("Delete all {} finished jobs", count_finished))
                .clicked()
            {
                *action = Action::DeleteAllFinished;
            }
        }
    });
}

fn render_filter_tabs(
    ui: &mut egui::Ui,
    filter: &mut JobListFilter,
    count_all: usize,
    count_active: usize,
    count_finished: usize,
    count_failed: usize,
) {
    ui.horizontal(|ui| {
        for (filter_option, count) in [
            (JobListFilter::All, count_all),
            (JobListFilter::Active, count_active),
            (JobListFilter::Finished, count_finished),
            (JobListFilter::Failed, count_failed),
        ] {
            let is_selected = *filter == filter_option;
            let label = filter_option.label();
            let label_with_count = if count > 0 {
                format!("{} ({})", label, count)
            } else {
                label.to_string()
            };

            let (text_color, bg_color) = if is_selected {
                (ACCENT_CYAN, BG_HIGHLIGHT)
            } else if count > 0 {
                (TEXT_DIM, BG_SECONDARY)
            } else {
                (TEXT_MUTED, Color32::TRANSPARENT)
            };

            let text_color =
                if filter_option == JobListFilter::Failed && count > 0 && !is_selected {
                    ACCENT_RED
                } else {
                    text_color
                };

            let btn = egui::Button::new(RichText::new(&label_with_count).small().color(text_color))
                .fill(bg_color)
                .corner_radius(4.0);

            if ui.add(btn).clicked() {
                *filter = filter_option;
            }

            ui.add_space(2.0);
        }
    });
}

fn render_job_scroll_area(
    ui: &mut egui::Ui,
    cached_jobs: &[Job],
    selected_job_id: &mut Option<u64>,
    filter: &JobListFilter,
    action: &mut JobListAction,
) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Use the scroll area's *actual* available width to avoid triggering an unintended
            // horizontal scrollbar (e.g. when a vertical scrollbar becomes visible).
            let available_width = ui.available_width();
            ui.set_min_width(available_width);

            let mut filtered_jobs: Vec<&Job> =
                cached_jobs.iter().filter(|j| filter.matches(j)).collect();

            filtered_jobs.sort_by(|a, b| {
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
                    .then_with(|| b.updated_at.cmp(&a.updated_at))
            });

            for job in filtered_jobs {
                let is_selected = *selected_job_id == Some(job.id);
                let response = render_job_row(ui, job, is_selected, available_width, action);

                // Only handle row click if no button action was triggered
                // (delete button sets action, which takes priority)
                if !matches!(action, JobListAction::DeleteJob(_))
                    && response.interact(egui::Sense::click()).clicked()
                {
                    *selected_job_id = Some(job.id);
                }
            }
        });
}
