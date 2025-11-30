//! Detail panel and activity log rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{config::Config, Job, JobStatus, LogEvent};

use super::colors::*;

/// Build the prompt string for a job using the mode template from config
fn build_prompt_preview(job: &Job, config: &Config) -> String {
    // Get mode template from config
    if let Some(mode_config) = config.get_mode(&job.mode) {
        if let Some(template) = &mode_config.prompt {
            let file_path = job.source_file.display().to_string();
            let description = job.description.as_deref().unwrap_or("");

            return template
                .replace("{file}", &file_path)
                .replace("{line}", &job.source_line.to_string())
                .replace("{target}", &job.target)
                .replace("{mode}", &job.mode)
                .replace("{description}", description)
                .replace("{scope_type}", &job.scope.scope.to_string());
        }
    }

    // Fallback to default format
    let file_path = job.source_file.display().to_string();
    let line = job.source_line;
    let file_ref = format!("{}:{}", file_path, line);
    let description = job.description.as_deref().unwrap_or("");

    if description.is_empty() {
        format!("In `{}`, execute '{}' task on the code at that location.", file_ref, job.mode)
    } else {
        format!("In `{}`: {}", file_ref, description)
    }
}

/// Render the detail panel
pub fn render(
    frame: &mut Frame,
    area: Rect,
    job: Option<&Job>,
    logs: &[LogEvent],
    config: &Config,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(5),
            Constraint::Min(0),
        ])
        .split(area);

    if let Some(job) = job {
        let (status_color, status_text) = match job.status {
            JobStatus::Pending => (YELLOW, "pending"),
            JobStatus::Queued => (BLUE, "queued"),
            JobStatus::Running => (MAGENTA, "running"),
            JobStatus::Done => (GREEN, "done"),
            JobStatus::Failed => (RED, "failed"),
            JobStatus::Rejected => (DARK_GRAY, "rejected"),
        };

        let details = vec![
            Line::from(vec![
                Span::styled("ID       ", Style::default().fg(DARK_GRAY)),
                Span::styled(format!("#{}", job.id), Style::default().fg(WHITE)),
                Span::styled("    Status  ", Style::default().fg(DARK_GRAY)),
                Span::styled(status_text, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Mode     ", Style::default().fg(DARK_GRAY)),
                Span::styled(&job.mode, Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)),
                Span::styled("    Agent   ", Style::default().fg(DARK_GRAY)),
                Span::styled(&job.agent_id, Style::default().fg(CYAN)),
            ]),
            Line::from(vec![
                Span::styled("Scope    ", Style::default().fg(DARK_GRAY)),
                Span::styled(job.scope.scope.to_string(), Style::default().fg(WHITE)),
            ]),
            Line::from(vec![
                Span::styled("Worktree ", Style::default().fg(DARK_GRAY)),
                Span::styled(
                    job.git_worktree_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "—".to_string()),
                    Style::default().fg(GRAY),
                ),
                Span::styled("    Changes ", Style::default().fg(DARK_GRAY)),
                Span::styled(
                    format!("{}", job.changed_files.len()),
                    Style::default().fg(if job.changed_files.is_empty() { GRAY } else { GREEN }),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Source   ", Style::default().fg(DARK_GRAY)),
                Span::styled(
                    format!("{}:{}", job.source_file.display(), job.source_line),
                    Style::default().fg(BLUE),
                ),
            ]),
        ];

        let para = Paragraph::new(details)
            .block(
                Block::default()
                    .title(Span::styled(" Details ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(CYAN)),
            );

        frame.render_widget(para, chunks[0]);

        // Prompt section
        let (prompt_text, prompt_title, title_color) = if let Some(ref sent) = job.sent_prompt {
            (sent.clone(), " Prompt (sent) ", GREEN)
        } else {
            (build_prompt_preview(job, config), " Prompt (preview) ", YELLOW)
        };

        let prompt_para = Paragraph::new(Span::styled(&prompt_text, Style::default().fg(WHITE)))
            .block(
                Block::default()
                    .title(Span::styled(prompt_title, Style::default().fg(title_color)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(title_color)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(prompt_para, chunks[1]);
    } else {
        let para = Paragraph::new(Span::styled("No job selected", Style::default().fg(DARK_GRAY)))
            .block(
                Block::default()
                    .title(Span::styled(" Details ", Style::default().fg(DARK_GRAY)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DARK_GRAY)),
            );
        frame.render_widget(para, chunks[0]);

        let prompt_para = Paragraph::new("")
            .block(
                Block::default()
                    .title(Span::styled(" Prompt ", Style::default().fg(DARK_GRAY)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DARK_GRAY)),
            );
        frame.render_widget(prompt_para, chunks[1]);
    }

    render_activity_log(frame, chunks[2], job, logs);
}

/// Render the activity log section
fn render_activity_log(frame: &mut Frame, area: Rect, job: Option<&Job>, logs: &[LogEvent]) {
    let selected_job_id = job.map(|j| j.id);
    let log_lines: Vec<Line> = logs
        .iter()
        .rev()
        .filter(|event| {
            match (selected_job_id, event.job_id) {
                (Some(selected), Some(log_job)) => selected == log_job,
                (_, None) => true, // Always show system-wide logs
                (None, _) => true,
            }
        })
        .take(50)
        .flat_map(|event| {
            let (kind_color, kind_icon) = match event.kind {
                crate::LogEventKind::Thought => (MAGENTA, "◆"),
                crate::LogEventKind::ToolCall => (YELLOW, "▶"),
                crate::LogEventKind::ToolOutput => (GRAY, "◁"),
                crate::LogEventKind::Text => (WHITE, "│"),
                crate::LogEventKind::Error => (RED, "✗"),
                crate::LogEventKind::System => (CYAN, "○"),
            };

            let time = event.timestamp.format("%H:%M:%S");

            // Split summary into multiple lines if it contains newlines
            let summary_lines: Vec<&str> = event.summary.lines().collect();

            if summary_lines.is_empty() {
                // Empty summary, just show timestamp and icon
                vec![Line::from(vec![
                    Span::styled(format!("{} ", time), Style::default().fg(DARK_GRAY)),
                    Span::styled(format!("{} ", kind_icon), Style::default().fg(kind_color)),
                ])]
            } else {
                // First line gets timestamp and icon
                let mut lines = vec![Line::from(vec![
                    Span::styled(format!("{} ", time), Style::default().fg(DARK_GRAY)),
                    Span::styled(format!("{} ", kind_icon), Style::default().fg(kind_color)),
                    Span::styled(summary_lines[0].to_string(), Style::default().fg(WHITE)),
                ])];

                // Continuation lines get indentation to align with first line's text
                for continuation in summary_lines.iter().skip(1) {
                    lines.push(Line::from(vec![
                        Span::styled("         ", Style::default().fg(DARK_GRAY)), // 9 spaces to align
                        Span::styled("  ", Style::default().fg(kind_color)), // 2 spaces for icon alignment
                        Span::styled(continuation.to_string(), Style::default().fg(WHITE)),
                    ]));
                }

                lines
            }
        })
        .collect();

    let log_title = if let Some(job) = job {
        format!(" Activity #{} ", job.id)
    } else {
        " Activity ".to_string()
    };

    let border_color = if job.map(|j| j.status == JobStatus::Running).unwrap_or(false) {
        MAGENTA
    } else {
        DARK_GRAY
    };

    let log_para = Paragraph::new(log_lines)
        .block(
            Block::default()
                .title(Span::styled(log_title, Style::default().fg(WHITE)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(log_para, area);
}
