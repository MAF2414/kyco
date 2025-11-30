//! UI rendering functions
//!
//! This module is split into submodules for better organization:
//! - `colors`: Color constants and shared styles
//! - `job_list`: Job list panel rendering
//! - `detail_panel`: Detail panel and activity log rendering
//! - `help_bar`: Help bar and syntax reference panel rendering
//! - `popups`: Popup rendering (help popup, diff popup)

mod colors;
mod detail_panel;
mod help_bar;
mod job_list;
mod popups;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::{config::Config, Job, LogEvent};

/// Render the main UI
pub fn render(
    frame: &mut Frame,
    jobs: &[&Job],
    selected_job: usize,
    logs: &[LogEvent],
    show_help: bool,
    config: &Config,
    show_diff: bool,
    diff_content: Option<&str>,
    diff_scroll: usize,
    auto_run: bool,
    auto_scan: bool,
) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(9)])
        .split(Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(main_chunks[0])[0]);

    let right_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(main_chunks[0])[1];

    let selected_job_data = jobs.get(selected_job).copied();

    job_list::render(frame, left_chunks[0], jobs, selected_job);
    help_bar::render_syntax_reference(frame, left_chunks[1]);
    detail_panel::render(frame, right_area, selected_job_data, logs, config);
    help_bar::render(frame, main_chunks[1], auto_run, auto_scan);

    if show_help {
        popups::render_help(frame);
    }

    if show_diff {
        if let Some(diff) = diff_content {
            popups::render_diff(frame, diff, diff_scroll);
        }
    }
}
