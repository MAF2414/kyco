//! Batch popup rendering

use eframe::egui::{self, RichText, Stroke, Vec2};
use std::path::PathBuf;

use super::types::{BatchPopupState, SelectionPopupAction};
use super::widgets::{render_status_message, render_suggestions_list};
use crate::gui::http_server::BatchFile;
use crate::gui::theme::{
    ACCENT_CYAN, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};

/// Render the batch popup and return any action triggered by the user
pub fn render_batch_popup(
    ctx: &egui::Context,
    state: &mut BatchPopupState<'_>,
) -> Option<SelectionPopupAction> {
    let mut action: Option<SelectionPopupAction> = None;

    egui::Window::new("kyco batch")
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(500.0, 350.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

            render_batch_header(ui, state.batch_files.len());

            render_batch_files_preview(ui, state.batch_files);

            ui.add_space(8.0);

            let input_changed = render_batch_input_field(ui, state);
            if input_changed {
                action = Some(SelectionPopupAction::InputChanged);
            }

            if let Some(idx) = render_suggestions_list(
                ui,
                state.suggestions,
                state.selected_suggestion,
                state.show_suggestions,
            ) {
                action = Some(SelectionPopupAction::SuggestionClicked(idx));
            }

            render_status_message(ui, state.popup_status);

            render_batch_help_bar(ui);
        });

    action
}

/// Render the batch popup header
fn render_batch_header(ui: &mut egui::Ui, file_count: usize) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("▶ kyco batch")
                .monospace()
                .size(18.0)
                .color(TEXT_PRIMARY),
        );
        ui.label(RichText::new("█").monospace().color(TEXT_PRIMARY));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{} files", file_count))
                    .small()
                    .color(ACCENT_CYAN),
            );
        });
    });
}

/// Render the batch files preview
fn render_batch_files_preview(ui: &mut egui::Ui, files: &[BatchFile]) {
    ui.add_space(4.0);
    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .stroke(Stroke::new(1.0, ACCENT_CYAN.linear_multiply(0.3)))
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("FILES")
                        .small()
                        .monospace()
                        .color(ACCENT_CYAN),
                );
                ui.label(
                    RichText::new(format!("({} total)", files.len()))
                        .small()
                        .color(TEXT_MUTED),
                );
            });

            let max_display = 5;
            egui::ScrollArea::vertical()
                .max_height(80.0)
                .show(ui, |ui| {
                    for (i, file) in files.iter().take(max_display).enumerate() {
                        let filename = PathBuf::from(&file.path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| file.path.clone());

                        let line_info = match (file.line_start, file.line_end) {
                            (Some(s), Some(e)) if s != e => format!(":{}-{}", s, e),
                            (Some(s), _) => format!(":{}", s),
                            _ => String::new(),
                        };

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{:2}.", i + 1))
                                    .small()
                                    .monospace()
                                    .color(TEXT_MUTED),
                            );
                            ui.label(
                                RichText::new(format!("{}{}", filename, line_info))
                                    .small()
                                    .monospace()
                                    .color(TEXT_DIM),
                            );
                        });
                    }

                    if files.len() > max_display {
                        ui.label(
                            RichText::new(format!("   ... and {} more", files.len() - max_display))
                                .small()
                                .color(TEXT_MUTED),
                        );
                    }
                });
        });
}

/// Render the batch input field (no microphone)
fn render_batch_input_field(ui: &mut egui::Ui, state: &mut BatchPopupState<'_>) -> bool {
    let mut input_changed = false;

    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .stroke(Stroke::new(2.0, TEXT_PRIMARY.linear_multiply(0.4)))
        .corner_radius(4.0)
        .inner_margin(egui::vec2(12.0, 10.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("❯")
                        .monospace()
                        .size(16.0)
                        .color(TEXT_PRIMARY),
                );
                ui.add_space(4.0);

                let text_edit = egui::TextEdit::singleline(state.popup_input)
                    .font(egui::TextStyle::Monospace)
                    .text_color(TEXT_PRIMARY)
                    .hint_text(RichText::new("[agent+agent:]mode [prompt]").color(TEXT_MUTED))
                    .desired_width(ui.available_width())
                    .frame(false)
                    .lock_focus(true);

                let output = text_edit.show(ui);

                if output.response.changed() {
                    input_changed = true;
                }
                output.response.request_focus();

                // Move cursor to end if requested (after Tab completion)
                if *state.cursor_to_end {
                    *state.cursor_to_end = false;
                    if let Some(mut edit_state) =
                        egui::TextEdit::load_state(ui.ctx(), output.response.id)
                    {
                        let cursor_pos = egui::text::CCursor::new(state.popup_input.len());
                        edit_state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(cursor_pos)));
                        edit_state.store(ui.ctx(), output.response.id);
                    }
                }
            });
        });

    input_changed
}

/// Render the batch help bar (no voice option)
fn render_batch_help_bar(ui: &mut egui::Ui) {
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("TAB").small().monospace().color(TEXT_DIM));
            ui.label(RichText::new("complete").small().color(TEXT_MUTED));
            ui.add_space(12.0);
            ui.label(RichText::new("↵").small().monospace().color(TEXT_DIM));
            ui.label(RichText::new("run all").small().color(TEXT_MUTED));
            ui.add_space(12.0);
            ui.label(RichText::new("⇧↵").small().monospace().color(TEXT_DIM));
            ui.label(RichText::new("worktree").small().color(TEXT_MUTED));
            ui.add_space(12.0);
            ui.label(RichText::new("ESC").small().monospace().color(TEXT_DIM));
            ui.label(RichText::new("cancel").small().color(TEXT_MUTED));
        });
    });
}
