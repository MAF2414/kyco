//! Job list panel rendering

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::{Job, JobStatus};

use super::colors::{BG, *};

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

/// Get sort priority for job status (lower = higher priority, shown first)
fn status_priority(status: &JobStatus) -> u8 {
    match status {
        JobStatus::Running => 0,  // Most important - currently executing
        JobStatus::Queued => 1,   // About to run
        JobStatus::Pending => 2,  // Waiting for user action
        JobStatus::Failed => 3,   // Needs attention
        JobStatus::Done => 4,     // Completed
        JobStatus::Rejected => 5, // Dismissed
        JobStatus::Merged => 6,   // Already merged
    }
}

/// Render the job list panel
pub fn render(frame: &mut Frame, area: Rect, jobs: &[&Job], selected: usize) {
    // Sort jobs: by status priority first, then by ID within each status group
    let mut sorted_jobs: Vec<&Job> = jobs.to_vec();
    sorted_jobs.sort_by(|a, b| {
        status_priority(&a.status)
            .cmp(&status_priority(&b.status))
            .then_with(|| a.id.cmp(&b.id))
    });

    // Get the ID of the selected job from the original order
    let selected_job_id = jobs.get(selected).map(|j| j.id);

    let queued_positions: std::collections::HashMap<u64, usize> = sorted_jobs
        .iter()
        .filter(|j| j.status == JobStatus::Queued)
        .enumerate()
        .map(|(i, j)| (j.id, i + 1))
        .collect();

    let items: Vec<ListItem> = sorted_jobs
        .iter()
        .map(|job| {
            let (status_color, status_icon) = match job.status {
                JobStatus::Pending => (YELLOW, "○"),
                JobStatus::Queued => (BLUE, "◐"),
                JobStatus::Running => (MAGENTA, get_spinner()),
                JobStatus::Done => (GREEN, "●"),
                JobStatus::Failed => (RED, "✗"),
                JobStatus::Rejected => (DARK_GRAY, "○"),
                JobStatus::Merged => (CYAN, "✓"),
            };

            let queue_suffix = if job.status == JobStatus::Queued {
                format!("{}", queued_positions.get(&job.id).unwrap_or(&0))
            } else {
                String::new()
            };

            let is_selected = selected_job_id == Some(job.id);

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
                .border_style(Style::default().fg(CYAN))
                .style(Style::default().bg(BG)),
        )
        .style(Style::default().bg(BG));

    frame.render_widget(list, area);
}
