//! Selection popup component for the GUI
//!
//! Renders the selection popup that appears when code is selected in an IDE.
//! Allows the user to enter a mode and prompt to create a job.

use eframe::egui::{self, Color32, Id, RichText, Stroke, Vec2};
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

    // Animate popup fade-in using egui's built-in animation
    let fade_alpha = ctx.animate_bool_with_time(
        Id::new("selection_popup_fade"),
        true,
        0.2,  // 200ms fade duration
    );

    // Apply fade to the window frame
    let frame = egui::Frame::window(&ctx.style())
        .fill(Color32::from_rgba_unmultiplied(30, 34, 42, (fade_alpha * 250.0) as u8))
        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 176, 0, (fade_alpha * 100.0) as u8)));

    egui::Window::new("kyco")
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(450.0, 280.0))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(frame)
        .show(ctx, |ui| {
            // Fade content opacity
            ui.set_opacity(fade_alpha);
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

            // Header
            render_header(ui, state.selection);

            // Selection preview
            if let Some(text) = &state.selection.selected_text {
                render_selection_preview(ui, text, state.selection.line_number);
            }

            ui.add_space(8.0);

            // Main input with microphone button
            let input_result = render_input_field(ui, state);
            if input_result.input_changed {
                action = Some(SelectionPopupAction::InputChanged);
            }
            if input_result.mic_clicked {
                action = Some(SelectionPopupAction::ToggleRecording);
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
    egui::Frame::NONE
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

/// Result from rendering the input field
struct InputFieldResult {
    /// True if the text input changed
    input_changed: bool,
    /// True if the microphone button was clicked
    mic_clicked: bool,
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

                // Microphone button
                if render_microphone_button(ui, state) {
                    result.mic_clicked = true;
                }
            });
        });

    result
}

/// Render the microphone button
/// Returns true if the button was clicked and voice is enabled
fn render_microphone_button(ui: &mut egui::Ui, state: &SelectionPopupState<'_>) -> bool {
    let mut clicked = false;

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let (mic_icon, mic_color, mic_tooltip) = match state.voice_state {
            VoiceState::Recording => ("⏺", ACCENT_RED, "Recording... Press Enter to run or ⌘D to stop"),
            VoiceState::Transcribing => ("◌", STATUS_RUNNING, "Transcribing..."),
            VoiceState::Listening => ("◉", ACCENT_GREEN, "Listening for keywords..."),
            VoiceState::Error => ("!", ACCENT_RED, "Voice error - click to retry"),
            VoiceState::Idle => {
                if state.voice_mode == VoiceInputMode::Disabled {
                    ("○", TEXT_MUTED, "Voice disabled - enable in Settings")
                } else {
                    ("●", ACCENT_CYAN, "Click or ⌘D to record")
                }
            }
        };

        // Button is interactive unless transcribing
        let is_enabled = state.voice_state != VoiceState::Transcribing
            && state.voice_mode != VoiceInputMode::Disabled;

        let mic_button = ui.add_enabled(
            is_enabled,
            egui::Button::new(RichText::new(mic_icon).size(16.0).color(mic_color))
                .fill(Color32::TRANSPARENT)
                .frame(false),
        );

        if mic_button.clicked() {
            clicked = true;
        }

        mic_button.on_hover_text(mic_tooltip);
    });

    clicked
}

/// Render the voice status indicator
fn render_voice_status(ui: &mut egui::Ui, voice_state: VoiceState, last_error: Option<&str>) {
    if voice_state != VoiceState::Idle {
        ui.add_space(4.0);
        let (status_icon, status_text, status_color) = match voice_state {
            VoiceState::Recording => ("⏺", "Recording... Press Enter to run", ACCENT_RED),
            VoiceState::Transcribing => ("⏳", "Transcribing...", STATUS_RUNNING),
            VoiceState::Listening => ("◉", "Listening for mode keywords...", ACCENT_GREEN),
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
        egui::Frame::NONE
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

                    let response = egui::Frame::NONE
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

// =============================================================================
// Batch Popup (for processing multiple files)
// =============================================================================

use crate::gui::http_server::BatchFile;

/// State required for rendering the batch popup
pub struct BatchPopupState<'a> {
    pub batch_files: &'a [BatchFile],
    pub popup_input: &'a mut String,
    pub popup_status: &'a Option<(String, bool)>,
    pub suggestions: &'a [Suggestion],
    pub selected_suggestion: usize,
    pub show_suggestions: bool,
    pub cursor_to_end: &'a mut bool,
}

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

            // Header
            render_batch_header(ui, state.batch_files.len());

            // Files preview
            render_batch_files_preview(ui, state.batch_files);

            ui.add_space(8.0);

            // Main input (no microphone for batch)
            let input_changed = render_batch_input_field(ui, state);
            if input_changed {
                action = Some(SelectionPopupAction::InputChanged);
            }

            // Suggestions dropdown
            if let Some(idx) = render_batch_suggestions(ui, state) {
                action = Some(SelectionPopupAction::SuggestionClicked(idx));
            }

            // Status message
            render_status_message(ui, state.popup_status);

            // Help bar (simplified, no voice)
            render_batch_help_bar(ui);
        });

    action
}

/// Render the batch popup header
fn render_batch_header(ui: &mut egui::Ui, file_count: usize) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("▶ kyco batch").monospace().size(18.0).color(TEXT_PRIMARY));
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
                ui.label(RichText::new("FILES").small().monospace().color(ACCENT_CYAN));
                ui.label(
                    RichText::new(format!("({} total)", files.len()))
                        .small()
                        .color(TEXT_MUTED),
                );
            });

            // Show first few files
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
                ui.label(RichText::new("❯").monospace().size(16.0).color(TEXT_PRIMARY));
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

/// Render suggestions for batch popup
fn render_batch_suggestions(ui: &mut egui::Ui, state: &BatchPopupState<'_>) -> Option<usize> {
    let mut clicked_suggestion: Option<usize> = None;

    if state.show_suggestions && !state.suggestions.is_empty() {
        ui.add_space(4.0);
        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .stroke(Stroke::new(1.0, TEXT_MUTED.linear_multiply(0.3)))
            .corner_radius(4.0)
            .inner_margin(4.0)
            .show(ui, |ui| {
                let visible_suggestions = state.suggestions.iter().take(MAX_SUGGESTIONS_VISIBLE);
                for (idx, suggestion) in visible_suggestions.enumerate() {
                    let is_selected = idx == state.selected_suggestion;
                    let bg = if is_selected {
                        BG_HIGHLIGHT
                    } else {
                        Color32::TRANSPARENT
                    };

                    let response = egui::Frame::NONE
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
