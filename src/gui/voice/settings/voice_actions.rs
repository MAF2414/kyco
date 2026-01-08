//! Voice actions (wakewords) display section

use eframe::egui::{self, Color32, RichText};

use crate::gui::theme::{ACCENT_CYAN, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};
use crate::gui::voice::VoiceActionRegistry;

pub fn render_voice_actions_section(ui: &mut egui::Ui, registry: &VoiceActionRegistry) {
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
                RichText::new("Speak a wakeword to trigger the corresponding skill:")
                    .color(TEXT_MUTED),
            );

            if let Some(ref prefix) = registry.global_prefix {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("Global prefix: \"{}\"", prefix))
                        .small()
                        .color(ACCENT_CYAN),
                );
            }

            ui.add_space(8.0);

            // Display actions from registry (dynamically loaded from modes/chains)
            if registry.actions.is_empty() {
                ui.label(
                    RichText::new("No skills or chains configured. Add skills to enable voice actions.")
                        .small()
                        .color(TEXT_DIM),
                );
            } else {
                render_actions_grid(ui, registry);
            }
        });
}

fn render_actions_grid(ui: &mut egui::Ui, registry: &VoiceActionRegistry) {
    egui::Grid::new("voice_actions_grid")
        .num_columns(3)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(RichText::new("Wakeword").color(TEXT_MUTED).small());
            ui.label(RichText::new("Skill").color(TEXT_MUTED).small());
            ui.label(RichText::new("Aliases").color(TEXT_MUTED).small());
            ui.end_row();

            for action in &registry.actions {
                let primary = action
                    .wakewords
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or(&action.mode);
                ui.label(RichText::new(primary).monospace().color(ACCENT_CYAN));
                ui.label(RichText::new(format!("→ {}", action.mode)).color(TEXT_PRIMARY));

                // Aliases (skip first wakeword, add action aliases)
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
    let example_mode = registry
        .actions
        .first()
        .map(|a| a.mode.as_str())
        .unwrap_or("refactor");
    ui.label(
        RichText::new(format!(
            "Example: Say \"{} this function\" to trigger {} skill",
            example_mode, example_mode
        ))
        .small()
        .italics()
        .color(TEXT_DIM),
    );
}
