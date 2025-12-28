//! Selection popup rendering

use eframe::egui::{self, Color32, Id, RichText, Stroke, Vec2};
use std::path::PathBuf;

use super::super::context::SelectionContext;
use super::types::{InputFieldResult, SelectionPopupAction, SelectionPopupState};
use super::widgets::{
    render_microphone_button, render_status_message, render_suggestions_list, render_voice_status,
};
use crate::gui::theme::{
    ACCENT_PURPLE, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};

/// Render the selection popup and return any action triggered by the user
pub fn render_selection_popup(
    ctx: &egui::Context,
    state: &mut SelectionPopupState<'_>,
) -> Option<SelectionPopupAction> {
    let mut action: Option<SelectionPopupAction> = None;

    // Animate popup fade-in using egui's built-in animation
    let fade_alpha = ctx.animate_bool_with_time(
        Id::new("selection_popup_fade"),
        true,
        0.2, // 200ms fade duration
    );

    let frame = egui::Frame::window(&ctx.style())
        .fill(Color32::from_rgba_unmultiplied(
            30,
            34,
            42,
            (fade_alpha * 250.0) as u8,
        ))
        .stroke(Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(255, 176, 0, (fade_alpha * 100.0) as u8),
        ));

    egui::Window::new("kyco")
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(450.0, 280.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_opacity(fade_alpha);
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

            render_header(ui, state.selection);

            if let Some(text) = &state.selection.selected_text {
                render_selection_preview(ui, text, state.selection.line_number);
            }

            ui.add_space(8.0);

            let input_result = render_input_field(ui, state);
            if input_result.input_changed {
                action = Some(SelectionPopupAction::InputChanged);
            }
            if input_result.mic_clicked {
                action = Some(SelectionPopupAction::ToggleRecording);
            }

            render_voice_status(ui, state.voice_state, state.voice_last_error);

            if let Some(idx) = render_suggestions_list(
                ui,
                state.suggestions,
                state.selected_suggestion,
                state.show_suggestions,
            ) {
                action = Some(SelectionPopupAction::SuggestionClicked(idx));
            }

            render_status_message(ui, state.popup_status);

            render_help_bar(ui);
        });

    action
}

/// Render the popup header with title and filename
fn render_header(ui: &mut egui::Ui, selection: &SelectionContext) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("▶ kyco")
                .monospace()
                .size(18.0)
                .color(TEXT_PRIMARY),
        );
        ui.label(RichText::new("█").monospace().color(TEXT_PRIMARY));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(file) = &selection.file_path {
                let filename = PathBuf::from(file)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| file.clone());
                ui.label(
                    RichText::new(format!("← {}", filename))
                        .small()
                        .color(TEXT_MUTED),
                );
            }
        });
    });
}

/// Render the selection preview panel
fn render_selection_preview(ui: &mut egui::Ui, text: &str, start_line: Option<usize>) {
    ui.add_space(4.0);
    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .stroke(Stroke::new(1.0, ACCENT_PURPLE.linear_multiply(0.3)))
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            let line_count = text.lines().count();
            let char_count = text.len();

            let line_info = if let Some(start) = start_line {
                if line_count <= 1 {
                    format!("L{}", start)
                } else {
                    format!("L{}-{}", start, start + line_count - 1)
                }
            } else {
                format!("{} lines", line_count)
            };

            let first_line = text.lines().next().unwrap_or("");
            let preview = if first_line.len() > 50 {
                format!("{}…", first_line.chars().take(50).collect::<String>())
            } else if line_count > 1 {
                format!("{} …", first_line)
            } else {
                first_line.to_string()
            };

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("SEL")
                        .small()
                        .monospace()
                        .color(ACCENT_PURPLE),
                );
                ui.label(
                    RichText::new(&line_info)
                        .small()
                        .monospace()
                        .color(ACCENT_PURPLE.linear_multiply(0.7)),
                );
                ui.label(
                    RichText::new(format!("({}c)", char_count))
                        .small()
                        .color(TEXT_MUTED),
                );
            });
            ui.label(RichText::new(&preview).small().monospace().color(TEXT_DIM));
        });
}

/// Render the main input field with microphone button
/// Returns the result indicating what actions occurred
fn render_input_field(ui: &mut egui::Ui, state: &mut SelectionPopupState<'_>) -> InputFieldResult {
    let mut result = InputFieldResult {
        input_changed: false,
        mic_clicked: false,
    };

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
                    .hint_text(RichText::new("mode [prompt]").color(TEXT_MUTED))
                    .desired_width(ui.available_width() - 40.0)
                    .frame(false)
                    .lock_focus(true);

                let output = text_edit.show(ui);

                if output.response.changed() {
                    result.input_changed = true;
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

                if render_microphone_button(ui, state.voice_state) {
                    result.mic_clicked = true;
                }
            });
        });

    result
}

/// Render the help bar at the bottom
fn render_help_bar(ui: &mut egui::Ui) {
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⌘D").small().monospace().color(TEXT_DIM));
            ui.label(RichText::new("voice").small().color(TEXT_MUTED));
            ui.add_space(12.0);
            ui.label(RichText::new("TAB").small().monospace().color(TEXT_DIM));
            ui.label(RichText::new("complete").small().color(TEXT_MUTED));
            ui.add_space(12.0);
            ui.label(RichText::new("↵").small().monospace().color(TEXT_DIM));
            ui.label(RichText::new("run").small().color(TEXT_MUTED));
            ui.add_space(12.0);
            ui.label(RichText::new("⇧↵").small().monospace().color(TEXT_DIM));
            ui.label(RichText::new("worktree").small().color(TEXT_MUTED));
            ui.add_space(12.0);
            ui.label(RichText::new("ESC").small().monospace().color(TEXT_DIM));
            ui.label(RichText::new("close").small().color(TEXT_MUTED));
        });
    });
}
