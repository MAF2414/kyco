//! Orchestrator settings section

use eframe::egui::{self, RichText};

use crate::gui::theme::{ACCENT_GREEN, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

use super::super::helpers::{render_section_frame, render_status_message};
use super::super::save::save_settings_to_config;
use super::super::state::SettingsState;

/// Render Orchestrator Settings section
pub fn render_settings_orchestrator(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(
        RichText::new("Orchestrator")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new("Configure the external CLI session launched via the orchestrator button.")
            .color(TEXT_DIM),
    );
    ui.add_space(12.0);

    render_section_frame(ui, |ui| {
        // CLI Agent dropdown
        ui.label(RichText::new("CLI Agent").color(TEXT_MUTED));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("orchestrator_cli_agent")
                .selected_text(if state.orchestrator_cli_agent.is_empty() {
                    "claude"
                } else {
                    state.orchestrator_cli_agent.as_str()
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(state.orchestrator_cli_agent, "claude".to_string(), "claude");
                    ui.selectable_value(state.orchestrator_cli_agent, "codex".to_string(), "codex");
                });
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("The CLI agent to launch (claude or codex)")
                .small()
                .color(TEXT_MUTED),
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);

        // CLI Command
        ui.label(RichText::new("CLI Command (optional)").color(TEXT_MUTED));
        ui.add_space(4.0);
        ui.label(
            RichText::new("Use {prompt_file} as placeholder for the system prompt file path.")
                .small()
                .color(TEXT_MUTED),
        );
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(state.orchestrator_cli_command)
                .font(egui::TextStyle::Monospace)
                .text_color(TEXT_PRIMARY)
                .hint_text("Leave empty to auto-generate based on CLI Agent")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("Leave empty to use default based on CLI Agent above")
                .small()
                .color(TEXT_MUTED),
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);

        ui.label(RichText::new("System Prompt").color(TEXT_MUTED));
        ui.add_space(4.0);
        ui.label(
            RichText::new("Custom system prompt for the orchestrator. Leave empty to use the built-in default.")
                .small()
                .color(TEXT_MUTED),
        );
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::multiline(state.orchestrator_system_prompt)
                .font(egui::TextStyle::Monospace)
                .text_color(TEXT_PRIMARY)
                .hint_text("Custom orchestrator instructions...")
                .desired_width(f32::INFINITY)
                .desired_rows(8),
        );
    });

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new("Save Orchestrator Settings").color(ACCENT_GREEN))
            .clicked()
        {
            save_settings_to_config(state);
        }
    });

    render_status_message(ui, state.settings_status);

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
}
