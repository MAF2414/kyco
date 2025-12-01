//! Output schema settings component for the GUI
//!
//! Renders the output schema settings section in the settings view where users can:
//! - View and edit the agent output schema template
//! - The template is appended to agent system prompts for structured output

use eframe::egui::{self, RichText};

use super::app::{BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

/// State for output schema settings UI
pub struct OutputSchemaState<'a> {
    pub settings_output_schema: &'a mut String,
}

/// Render the output schema settings section
pub fn render_output_schema(ui: &mut egui::Ui, state: &mut OutputSchemaState<'_>) {
    ui.label(
        RichText::new("Agent Output Schema")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new("Template appended to agent system prompts for structured output.")
            .color(TEXT_DIM),
    );
    ui.add_space(12.0);

    render_section_frame(ui, |ui| {
        ui.label(
            RichText::new("Placeholders: ---kyco marker for YAML output")
                .small()
                .color(TEXT_MUTED),
        );
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::multiline(state.settings_output_schema)
                .font(egui::TextStyle::Monospace)
                .text_color(TEXT_PRIMARY)
                .desired_width(f32::INFINITY)
                .desired_rows(8),
        );
    });

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
}

/// Render a section frame with secondary background
fn render_section_frame<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::none()
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, add_contents)
        .inner
}
