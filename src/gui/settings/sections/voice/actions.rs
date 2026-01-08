//! Voice actions (wakewords) settings section

use eframe::egui::{self, Color32, RichText};

use crate::gui::settings::state::SettingsState;
use crate::gui::theme::{ACCENT_CYAN, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

/// Render Voice Actions (Wakewords → Skills) section
pub fn render_voice_actions(ui: &mut egui::Ui, state: &SettingsState<'_>) {
    ui.label(
        RichText::new("Voice Actions (Wakewords → Skills)")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);

    egui::Frame::NONE
        .fill(Color32::from_rgb(30, 30, 35))
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new(
                    "Speak a wakeword to trigger the corresponding skill (loaded from config):",
                )
                .color(TEXT_MUTED),
            );

            // Show global prefix if set
            if let Some(ref prefix) = state.voice_action_registry.global_prefix {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("Global prefix: \"{}\"", prefix))
                        .small()
                        .color(ACCENT_CYAN),
                );
            }

            ui.add_space(8.0);

            // Display actions from registry
            if state.voice_action_registry.actions.is_empty() {
                ui.label(
                    RichText::new("No skills configured. Add skills to .claude/skills/ or .codex/skills/")
                        .small()
                        .color(TEXT_DIM),
                );
            } else {
                render_actions_grid(ui, state);
            }
        });
}

/// Render the grid of voice actions
fn render_actions_grid(ui: &mut egui::Ui, state: &SettingsState<'_>) {
    egui::Grid::new("voice_actions_grid")
        .num_columns(3)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(RichText::new("Wakeword").color(TEXT_MUTED).small());
            ui.label(RichText::new("Skill").color(TEXT_MUTED).small());
            ui.label(RichText::new("Aliases").color(TEXT_MUTED).small());
            ui.end_row();

            for action in &state.voice_action_registry.actions {
                let primary = action
                    .wakewords
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or(&action.mode);
                ui.label(RichText::new(primary).monospace().color(ACCENT_CYAN));
                ui.label(
                    RichText::new(format!("→ {}", action.mode)).color(TEXT_PRIMARY),
                );

                let aliases: Vec<&str> = action
                    .wakewords
                    .iter()
                    .skip(1)
                    .map(|s| s.as_str())
                    .chain(action.aliases.iter().map(|s| s.as_str()))
                    .collect();
                let aliases_str = if aliases.is_empty() {
                    "-".to_string()
                } else {
                    aliases.join(", ")
                };
                ui.label(RichText::new(aliases_str).small().color(TEXT_DIM));
                ui.end_row();
            }
        });

    ui.add_space(8.0);
    if let Some(first) = state.voice_action_registry.actions.first() {
        ui.label(
            RichText::new(format!("Example: \"{}\" this function", first.mode))
                .small()
                .italics()
                .color(TEXT_DIM),
        );
    }
}

/// Render VAD (Voice Activity Detection) settings - Coming Soon
pub fn render_vad_settings(ui: &mut egui::Ui, _state: &mut SettingsState<'_>) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("VAD (Voice Activity Detection)")
                .monospace()
                .color(TEXT_MUTED),
        );
        ui.label(
            RichText::new("Coming Soon")
                .small()
                .color(Color32::from_rgb(255, 180, 0)),
        );
    });
    ui.add_space(8.0);

    egui::Frame::NONE
        .fill(Color32::from_rgb(30, 30, 35))
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new("VAD will detect speech to efficiently trigger transcription in continuous mode. This feature is currently under development.")
                    .small()
                    .color(TEXT_DIM),
            );
        });
}
