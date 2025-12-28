//! State definitions rendering for chain editor

use eframe::egui::{self, RichText};

use super::state::StateDefinitionEdit;
use crate::gui::theme::{ACCENT_RED, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

/// Color for state definitions
const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(200, 150, 255);

/// Render the state definitions section
pub fn render_state_definitions(ui: &mut egui::Ui, states: &mut Vec<StateDefinitionEdit>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("State Definitions:").color(TEXT_PRIMARY));
        ui.label(
            RichText::new("(detected via pattern matching in output)")
                .small()
                .color(TEXT_DIM),
        );
    });
    ui.add_space(8.0);

    let mut state_to_remove: Option<usize> = None;

    for (i, state_def) in states.iter_mut().enumerate() {
        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .corner_radius(4.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("ID:").color(TEXT_MUTED));
                    ui.add(
                        egui::TextEdit::singleline(&mut state_def.id)
                            .font(egui::TextStyle::Monospace)
                            .text_color(ACCENT_PURPLE)
                            .hint_text("issues_found")
                            .desired_width(150.0),
                    );

                    ui.add_space(16.0);
                    ui.checkbox(&mut state_def.is_regex, "");
                    ui.label(RichText::new("Regex").small().color(TEXT_MUTED));

                    ui.checkbox(&mut state_def.case_insensitive, "");
                    ui.label(RichText::new("Case-insensitive").small().color(TEXT_MUTED));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("✕").color(ACCENT_RED)).clicked() {
                            state_to_remove = Some(i);
                        }
                    });
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Description:").small().color(TEXT_MUTED));
                    ui.add(
                        egui::TextEdit::singleline(&mut state_def.description)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .hint_text("Issues were found in the code")
                            .desired_width(300.0),
                    );
                });

                ui.add_space(4.0);
                ui.label(
                    RichText::new("Patterns (one per line):")
                        .small()
                        .color(TEXT_MUTED),
                );
                ui.add(
                    egui::TextEdit::multiline(&mut state_def.patterns)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("issues found\nproblems detected\nbugs identified")
                        .desired_width(ui.available_width() - 20.0)
                        .desired_rows(3),
                );
            });
        ui.add_space(4.0);
    }

    if let Some(i) = state_to_remove {
        states.remove(i);
    }

    ui.add_space(8.0);
    if ui
        .button(RichText::new("+ Add State Definition").color(ACCENT_PURPLE))
        .clicked()
    {
        states.push(StateDefinitionEdit {
            case_insensitive: true,
            ..Default::default()
        });
    }
    ui.add_space(20.0);
}
