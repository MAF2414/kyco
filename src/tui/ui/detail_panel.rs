//! Detail panel and activity log rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{config::Config, Job, JobStatus, LogEvent};

use super::colors::{BG, *};

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
                .replace("{scope_type}", "file");
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
            JobStatus::Merged => (CYAN, "merged"),
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
                Span::styled("file", Style::default().fg(WHITE)),
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
                    .border_style(Style::default().fg(CYAN))
                    .style(Style::default().bg(BG)),
            )
            .style(Style::default().bg(BG));

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
                    .border_style(Style::default().fg(title_color))
                    .style(Style::default().bg(BG)),
            )
            .style(Style::default().bg(BG))
            .wrap(Wrap { trim: false });

        frame.render_widget(prompt_para, chunks[1]);
    } else {
        let para = Paragraph::new(Span::styled("No job selected", Style::default().fg(DARK_GRAY)))
            .block(
                Block::default()
                    .title(Span::styled(" Details ", Style::default().fg(DARK_GRAY)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DARK_GRAY))
                    .style(Style::default().bg(BG)),
            )
            .style(Style::default().bg(BG));
        frame.render_widget(para, chunks[0]);

        let prompt_para = Paragraph::new("")
            .block(
                Block::default()
                    .title(Span::styled(" Prompt ", Style::default().fg(DARK_GRAY)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DARK_GRAY))
                    .style(Style::default().bg(BG)),
            )
            .style(Style::default().bg(BG));
        frame.render_widget(prompt_para, chunks[1]);
    }

    render_activity_log(frame, chunks[2], job, logs);
}

/// Wrap text to fit within a given width, returning wrapped lines
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_width = word.chars().count();

        if current_width == 0 {
            // First word on line
            current_line = word.to_string();
            current_width = word_width;
        } else if current_width + 1 + word_width <= max_width {
            // Word fits with space
            current_line.push(' ');
            current_line.push_str(word);
            current_width += 1 + word_width;
        } else {
            // Word doesn't fit, start new line
            result.push(current_line);
            current_line = word.to_string();
            current_width = word_width;
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    if result.is_empty() {
        result.push(String::new());
    }

    result
}

/// Render the activity log section
fn render_activity_log(frame: &mut Frame, area: Rect, job: Option<&Job>, logs: &[LogEvent]) {
    // Calculate available width for text content
    // Area width minus borders (2) minus timestamp "HH:MM:SS " (9) minus icon "X " (2)
    let prefix_width = 11; // "HH:MM:SS " + "X "
    let border_width = 2;
    let text_width = area.width.saturating_sub(border_width + prefix_width) as usize;

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

            // Split summary into multiple lines if it contains newlines, then wrap each line
            let summary_lines: Vec<&str> = event.summary.lines().collect();

            if summary_lines.is_empty() {
                // Empty summary, just show timestamp and icon
                vec![Line::from(vec![
                    Span::styled(format!("{} ", time), Style::default().fg(DARK_GRAY)),
                    Span::styled(format!("{} ", kind_icon), Style::default().fg(kind_color)),
                ])]
            } else {
                let mut lines = Vec::new();
                let mut is_first = true;

                for summary_line in &summary_lines {
                    // Wrap this line of text
                    let wrapped = wrap_text(summary_line, text_width);

                    for (i, wrapped_line) in wrapped.iter().enumerate() {
                        if is_first && i == 0 {
                            // First line of first paragraph gets timestamp and icon
                            lines.push(Line::from(vec![
                                Span::styled(format!("{} ", time), Style::default().fg(DARK_GRAY)),
                                Span::styled(format!("{} ", kind_icon), Style::default().fg(kind_color)),
                                Span::styled(wrapped_line.clone(), Style::default().fg(WHITE)),
                            ]));
                        } else {
                            // Continuation lines get indentation to align with first line's text
                            lines.push(Line::from(vec![
                                Span::styled("         ", Style::default().fg(DARK_GRAY)), // 9 spaces
                                Span::styled("  ", Style::default().fg(kind_color)), // 2 spaces for icon
                                Span::styled(wrapped_line.clone(), Style::default().fg(WHITE)),
                            ]));
                        }
                    }
                    is_first = false;
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
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(BG)),
        )
        .style(Style::default().bg(BG))
        .wrap(Wrap { trim: false });

    frame.render_widget(log_para, area);
}
