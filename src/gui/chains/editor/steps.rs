//! Steps rendering for chain editor

use eframe::egui::{self, RichText};

use super::state::ChainStepEdit;
use crate::gui::theme::{ACCENT_CYAN, ACCENT_RED, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

/// Render the steps section
pub fn render_steps(
    ui: &mut egui::Ui,
    steps: &mut Vec<ChainStepEdit>,
    available_modes: &[String],
    available_state_ids: &[String],
) {
    ui.label(RichText::new("Steps:").color(TEXT_PRIMARY));
    ui.add_space(8.0);

    let mut step_to_remove: Option<usize> = None;
    let mut step_to_move_up: Option<usize> = None;
    let mut step_to_move_down: Option<usize> = None;
    let step_count = steps.len();

    for (i, step) in steps.iter_mut().enumerate() {
        egui::Frame::NONE
            .fill(BG_SECONDARY)
            .corner_radius(4.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Step {}:", i + 1)).color(ACCENT_CYAN));

                    egui::ComboBox::from_id_salt(format!("mode_{}", i))
                        .selected_text(if step.mode.is_empty() {
                            "(select mode)"
                        } else {
                            &step.mode
                        })
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for mode in available_modes {
                                ui.selectable_value(&mut step.mode, mode.clone(), mode);
                            }
                        });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("✕").color(ACCENT_RED)).clicked() {
                            step_to_remove = Some(i);
                        }
                        if i > 0 && ui.button(RichText::new("↑").color(TEXT_DIM)).clicked() {
                            step_to_move_up = Some(i);
                        }
                        if i + 1 < step_count
                            && ui.button(RichText::new("↓").color(TEXT_DIM)).clicked()
                        {
                            step_to_move_down = Some(i);
                        }
                    });
                });

                ui.add_space(8.0);

                if i == 0 {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("ℹ").color(TEXT_DIM)).on_hover_text(
                            "The first step always runs - trigger conditions only apply to subsequent steps.",
                        );
                        ui.label(
                            RichText::new("First step always runs")
                                .small()
                                .italics()
                                .color(TEXT_DIM),
                        );
                    });
                } else {
                    let state_hint = if available_state_ids.is_empty() {
                        "Define states above first".to_string()
                    } else {
                        available_state_ids.join(", ")
                    };

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Trigger on:").small().color(TEXT_MUTED));
                        ui.add(
                            egui::TextEdit::singleline(&mut step.trigger_on)
                                .font(egui::TextStyle::Monospace)
                                .text_color(TEXT_PRIMARY)
                                .hint_text(&state_hint)
                                .desired_width(180.0),
                        );
                        ui.label(RichText::new("Skip on:").small().color(TEXT_MUTED));
                        ui.add(
                            egui::TextEdit::singleline(&mut step.skip_on)
                                .font(egui::TextStyle::Monospace)
                                .text_color(TEXT_PRIMARY)
                                .hint_text(&state_hint)
                                .desired_width(180.0),
                        );
                    });
                }

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Agent:").small().color(TEXT_MUTED));
                    ui.add(
                        egui::TextEdit::singleline(&mut step.agent)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .hint_text("(mode default)")
                            .desired_width(100.0),
                    );
                    ui.add_space(16.0);
                    ui.label(RichText::new("Loop to:").small().color(TEXT_MUTED));
                    ui.add(
                        egui::TextEdit::singleline(&mut step.loop_to)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .hint_text("(mode name)")
                            .desired_width(100.0),
                    )
                    .on_hover_text(
                        "If triggered, restart chain from this step's mode.\n\
                         Useful for review → fix loops. Limited by max_loops.",
                    );
                });

                ui.add_space(4.0);

                egui::CollapsingHeader::new(
                    RichText::new("Inject Context").small().color(TEXT_MUTED),
                )
                .id_salt(format!("inject_ctx_{}", i))
                .default_open(!step.inject_context.is_empty())
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut step.inject_context)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .hint_text("Additional context to inject into the prompt...")
                            .desired_width(ui.available_width() - 20.0)
                            .desired_rows(2),
                    );
                });
            });
        ui.add_space(4.0);
    }

    // Only process one operation per frame to prevent index invalidation
    if let Some(i) = step_to_remove {
        steps.remove(i);
    } else if let Some(i) = step_to_move_up {
        // Bounds check: ensure both indices are valid
        if i > 0 && i < steps.len() {
            steps.swap(i, i - 1);
        }
    } else if let Some(i) = step_to_move_down {
        // Bounds check: ensure both indices are valid
        if i + 1 < steps.len() {
            steps.swap(i, i + 1);
        }
    }

    ui.add_space(8.0);
    if ui
        .button(RichText::new("+ Add Step").color(ACCENT_CYAN))
        .clicked()
    {
        steps.push(ChainStepEdit::default());
    }
    ui.add_space(20.0);
}
