//! Help bar and syntax reference panel rendering

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::colors::*;

/// Render the help bar at the bottom
pub fn render(frame: &mut Frame, area: Rect) {
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
        // TODO: Add autostart/autoscan mode indicators (see config.toml)
    ]);

    frame.render_widget(Paragraph::new(help), area);
}

/// Render the syntax reference panel
pub fn render_syntax_reference(frame: &mut Frame, area: Rect) {
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
