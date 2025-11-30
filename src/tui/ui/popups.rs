//! Popup rendering (help popup, diff popup)

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::colors::*;

/// Create a centered rectangle
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

/// Render a help popup
// TODO: Add mouse wheel scroll support
pub fn render_help(frame: &mut Frame) {
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
            Span::styled("  S        ", Style::default().fg(BLUE)),
            Span::styled("Toggle AutoScan mode", Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  A        ", Style::default().fg(GREEN)),
            Span::styled("Toggle AutoRun mode", Style::default().fg(WHITE)),
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

    frame.render_widget(Clear, area);
    frame.render_widget(para, area);
}

/// Render a diff popup with syntax highlighting
pub fn render_diff(frame: &mut Frame, diff: &str, scroll: usize) {
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

    frame.render_widget(Clear, area);
    frame.render_widget(para, area);
}
