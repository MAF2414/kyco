//! Flow preview rendering for chain editor

use eframe::egui::{self, RichText};

use super::state::ChainStepEdit;
use crate::gui::theme::{ACCENT_CYAN, BG_SECONDARY, TEXT_DIM, TEXT_PRIMARY};

/// Render the flow preview showing step connections
pub fn render_flow_preview(ui: &mut egui::Ui, steps: &[ChainStepEdit]) {
    if steps.is_empty() {
        return;
    }

    ui.label(RichText::new("Flow Preview:").color(TEXT_PRIMARY));
    ui.add_space(4.0);

    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, step) in steps.iter().enumerate() {
                    let mode_name = if step.mode.is_empty() {
                        "?"
                    } else {
                        &step.mode
                    };
                    ui.label(RichText::new(mode_name).monospace().color(ACCENT_CYAN));

                    if i + 1 < steps.len() {
                        let next_step = &steps[i + 1];
                        let arrow_label = if let Some(triggers) = next_step
                            .trigger_on
                            .split(',')
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .next()
                        {
                            format!("─[{}]→", triggers)
                        } else {
                            "───→".to_string()
                        };
                        ui.label(RichText::new(arrow_label).monospace().color(TEXT_DIM));
                    }
                }
            });
        });
    ui.add_space(16.0);
}
