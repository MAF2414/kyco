//! Selection popup component for the GUI
//!
//! Renders the selection popup that appears when code is selected in an IDE.
//! Allows the user to enter a mode and prompt to create a job.

use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use std::path::PathBuf;

use super::context::SelectionContext;
use super::autocomplete::Suggestion;
use crate::gui::app::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_RED, BG_HIGHLIGHT, BG_SECONDARY,
    STATUS_RUNNING, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::gui::voice::{VoiceInputMode, VoiceState};

/// Actions that can be triggered from the selection popup
#[derive(Debug, Clone)]
pub enum SelectionPopupAction {
    /// User changed the input text
    InputChanged,
    /// User clicked a suggestion
    SuggestionClicked(usize),
    /// User toggled voice recording
    ToggleRecording,
}

/// State required for rendering the selection popup
pub struct SelectionPopupState<'a> {
    pub selection: &'a SelectionContext,
    pub popup_input: &'a mut String,
    pub popup_status: &'a Option<(String, bool)>,
    pub suggestions: &'a [Suggestion],
    pub selected_suggestion: usize,
    pub show_suggestions: bool,
    pub cursor_to_end: &'a mut bool,
    pub voice_state: VoiceState,
    pub voice_mode: VoiceInputMode,
    pub voice_last_error: Option<&'a str>,
}

/// Render the selection popup and return any action triggered by the user
pub fn render_selection_popup(
    ctx: &egui::Context,
    state: &mut SelectionPopupState<'_>,
) -> Option<SelectionPopupAction> {
    let mut action: Option<SelectionPopupAction> = None;

    egui::Window::new("kyco")
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(450.0, 280.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

            // Header
            render_header(ui, state.selection);

            // Selection preview
            if let Some(text) = &state.selection.selected_text {
                render_selection_preview(ui, text, state.selection.line_number);
            }

            ui.add_space(8.0);

            // Main input with microphone button
            if render_input_field(ui, state) {
                action = Some(SelectionPopupAction::InputChanged);
            }

            // Voice status indicator
            render_voice_status(ui, state.voice_state, state.voice_last_error);

            // Suggestions dropdown
            if let Some(idx) = render_suggestions(ui, state) {
                action = Some(SelectionPopupAction::SuggestionClicked(idx));
            }

            // Status message
            render_status_message(ui, state.popup_status);

            // Help bar
            render_help_bar(ui);
        });

    action
}

/// Render the popup header with title and filename
fn render_header(ui: &mut egui::Ui, selection: &SelectionContext) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("▶ kyco").monospace().size(18.0).color(TEXT_PRIMARY));
        ui.label(RichText::new("█").monospace().color(TEXT_PRIMARY));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(file) = &selection.file_path {
                let filename = PathBuf::from(file)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| file.clone());
                ui.label(RichText::new(format!("← {}", filename)).small().color(TEXT_MUTED));
            }
        });
    });
}

/// Render the selection preview panel
fn render_selection_preview(ui: &mut egui::Ui, text: &str, start_line: Option<usize>) {
    ui.add_space(4.0);
    egui::Frame::none()
        .fill(BG_SECONDARY)
        .stroke(Stroke::new(1.0, ACCENT_PURPLE.linear_multiply(0.3)))
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            let line_count = text.lines().count();
            let char_count = text.len();

            let line_info = if let Some(start) = start_line {
                if line_count == 1 {
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
                ui.label(RichText::new("SEL").small().monospace().color(ACCENT_PURPLE));
                ui.label(
                    RichText::new(&line_info)
                        .small()
                        .monospace()
                        .color(ACCENT_PURPLE.linear_multiply(0.7)),
                );
                ui.label(RichText::new(format!("({}c)", char_count)).small().color(TEXT_MUTED));
            });
            ui.label(RichText::new(&preview).small().monospace().color(TEXT_DIM));
        });
}

/// Render the main input field with microphone button
/// Returns true if input changed
fn render_input_field(ui: &mut egui::Ui, state: &mut SelectionPopupState<'_>) -> bool {
    let mut input_changed = false;

    egui::Frame::none()
        .fill(BG_SECONDARY)
        .stroke(Stroke::new(2.0, TEXT_PRIMARY.linear_multiply(0.4)))
        .corner_radius(4.0)
        .inner_margin(egui::vec2(12.0, 10.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("❯").monospace().size(16.0).color(TEXT_PRIMARY));
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

                // Microphone button
                render_microphone_button(ui, state);
            });
        });

    input_changed
}

