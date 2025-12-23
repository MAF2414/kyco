//! UI helper functions for settings rendering
//!
//! Provides reusable UI components for the settings panel.

use eframe::egui::{self, RichText};

use crate::gui::app::{ACCENT_GREEN, ACCENT_RED, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

/// Render a labeled text input field
pub fn render_text_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    width: f32,
    hint: Option<&str>,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT_MUTED));
        let mut edit = egui::TextEdit::singleline(value)
            .font(egui::TextStyle::Monospace)
            .text_color(TEXT_PRIMARY)
            .desired_width(width);
        if let Some(h) = hint {
            edit = edit.hint_text(h);
        }
        ui.add(edit);
    });
}

/// Render a labeled text input field with a description on the same line
pub fn render_text_field_with_desc(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    width: f32,
    description: &str,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT_MUTED));
        ui.add(
            egui::TextEdit::singleline(value)
                .font(egui::TextStyle::Monospace)
                .text_color(TEXT_PRIMARY)
                .desired_width(width),
        );
        ui.label(RichText::new(description).small().color(TEXT_MUTED));
    });
}

/// Render a labeled checkbox with description
pub fn render_checkbox_field(ui: &mut egui::Ui, value: &mut bool, label: &str, description: &str) {
    ui.horizontal(|ui| {
        ui.checkbox(value, "");
        ui.label(RichText::new(label).color(TEXT_DIM));
        ui.label(RichText::new(description).small().color(TEXT_MUTED));
    });
}

/// Render a status message (success or error)
pub fn render_status_message(ui: &mut egui::Ui, status: &Option<(String, bool)>) {
    if let Some((msg, is_error)) = status {
        let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
        ui.label(RichText::new(msg).color(color));
    }
}

/// Render a section frame with secondary background
pub fn render_section_frame<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, add_contents)
        .inner
}
