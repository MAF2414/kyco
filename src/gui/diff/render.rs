//! Diff rendering functions

use eframe::egui::{self, Color32, Frame, RichText, ScrollArea, Stroke, Vec2};

use crate::gui::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, BG_HIGHLIGHT, BG_PRIMARY, BG_SECONDARY, TEXT_DIM,
    TEXT_MUTED, TEXT_PRIMARY,
};

use super::state::DiffState;
use super::{BG_ADDED, BG_HUNK, BG_REMOVED};

/// Parsed hunk header info
pub(super) struct HunkInfo {
    pub old_start: u32,
    pub new_start: u32,
}

/// Parse hunk header like "@@ -10,5 +12,7 @@ fn foo()"
pub(super) fn parse_hunk_header(line: &str) -> Option<HunkInfo> {
    // Format: @@ -old_start,old_count +new_start,new_count @@ context
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 || parts[0] != "@@" {
        return None;
    }

    let old_part = parts[1].trim_start_matches('-');
    let new_part = parts[2].trim_start_matches('+');

    let old_start = old_part
        .split(',')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let new_start = new_part
        .split(',')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    Some(HunkInfo {
        old_start,
        new_start,
    })
}

/// Render the diff view popup
///
/// Returns true if the close button was clicked
pub fn render_diff_popup(ctx: &egui::Context, diff_state: &DiffState) -> bool {
    let mut should_close = false;

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        return true;
    }

    let screen_rect = ctx.screen_rect();
    let window_size = Vec2::new(
        (screen_rect.width() * 0.85).min(1000.0).max(500.0),
        (screen_rect.height() * 0.85).min(700.0).max(400.0),
    );

    egui::Window::new("Diff")
        .collapsible(false)
        .resizable(true)
        .default_size(window_size)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            Frame::window(&ctx.style())
                .fill(BG_PRIMARY)
                .corner_radius(8.0),
        )
        .show(ctx, |ui| {
            if let Some(path) = &diff_state.file_path {
                Frame::group(ui.style())
                    .fill(BG_SECONDARY)
                    .corner_radius(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("📄").size(14.0));
                            ui.label(
                                RichText::new(path)
                                    .monospace()
                                    .color(TEXT_PRIMARY)
                                    .size(13.0),
                            );
                        });
                    });
                ui.add_space(8.0);
            }

            if let Some(diff) = &diff_state.content {
                let available_height = ui.available_height() - 40.0; // Reserve space for button

                Frame::group(ui.style())
                    .fill(BG_SECONDARY)
                    .corner_radius(4.0)
                    .stroke(Stroke::new(1.0, BG_HIGHLIGHT))
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .max_height(available_height)
                            .show(ui, |ui| {
                                render_diff_content(ui, diff);
                            });
                    });
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label(RichText::new("No diff content").color(TEXT_MUTED));
                });
            }

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(RichText::new("Close (Esc)").color(TEXT_DIM))
                        .clicked()
                    {
                        should_close = true;
                    }
                });
            });
        });

    should_close
}

/// Render diff content with line numbers and colored backgrounds
///
/// This function can be used both in the diff popup and inline in other panels.
pub fn render_diff_content(ui: &mut egui::Ui, diff: &str) {
    let mut old_line_num: u32 = 0;
    let mut new_line_num: u32 = 0;
    let mut in_header = true;

    for line in diff.lines() {
        if in_header {
            if line.starts_with("@@") {
                in_header = false;
            } else {
                render_header_line(ui, line);
                continue;
            }
        }

        if line.starts_with("@@") {
            if let Some(info) = parse_hunk_header(line) {
                old_line_num = info.old_start;
                new_line_num = info.new_start;
            }
            render_hunk_header(ui, line);
            continue;
        }

        let (line_type, display_line) = if let Some(rest) = line.strip_prefix('+') {
            (LineType::Added, rest)
        } else if let Some(rest) = line.strip_prefix('-') {
            (LineType::Removed, rest)
        } else if let Some(rest) = line.strip_prefix(' ') {
            (LineType::Context, rest)
        } else {
            (LineType::Context, line)
        };

        let (old_num, new_num) = match line_type {
            LineType::Added => {
                let n = new_line_num;
                new_line_num += 1;
                (None, Some(n))
            }
            LineType::Removed => {
                let n = old_line_num;
                old_line_num += 1;
                (Some(n), None)
            }
            LineType::Context => {
                let o = old_line_num;
                let n = new_line_num;
                old_line_num += 1;
                new_line_num += 1;
                (Some(o), Some(n))
            }
        };

        render_diff_line(ui, display_line, line_type, old_num, new_num);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum LineType {
    Added,
    Removed,
    Context,
}

/// Render a header line (diff --git, index, ---, +++)
fn render_header_line(ui: &mut egui::Ui, line: &str) {
    Frame::NONE
        .fill(BG_HIGHLIGHT)
        .inner_margin(egui::vec2(8.0, 2.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(line).monospace().color(TEXT_MUTED).size(12.0));
            });
        });
}

/// Render a hunk header (@@ ... @@)
fn render_hunk_header(ui: &mut egui::Ui, line: &str) {
    ui.add_space(4.0);
    Frame::NONE
        .fill(BG_HUNK)
        .corner_radius(2.0)
        .inner_margin(egui::vec2(8.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(line)
                        .monospace()
                        .color(ACCENT_CYAN)
                        .size(12.0),
                );
            });
        });
    ui.add_space(2.0);
}

/// Render a single diff line with line numbers
fn render_diff_line(
    ui: &mut egui::Ui,
    content: &str,
    line_type: LineType,
    old_num: Option<u32>,
    new_num: Option<u32>,
) {
    let (bg_color, text_color, prefix) = match line_type {
        LineType::Added => (BG_ADDED, ACCENT_GREEN, "+"),
        LineType::Removed => (BG_REMOVED, ACCENT_RED, "-"),
        LineType::Context => (Color32::TRANSPARENT, TEXT_DIM, " "),
    };

    Frame::NONE
        .fill(bg_color)
        .inner_margin(egui::vec2(0.0, 1.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let old_str = old_num
                    .map(|n| format!("{:4}", n))
                    .unwrap_or_else(|| "    ".to_string());
                ui.label(
                    RichText::new(&old_str)
                        .monospace()
                        .color(TEXT_MUTED)
                        .size(11.0),
                );

                let new_str = new_num
                    .map(|n| format!("{:4}", n))
                    .unwrap_or_else(|| "    ".to_string());
                ui.label(
                    RichText::new(&new_str)
                        .monospace()
                        .color(TEXT_MUTED)
                        .size(11.0),
                );

                ui.label(RichText::new("│").color(BG_HIGHLIGHT).size(12.0));

                ui.label(
                    RichText::new(prefix)
                        .monospace()
                        .color(text_color)
                        .size(12.0),
                );

                ui.label(
                    RichText::new(content)
                        .monospace()
                        .color(text_color)
                        .size(12.0),
                );
            });
        });
}
