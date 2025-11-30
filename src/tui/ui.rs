//! UI rendering functions

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::{config::Config, Job, JobStatus, LogEvent};

// VIVID color constants - direct usage for clarity
const CYAN: Color = Color::Cyan;
const GREEN: Color = Color::LightGreen;
const YELLOW: Color = Color::Yellow;
const RED: Color = Color::LightRed;
const MAGENTA: Color = Color::Magenta;
const BLUE: Color = Color::LightBlue;
const WHITE: Color = Color::White;
const GRAY: Color = Color::Gray;
const DARK_GRAY: Color = Color::DarkGray;

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

    render_job_list(frame, left_chunks[0], jobs, selected_job);
    render_syntax_reference(frame, left_chunks[1]);
    render_detail_panel(frame, right_area, selected_job_data, logs, config);
    render_help_bar(frame, main_chunks[1]);

    if show_help {
        render_help_popup(frame);
    }

    if show_diff {
        if let Some(diff) = diff_content {
            render_diff_popup(frame, diff, diff_scroll);
        }
    }
}

/// Render the syntax reference panel
fn render_syntax_reference(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(vec![
            Span::styled("@@", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("{agent:}", Style::default().fg(DARK_GRAY)),
            Span::styled("mode", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
            Span::styled(" desc", Style::default().fg(GRAY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  @@docs ", Style::default().fg(CYAN)),
            Span::styled("add docstrings", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  @@c:fix ", Style::default().fg(CYAN)),
            Span::styled("handle edge case", Style::default().fg(GRAY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Agents ", Style::default().fg(YELLOW)),
            Span::styled("c/claude x/codex g/gemini", Style::default().fg(WHITE)),
        ]),
    ];

    let para = Paragraph::new(text)
        .block(
            Block::default()
                .title(Span::styled(" Syntax ", Style::default().fg(YELLOW)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DARK_GRAY)),
        );

    frame.render_widget(para, area);
}

/// Render the help bar at the bottom
fn render_help_bar(frame: &mut Frame, area: Rect) {
    let help = Line::from(vec![
        Span::styled(" ↑↓ ", Style::default().fg(Color::Black).bg(WHITE)),
        Span::styled(" Nav ", Style::default().fg(GRAY)),
        Span::styled(" ⏎ ", Style::default().fg(Color::Black).bg(GREEN)),
        Span::styled(" Run ", Style::default().fg(GRAY)),
        Span::styled(" d ", Style::default().fg(Color::Black).bg(GREEN)),
        Span::styled(" Diff ", Style::default().fg(GRAY)),
        Span::styled(" m ", Style::default().fg(Color::Black).bg(CYAN)),
        Span::styled(" Merge ", Style::default().fg(GRAY)),
        Span::styled(" r ", Style::default().fg(Color::Black).bg(RED)),
        Span::styled(" Reject ", Style::default().fg(GRAY)),
        Span::styled(" s ", Style::default().fg(Color::Black).bg(BLUE)),
        Span::styled(" Scan ", Style::default().fg(GRAY)),
        Span::styled(" ? ", Style::default().fg(Color::Black).bg(MAGENTA)),
        Span::styled(" Help ", Style::default().fg(GRAY)),
        Span::styled(" q ", Style::default().fg(Color::Black).bg(GRAY)),
        Span::styled(" Quit ", Style::default().fg(GRAY)),
    ]);

    frame.render_widget(Paragraph::new(help), area);
}

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
fn render_job_list(frame: &mut Frame, area: Rect, jobs: &[&Job], selected: usize) {
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
fn render_detail_panel(
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
                Span::styled("Target   ", Style::default().fg(DARK_GRAY)),
                Span::styled(&job.target, Style::default().fg(GRAY)),
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

    frame.render_widget(log_para, chunks[2]);
}

/// Render a help popup
fn render_help_popup(frame: &mut Frame) {
    let area = centered_rect(55, 55, frame.area());

    let help_text = vec![
        Line::from(Span::styled("Keyboard Shortcuts", Style::default().fg(CYAN).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑/↓ j/k  ", Style::default().fg(YELLOW)),
            Span::styled("Navigate jobs", Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  Enter    ", Style::default().fg(GREEN)),
            Span::styled("Start/queue selected job", Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  d        ", Style::default().fg(GREEN)),
            Span::styled("View diff of job's worktree", Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  m        ", Style::default().fg(CYAN)),
            Span::styled("Merge worktree changes to main", Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  a        ", Style::default().fg(CYAN)),
            Span::styled("Apply changes (legacy)", Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  r        ", Style::default().fg(RED)),
            Span::styled("Reject job & cleanup worktree", Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  f        ", Style::default().fg(MAGENTA)),
            Span::styled("Focus terminal (REPL jobs)", Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  s        ", Style::default().fg(BLUE)),
            Span::styled("Scan for new tasks", Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  ?        ", Style::default().fg(MAGENTA)),
            Span::styled("Toggle this help", Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  q/Esc    ", Style::default().fg(GRAY)),
            Span::styled("Quit", Style::default().fg(WHITE)),
        ]),
    ];

    let para = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(Span::styled(" Help ", Style::default().fg(CYAN)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(CYAN)),
        );

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(para, area);
}

/// Create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Render a diff popup with syntax highlighting
fn render_diff_popup(frame: &mut Frame, diff: &str, scroll: usize) {
    let area = centered_rect(85, 85, frame.area());

    let lines: Vec<Line> = diff
        .lines()
        .skip(scroll)
        .take(area.height.saturating_sub(2) as usize)
        .map(|line| {
            let (color, prefix) = if line.starts_with('+') && !line.starts_with("+++") {
                (GREEN, "")
            } else if line.starts_with('-') && !line.starts_with("---") {
                (RED, "")
            } else if line.starts_with("@@") {
                (CYAN, "")
            } else if line.starts_with("diff ") || line.starts_with("index ") {
                (YELLOW, "")
            } else if line.starts_with("---") || line.starts_with("+++") {
                (BLUE, "")
            } else {
                (GRAY, "")
            };
            Line::from(Span::styled(format!("{}{}", prefix, line), Style::default().fg(color)))
        })
        .collect();

    let total_lines = diff.lines().count();
    let title = format!(
        " Diff (↑↓/jk scroll, PgUp/PgDn, d/Esc close) [{}/{}] ",
        scroll + 1,
        total_lines
    );

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(title, Style::default().fg(GREEN)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(GREEN)),
        );

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(para, area);
}
