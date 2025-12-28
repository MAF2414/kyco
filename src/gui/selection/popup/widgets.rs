//! Shared widget rendering for popups

use eframe::egui::{self, Color32, RichText, Stroke};

use super::super::autocomplete::Suggestion;
use super::types::MAX_SUGGESTIONS_VISIBLE;
use crate::gui::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, ACCENT_YELLOW, BG_HIGHLIGHT, BG_SECONDARY,
    STATUS_RUNNING, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::gui::voice::VoiceState;

/// Render the voice status indicator
pub(crate) fn render_voice_status(
    ui: &mut egui::Ui,
    voice_state: VoiceState,
    last_error: Option<&str>,
) {
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

/// Render the status message
pub(crate) fn render_status_message(ui: &mut egui::Ui, popup_status: &Option<(String, bool)>) {
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

/// Render suggestions list
/// Returns the index of clicked suggestion if any
pub(crate) fn render_suggestions_list(
    ui: &mut egui::Ui,
    suggestions: &[Suggestion],
    selected_suggestion: usize,
    show_suggestions: bool,
) -> Option<usize> {
    let mut clicked_suggestion: Option<usize> = None;

    if show_suggestions && !suggestions.is_empty() {
        ui.add_space(4.0);
        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .stroke(Stroke::new(1.0, TEXT_MUTED.linear_multiply(0.3)))
            .corner_radius(4.0)
            .inner_margin(4.0)
            .show(ui, |ui| {
                let visible_suggestions = suggestions.iter().take(MAX_SUGGESTIONS_VISIBLE);
                for (idx, suggestion) in visible_suggestions.enumerate() {
                    let is_selected = idx == selected_suggestion;
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
                                    "chain" => (ACCENT_YELLOW, "CHN"),
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

/// Render the microphone button
/// Returns true if the button was clicked
pub(crate) fn render_microphone_button(ui: &mut egui::Ui, voice_state: VoiceState) -> bool {
    let mut clicked = false;

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let (mic_icon, mic_color, mic_tooltip) = match voice_state {
            VoiceState::Recording => (
                "⏺",
                ACCENT_RED,
                "Recording... Press Enter to run or ⌘D to stop",
            ),
            VoiceState::Transcribing => ("◌", STATUS_RUNNING, "Transcribing..."),
            VoiceState::Listening => ("◉", ACCENT_GREEN, "Listening for keywords..."),
            VoiceState::Error => ("!", ACCENT_RED, "Voice error - click to retry"),
            VoiceState::Idle => {
                // Always show as clickable - will auto-install if needed
                ("●", ACCENT_CYAN, "Click or ⌘D to record")
            }
        };

        // Button is interactive unless transcribing (will auto-install if dependencies missing)
        let is_enabled = voice_state != VoiceState::Transcribing;

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
