//! Output schema settings section

use eframe::egui::{self, RichText};

use crate::gui::theme::{TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

use super::super::helpers::render_section_frame;
use super::super::state::SettingsState;

/// Render Output Schema section
pub fn render_settings_output_schema(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(
        RichText::new("Agent Output Schema")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "YAML summary template is appended to system prompts. Optional JSON Schema enables true SDK structured output.",
        )
            .color(TEXT_DIM),
    );
    ui.add_space(12.0);

    render_section_frame(ui, |ui| {
        ui.label(RichText::new("YAML Summary Template").color(TEXT_MUTED));
        ui.add_space(4.0);
        ui.label(
            RichText::new("Placeholders: --- markers for YAML output")
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

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);

        ui.label(RichText::new("Structured Output (JSON Schema, optional)").color(TEXT_MUTED));
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "When set, Claude/Codex SDK will be asked to return JSON matching this schema.",
            )
            .small()
            .color(TEXT_MUTED),
        );
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::multiline(state.settings_structured_output_schema)
                .font(egui::TextStyle::Monospace)
                .text_color(TEXT_PRIMARY)
                .hint_text("{\n  \"type\": \"object\",\n  \"properties\": { ... }\n}")
                .desired_width(f32::INFINITY)
                .desired_rows(8),
        );
    });

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
}
