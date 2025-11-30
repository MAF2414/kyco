//! Job list panel rendering

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::{Job, JobStatus};

use super::colors::*;

/// Get spinner frame based on time
fn get_spinner() -> &'static str {
    const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let idx = (ms / 100) as usize % SPINNER_FRAMES.len();
    SPINNER_FRAMES[idx]
}

/// Render the job list panel
pub fn render(frame: &mut Frame, area: Rect, jobs: &[&Job], selected: usize) {
    let queued_positions: std::collections::HashMap<u64, usize> = jobs
        .iter()
        .filter(|j| j.status == JobStatus::Queued)
        .enumerate()
        .map(|(i, j)| (j.id, i + 1))
        .collect();

    let items: Vec<ListItem> = jobs
        .iter()
        .enumerate()
        .map(|(i, job)| {
            let (status_color, status_icon) = match job.status {
                JobStatus::Pending => (YELLOW, "○"),
                JobStatus::Queued => (BLUE, "◐"),
                JobStatus::Running => (MAGENTA, get_spinner()),
                JobStatus::Done => (GREEN, "●"),
                JobStatus::Failed => (RED, "✗"),
                JobStatus::Rejected => (DARK_GRAY, "○"),
            };

            let queue_suffix = if job.status == JobStatus::Queued {
                format!("{}", queued_positions.get(&job.id).unwrap_or(&0))
            } else {
                String::new()
            };

            let is_selected = i == selected;

            let content = Line::from(vec![
                Span::styled(format!("{}{} ", status_icon, queue_suffix), Style::default().fg(status_color)),
                Span::styled(format!("#{} ", job.id), Style::default().fg(DARK_GRAY)),
                Span::styled(&job.mode, Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(" ", Style::default()),
                Span::styled(&job.target, Style::default().fg(GRAY)),
            ]);

            let style = if is_selected {
                Style::default().bg(Color::Rgb(80, 80, 100))
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(" Jobs ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(CYAN)),
        );

    frame.render_widget(list, area);
}
