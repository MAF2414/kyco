//! Advanced voice settings section

use eframe::egui::{self, RichText};

use crate::gui::theme::{TEXT_MUTED, TEXT_PRIMARY};

pub fn render_advanced_settings(
    ui: &mut egui::Ui,
    silence_threshold: &mut String,
    silence_duration: &mut String,
    max_duration: &mut String,
) {
    ui.collapsing("Advanced Settings", |ui| {
        ui.add_space(4.0);
        render_text_field_with_desc(ui, "Silence Threshold:", silence_threshold, 60.0, "(0.0-1.0)");
        ui.add_space(4.0);
        render_text_field_with_desc(ui, "Silence Duration:", silence_duration, 60.0, "seconds");
        ui.add_space(4.0);
        render_text_field_with_desc(ui, "Max Duration:", max_duration, 60.0, "seconds");
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