/// Render the microphone button
fn render_microphone_button(ui: &mut egui::Ui, state: &mut SelectionPopupState<'_>) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let (mic_icon, mic_color, mic_tooltip) = match state.voice_state {
            VoiceState::Recording => ("⏺", ACCENT_RED, "Recording... Click to stop"),
            VoiceState::Transcribing => ("⏳", STATUS_RUNNING, "Transcribing..."),
            VoiceState::Listening => ("👂", ACCENT_GREEN, "Listening for keywords..."),
            VoiceState::Error => ("⚠", ACCENT_RED, "Voice error"),
            VoiceState::Idle => {
                if state.voice_mode == VoiceInputMode::Disabled {
                    ("🎤", TEXT_MUTED, "Voice input disabled (enable in Settings)")
                } else {
                    ("🎤", TEXT_DIM, "Click to record")
                }
            }
        };

        let mic_button = ui.add(
            egui::Button::new(RichText::new(mic_icon).size(16.0).color(mic_color))
                .fill(Color32::TRANSPARENT)
                .frame(false),
        );

        // Note: The actual toggle_recording() call must be done by the caller
        // since we can't mutate voice_manager here
        if mic_button.clicked() && state.voice_mode != VoiceInputMode::Disabled {
            // The caller should handle this via SelectionPopupAction::ToggleRecording
            // but for now we just consume the click
        }

        mic_button.on_hover_text(mic_tooltip);
    });
}

/// Render the voice status indicator
fn render_voice_status(ui: &mut egui::Ui, voice_state: VoiceState, last_error: Option<&str>) {
    if voice_state != VoiceState::Idle {
        ui.add_space(4.0);
        let (status_icon, status_text, status_color) = match voice_state {
            VoiceState::Recording => ("⏺", "Recording...", ACCENT_RED),
            VoiceState::Transcribing => ("⏳", "Transcribing...", STATUS_RUNNING),
            VoiceState::Listening => ("👂", "Listening for mode keywords...", ACCENT_GREEN),
            VoiceState::Error => ("⚠", last_error.unwrap_or("Voice error"), ACCENT_RED),
            VoiceState::Idle => ("", "", TEXT_MUTED),
        };
        if !status_text.is_empty() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(status_icon).color(status_color));
                ui.label(RichText::new(status_text).small().color(status_color));
            });
        }
    }
}

/// Maximum number of suggestions to display
const MAX_SUGGESTIONS_VISIBLE: usize = 5;

/// Render the suggestions dropdown
/// Returns the index of clicked suggestion if any
fn render_suggestions(ui: &mut egui::Ui, state: &SelectionPopupState<'_>) -> Option<usize> {
    let mut clicked_suggestion: Option<usize> = None;

    if state.show_suggestions && !state.suggestions.is_empty() {
        ui.add_space(4.0);
        egui::Frame::none()
            .fill(BG_SECONDARY)
            .stroke(Stroke::new(1.0, TEXT_MUTED.linear_multiply(0.3)))
            .corner_radius(4.0)
            .inner_margin(4.0)
            .show(ui, |ui| {
                // Only show up to MAX_SUGGESTIONS_VISIBLE suggestions
                let visible_suggestions = state.suggestions.iter().take(MAX_SUGGESTIONS_VISIBLE);
                for (idx, suggestion) in visible_suggestions.enumerate() {
                    let is_selected = idx == state.selected_suggestion;
                    let bg = if is_selected {
                        BG_HIGHLIGHT
                    } else {
                        Color32::TRANSPARENT
                    };

                    let response = egui::Frame::none()
                        .fill(bg)
                        .corner_radius(2.0)
                        .inner_margin(egui::vec2(8.0, 4.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (badge_color, badge_text) = match suggestion.category {
                                    "agent" => (ACCENT_CYAN, "AGT"),
                                    "mode" => (ACCENT_GREEN, "MOD"),
                                    _ => (TEXT_MUTED, "???"),
                                };
                                ui.label(
                                    RichText::new(badge_text)
                                        .small()
                                        .monospace()
                                        .color(badge_color),
                                );
                                ui.add_space(8.0);
                                let text_color = if is_selected { TEXT_PRIMARY } else { TEXT_DIM };
                                ui.label(
                                    RichText::new(&suggestion.text)
                                        .monospace()
                                        .color(text_color),
                                );
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(&suggestion.description)
                                        .small()
                                        .color(TEXT_MUTED),
                                );
                            });
                        });

                    if response.response.interact(egui::Sense::click()).clicked() {
                        clicked_suggestion = Some(idx);
                    }
                }
            });
    }

    clicked_suggestion
}

/// Render the status message
fn render_status_message(ui: &mut egui::Ui, popup_status: &Option<(String, bool)>) {
    if let Some((msg, is_error)) = popup_status {
        ui.add_space(8.0);
        let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
        let prefix = if *is_error { "ERR" } else { " OK" };
        ui.horizontal(|ui| {
            ui.label(RichText::new(prefix).small().monospace().color(color));
            ui.label(RichText::new(msg).small().color(color));
        });
    }
}

/// Render the help bar at the bottom
fn render_help_bar(ui: &mut egui::Ui) {
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("TAB").small().monospace().color(TEXT_DIM));
            ui.label(RichText::new("complete").small().color(TEXT_MUTED));
            ui.add_space(12.0);
            ui.label(RichText::new("↵").small().monospace().color(TEXT_DIM));
            ui.label(RichText::new("run").small().color(TEXT_MUTED));
            ui.add_space(12.0);
            ui.label(RichText::new("ESC").small().monospace().color(TEXT_DIM));
            ui.label(RichText::new("close").small().color(TEXT_MUTED));
        });
    });
}
