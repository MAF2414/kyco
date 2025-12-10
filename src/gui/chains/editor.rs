//! Chain editor form rendering

use eframe::egui::{self, RichText, ScrollArea};

use super::persistence::{delete_chain_from_config, save_chain_to_config};
use super::state::{ChainEditorState, ChainStepEdit};
use crate::gui::animations::animated_button;
use crate::gui::app::{ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, ACCENT_YELLOW, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

/// Render the chain editor form
pub fn render_chain_editor(ui: &mut egui::Ui, state: &mut ChainEditorState<'_>, chain_name: &str) {
    let is_new = chain_name == "__new__";
    let title = if is_new {
        "Create New Chain".to_string()
    } else {
        format!("Edit Chain: {}", chain_name)
    };

    ui.label(RichText::new(&title).monospace().color(TEXT_PRIMARY));
    ui.add_space(16.0);

    // Get available modes for dropdown
    let available_modes: Vec<String> = state.config.mode.keys().cloned().collect();

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Name (only editable for new chains)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Name:").color(TEXT_MUTED));
                if is_new {
                    ui.add(
                        egui::TextEdit::singleline(state.chain_edit_name)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .hint_text("review+fix")
                            .desired_width(200.0),
                    );
                } else {
                    ui.label(
                        RichText::new(&*state.chain_edit_name)
                            .monospace()
                            .color(ACCENT_YELLOW),
                    );
                }
            });
            ui.add_space(8.0);

            // Description
            ui.horizontal(|ui| {
                ui.label(RichText::new("Description:").color(TEXT_MUTED));
                ui.add(
                    egui::TextEdit::singleline(state.chain_edit_description)
                        .font(egui::TextStyle::Monospace)
                        .text_color(TEXT_PRIMARY)
                        .hint_text("Review code and fix issues found")
                        .desired_width(400.0),
                );
            });
            ui.add_space(8.0);

            // Stop on failure toggle
            ui.horizontal(|ui| {
                ui.checkbox(state.chain_edit_stop_on_failure, "");
                ui.label(
                    RichText::new("Stop chain on failure").color(TEXT_MUTED),
                );
            });
            ui.add_space(16.0);

            // Steps section
            ui.label(RichText::new("Steps:").color(TEXT_PRIMARY));
            ui.add_space(8.0);
            ui.label(
                RichText::new("Each step runs a mode. Use trigger_on/skip_on to control flow based on previous step's state.")
                    .small()
                    .color(TEXT_DIM),
            );
            ui.add_space(12.0);

            // Render each step
            let mut step_to_remove: Option<usize> = None;
            let mut step_to_move_up: Option<usize> = None;
            let mut step_to_move_down: Option<usize> = None;
            let step_count = state.chain_edit_steps.len();

            for (i, step) in state.chain_edit_steps.iter_mut().enumerate() {
                egui::Frame::NONE
                    .fill(BG_SECONDARY)
                    .corner_radius(4.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Step {}:", i + 1)).color(ACCENT_CYAN));

                            // Mode dropdown
                            egui::ComboBox::from_id_salt(format!("mode_{}", i))
                                .selected_text(&step.mode)
                                .width(150.0)
                                .show_ui(ui, |ui| {
                                    for mode in &available_modes {
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
                                if i < step_count - 1 && ui.button(RichText::new("↓").color(TEXT_DIM)).clicked() {
                                    step_to_move_down = Some(i);
                                }
                            });
                        });

                        ui.add_space(8.0);

                        // Trigger/Skip conditions (only for steps after first)
                        if i > 0 {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Trigger on:").small().color(TEXT_MUTED));
                                ui.add(
                                    egui::TextEdit::singleline(&mut step.trigger_on)
                                        .font(egui::TextStyle::Monospace)
                                        .text_color(TEXT_PRIMARY)
                                        .hint_text("issues_found, needs_fix")
                                        .desired_width(200.0),
                                );
                                ui.label(RichText::new("Skip on:").small().color(TEXT_MUTED));
                                ui.add(
                                    egui::TextEdit::singleline(&mut step.skip_on)
                                        .font(egui::TextStyle::Monospace)
                                        .text_color(TEXT_PRIMARY)
                                        .hint_text("clean, approved")
                                        .desired_width(200.0),
                                );
                            });
                        }

                        // Optional agent override
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Agent override:").small().color(TEXT_MUTED));
                            ui.add(
                                egui::TextEdit::singleline(&mut step.agent)
                                    .font(egui::TextStyle::Monospace)
                                    .text_color(TEXT_PRIMARY)
                                    .hint_text("(uses mode default)")
                                    .desired_width(150.0),
                            );
                        });
                    });
                ui.add_space(4.0);
            }

            // Handle step modifications
            if let Some(i) = step_to_remove {
                state.chain_edit_steps.remove(i);
            }
            if let Some(i) = step_to_move_up {
                state.chain_edit_steps.swap(i, i - 1);
            }
            if let Some(i) = step_to_move_down {
                state.chain_edit_steps.swap(i, i + 1);
            }

            // Add step button
            ui.add_space(8.0);
            if ui
                .button(RichText::new("+ Add Step").color(ACCENT_CYAN))
                .clicked()
            {
                state.chain_edit_steps.push(ChainStepEdit::default());
            }
            ui.add_space(16.0);

            // Status message
            if let Some((msg, is_error)) = &state.chain_edit_status {
                let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
                ui.label(RichText::new(msg.as_str()).color(color));
                ui.add_space(8.0);
            }

            // Save button
            ui.horizontal(|ui| {
                if animated_button(ui, "Save to Config", ACCENT_GREEN, "chain_save_btn").clicked() {
                    save_chain_to_config(state, is_new);
                }
                if !is_new {
                    ui.add_space(16.0);
                    if animated_button(ui, "Delete", ACCENT_RED, "chain_delete_btn").clicked() {
                        delete_chain_from_config(state);
                    }
                }
            });
        });
}
