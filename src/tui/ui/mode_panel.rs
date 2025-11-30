//! Mode panel for displaying and editing config.toml modes

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::config::{Config, ModeConfig};

use super::colors::{BG, *};
use super::popups::centered_rect;

/// State for the mode panel
#[derive(Debug, Default)]
pub struct ModePanelState {
    /// Index of the currently selected mode
    pub selected_mode: usize,
    /// List of mode names (sorted)
    pub mode_names: Vec<String>,
    /// Whether the panel is visible
    pub visible: bool,
}

impl ModePanelState {
    /// Create a new mode panel state from config
    pub fn new(config: &Config) -> Self {
        let mut mode_names: Vec<String> = config.mode.keys().cloned().collect();
        mode_names.sort();

        Self {
            selected_mode: 0,
            mode_names,
            visible: false,
        }
    }

    /// Refresh the mode list from config
    pub fn refresh(&mut self, config: &Config) {
        let mut mode_names: Vec<String> = config.mode.keys().cloned().collect();
        mode_names.sort();
        self.mode_names = mode_names;

        // Ensure selected index is still valid
        if self.selected_mode >= self.mode_names.len() && !self.mode_names.is_empty() {
            self.selected_mode = self.mode_names.len() - 1;
        }
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Navigate up in the list
    pub fn up(&mut self) {
        if self.selected_mode > 0 {
            self.selected_mode -= 1;
        }
    }

    /// Navigate down in the list
    pub fn down(&mut self) {
        if !self.mode_names.is_empty() && self.selected_mode + 1 < self.mode_names.len() {
            self.selected_mode += 1;
        }
    }

    /// Get the currently selected mode name
    pub fn selected_mode_name(&self) -> Option<&str> {
        self.mode_names.get(self.selected_mode).map(|s| s.as_str())
    }
}

/// Render the mode panel popup
pub fn render(frame: &mut Frame, state: &ModePanelState, config: &Config) {
    if !state.visible {
        return;
    }

    let area = centered_rect(80, 80, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    // Split into left (mode list) and right (mode details)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    render_mode_list(frame, chunks[0], state);
    render_mode_details(frame, chunks[1], state, config);
}

/// Render the mode list on the left side
fn render_mode_list(frame: &mut Frame, area: Rect, state: &ModePanelState) {
    let items: Vec<ListItem> = state
        .mode_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let style = if i == state.selected_mode {
                Style::default()
                    .fg(CYAN)
                    .add_modifier(Modifier::BOLD)
                    .bg(ratatui::style::Color::Rgb(60, 60, 80))
            } else {
                Style::default().fg(WHITE)
            };

            ListItem::new(Line::from(Span::styled(format!("  {}  ", name), style)))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(" Modes ", Style::default().fg(CYAN).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(CYAN))
                .style(Style::default().bg(BG)),
        )
        .style(Style::default().bg(BG));

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_mode));

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Render mode details on the right side
fn render_mode_details(frame: &mut Frame, area: Rect, state: &ModePanelState, config: &Config) {
    let Some(mode_name) = state.selected_mode_name() else {
        let para = Paragraph::new("No modes configured")
            .block(
                Block::default()
                    .title(Span::styled(" Details ", Style::default().fg(DARK_GRAY)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DARK_GRAY))
                    .style(Style::default().bg(BG)),
            )
            .style(Style::default().bg(BG));
        frame.render_widget(para, area);
        return;
    };

    let Some(mode_config) = config.get_mode(mode_name) else {
        let para = Paragraph::new("Mode not found in config")
            .block(
                Block::default()
                    .title(Span::styled(" Details ", Style::default().fg(RED)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(RED))
                    .style(Style::default().bg(BG)),
            )
            .style(Style::default().bg(BG));
        frame.render_widget(para, area);
        return;
    };

    // Split details area into sections
    let detail_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Basic info
            Constraint::Min(4),     // Prompt template
            Constraint::Length(8),  // System prompt preview
        ])
        .split(area);

    render_basic_info(frame, detail_chunks[0], mode_name, mode_config);
    render_prompt_template(frame, detail_chunks[1], mode_config);
    render_system_prompt(frame, detail_chunks[2], mode_config);
}

/// Render basic mode information
fn render_basic_info(frame: &mut Frame, area: Rect, mode_name: &str, mode_config: &ModeConfig) {
    let aliases = if mode_config.aliases.is_empty() {
        "—".to_string()
    } else {
        mode_config.aliases.join(", ")
    };

    let agent = mode_config.agent.as_deref().unwrap_or("claude (default)");
    let target_default = mode_config.target_default.as_deref().unwrap_or("—");
    let scope_default = mode_config.scope_default.as_deref().unwrap_or("—");

    let lines = vec![
        Line::from(vec![
            Span::styled("Name     ", Style::default().fg(DARK_GRAY)),
            Span::styled(mode_name, Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Aliases  ", Style::default().fg(DARK_GRAY)),
            Span::styled(aliases, Style::default().fg(YELLOW)),
        ]),
        Line::from(vec![
            Span::styled("Agent    ", Style::default().fg(DARK_GRAY)),
            Span::styled(agent, Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("Target   ", Style::default().fg(DARK_GRAY)),
            Span::styled(target_default, Style::default().fg(GRAY)),
            Span::styled("    Scope    ", Style::default().fg(DARK_GRAY)),
            Span::styled(scope_default, Style::default().fg(GRAY)),
        ]),
    ];

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(" Mode Info ", Style::default().fg(YELLOW)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(YELLOW))
                .style(Style::default().bg(BG)),
        )
        .style(Style::default().bg(BG));

    frame.render_widget(para, area);
}

/// Render the prompt template section
fn render_prompt_template(frame: &mut Frame, area: Rect, mode_config: &ModeConfig) {
    let prompt_text = mode_config
        .prompt
        .as_deref()
        .unwrap_or("(no prompt template defined)");

    // Truncate to fit in the area
    let max_lines = area.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = prompt_text
        .lines()
        .take(max_lines)
        .map(|line| Line::from(Span::styled(line, Style::default().fg(WHITE))))
        .collect();

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(" Prompt Template ", Style::default().fg(GREEN)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(GREEN))
                .style(Style::default().bg(BG)),
        )
        .style(Style::default().bg(BG))
        .wrap(Wrap { trim: false });

    frame.render_widget(para, area);
}

/// Render system prompt preview
fn render_system_prompt(frame: &mut Frame, area: Rect, mode_config: &ModeConfig) {
    let system_text = mode_config
        .system_prompt
        .as_deref()
        .unwrap_or("(no system prompt defined)");

    // Show just a preview (first few lines)
    let max_lines = area.height.saturating_sub(2) as usize;
    let total_lines = system_text.lines().count();
    let mut lines: Vec<Line> = system_text
        .lines()
        .take(max_lines.saturating_sub(1))
        .map(|line| Line::from(Span::styled(line, Style::default().fg(GRAY))))
        .collect();

    // Add truncation indicator if needed
    if total_lines > max_lines {
        lines.push(Line::from(Span::styled(
            format!("... ({} more lines)", total_lines - max_lines + 1),
            Style::default().fg(DARK_GRAY),
        )));
    }

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(" System Prompt ", Style::default().fg(MAGENTA)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MAGENTA))
                .style(Style::default().bg(BG)),
        )
        .style(Style::default().bg(BG))
        .wrap(Wrap { trim: false });

    frame.render_widget(para, area);
}
