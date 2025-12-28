//! IDE extensions settings section

use eframe::egui::{self, Color32, RichText};

use crate::gui::theme::{ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

use super::super::helpers::render_section_frame;
use super::super::state::SettingsState;

/// Render IDE Extensions section
pub fn render_settings_ide_extensions(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(
        RichText::new("IDE Extensions")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new("Install extensions to send code selections to kyco with a hotkey.")
            .color(TEXT_DIM),
    );
    ui.add_space(12.0);

    // VS Code
    render_section_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("VS Code").monospace().color(ACCENT_CYAN));
            ui.label(RichText::new("Cmd+Option+K").small().color(TEXT_MUTED));
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("Sends current selection + file path to kyco")
                .small()
                .color(TEXT_DIM),
        );
        ui.add_space(8.0);

        if ui
            .button(RichText::new("📦 Install VS Code Extension").color(ACCENT_GREEN))
            .clicked()
        {
            install_vscode_extension(state);
        }
    });

    ui.add_space(12.0);

    // JetBrains
    render_section_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("JetBrains IDEs")
                    .monospace()
                    .color(ACCENT_CYAN),
            );
            ui.label(RichText::new("Ctrl+Alt+Y").small().color(TEXT_MUTED));
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("IntelliJ, WebStorm, PyCharm, etc.")
                .small()
                .color(TEXT_DIM),
        );
        ui.add_space(8.0);

        if ui
            .button(RichText::new("📦 Install JetBrains Plugin").color(ACCENT_GREEN))
            .clicked()
        {
            install_jetbrains_plugin(state);
        }
    });

    // Status message
    if let Some((msg, is_error)) = &state.extension_status {
        ui.add_space(16.0);
        let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
        egui::Frame::NONE
            .fill(if *is_error {
                Color32::from_rgb(40, 20, 20)
            } else {
                Color32::from_rgb(20, 40, 20)
            })
            .corner_radius(4.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(RichText::new(msg).color(color));
            });
    }

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);
}

/// Install VS Code extension
fn install_vscode_extension(state: &mut SettingsState<'_>) {
    let result = crate::gui::install::install_vscode_extension(state.work_dir);
    *state.extension_status = Some((result.message, result.is_error));
}

/// Install JetBrains plugin
fn install_jetbrains_plugin(state: &mut SettingsState<'_>) {
    let result = crate::gui::install::install_jetbrains_plugin(state.work_dir);
    *state.extension_status = Some((result.message, result.is_error));
}
