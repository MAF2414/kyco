//! Diff view module for displaying git diffs
//!
//! This module provides the diff popup UI and related functionality.

use eframe::egui::{self, Color32, RichText, ScrollArea, Vec2};

// Import color constants from parent module
use super::app::{ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, TEXT_DIM};

/// State for the diff viewer
#[derive(Default)]
pub struct DiffState {
    /// Diff content to display
    pub content: Option<String>,
    /// Diff scroll offset (reserved for future use)
    #[allow(dead_code)]
    pub scroll: f32,
}

impl DiffState {
    /// Create a new diff state
    pub fn new() -> Self {
        Self {
            content: None,
            scroll: 0.0,
        }
    }

    /// Set the diff content to display
    pub fn set_content(&mut self, content: String) {
        self.content = Some(content);
    }

    /// Clear the diff content
    pub fn clear(&mut self) {
        self.content = None;
        self.scroll = 0.0;
    }

    /// Check if there is content to display
    pub fn has_content(&self) -> bool {
        self.content.is_some()
    }
}

/// Render the diff view popup
///
/// Returns true if the close button was clicked
pub fn render_diff_popup(ctx: &egui::Context, diff_state: &DiffState) -> bool {
    let mut should_close = false;

    egui::Window::new("Diff")
        .collapsible(false)
        .resizable(true)
        .default_size(Vec2::new(600.0, 400.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if let Some(diff) = &diff_state.content {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for line in diff.lines() {
                            let color = get_diff_line_color(line);
                            ui.label(RichText::new(line).monospace().color(color));
                        }
                    });
            }

            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    should_close = true;
                }
            });
        });

    should_close
}

/// Get the appropriate color for a diff line based on its prefix
fn get_diff_line_color(line: &str) -> Color32 {
    if line.starts_with('+') {
        ACCENT_GREEN
    } else if line.starts_with('-') {
        ACCENT_RED
    } else if line.starts_with("@@") {
        ACCENT_CYAN
    } else {
        TEXT_DIM
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_state_new() {
        let state = DiffState::new();
        assert!(state.content.is_none());
        assert!(!state.has_content());
    }

    #[test]
    fn test_diff_state_set_content() {
        let mut state = DiffState::new();
        state.set_content("test diff".to_string());
        assert!(state.has_content());
        assert_eq!(state.content.as_deref(), Some("test diff"));
    }

    #[test]
    fn test_diff_state_clear() {
        let mut state = DiffState::new();
        state.set_content("test diff".to_string());
        state.clear();
        assert!(!state.has_content());
    }

    #[test]
    fn test_get_diff_line_color() {
        assert_eq!(get_diff_line_color("+added"), ACCENT_GREEN);
        assert_eq!(get_diff_line_color("-removed"), ACCENT_RED);
        assert_eq!(get_diff_line_color("@@ -1,2 +1,3 @@"), ACCENT_CYAN);
        assert_eq!(get_diff_line_color(" context"), TEXT_DIM);
    }
}
