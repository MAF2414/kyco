//! Chain editor form rendering

use eframe::egui::{self, RichText, ScrollArea};

use super::persistence::save_chain_to_config;
use super::state::{ChainEditorState, ChainStepEdit, PendingConfirmation, StateDefinitionEdit};
use crate::gui::animations::animated_button;
use crate::gui::app::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, ACCENT_YELLOW, BG_SECONDARY, TEXT_DIM, TEXT_MUTED,
    TEXT_PRIMARY,
};

/// Color for state definitions
const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(200, 150, 255);

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

    // Get available modes for dropdown (sorted alphabetically for consistent UX)
    let mut available_modes: Vec<String> = state.config.mode.keys().cloned().collect();
    available_modes.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

    // Get available state IDs for trigger_on/skip_on hints
    let available_state_ids: Vec<String> = state
        .chain_edit_states
        .iter()
        .map(|s| s.id.clone())
        .filter(|id| !id.is_empty())
        .collect();

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ═══════════════════════════════════════════════════════════════
            // BASIC INFO
            // ═══════════════════════════════════════════════════════════════

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

            // Toggles row
            ui.horizontal(|ui| {
                ui.checkbox(state.chain_edit_stop_on_failure, "");
                ui.label(RichText::new("Stop on failure").color(TEXT_MUTED));
                ui.add_space(24.0);
                ui.checkbox(state.chain_edit_pass_full_response, "");
                ui.label(RichText::new("Pass full response").color(TEXT_MUTED))
                    .on_hover_text("When enabled, the complete output is passed to the next step.\nWhen disabled, only the summary is passed.");
            });
            ui.add_space(16.0);

            // ═══════════════════════════════════════════════════════════════
            // VISUAL FLOW PREVIEW
            // ═══════════════════════════════════════════════════════════════

            if !state.chain_edit_steps.is_empty() {
                ui.label(RichText::new("Flow Preview:").color(TEXT_PRIMARY));
                ui.add_space(4.0);

                egui::Frame::NONE
                    .fill(BG_SECONDARY)
                    .corner_radius(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for (i, step) in state.chain_edit_steps.iter().enumerate() {
                                let mode_name = if step.mode.is_empty() { "?" } else { &step.mode };
                                ui.label(RichText::new(mode_name).monospace().color(ACCENT_CYAN));

                                if i < state.chain_edit_steps.len() - 1 {
                                    let next_step = &state.chain_edit_steps[i + 1];
                                    let arrow_label = if let Some(triggers) = next_step.trigger_on.split(',')
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

            // ═══════════════════════════════════════════════════════════════
            // STATE DEFINITIONS
            // ═══════════════════════════════════════════════════════════════

            ui.horizontal(|ui| {
                ui.label(RichText::new("State Definitions:").color(TEXT_PRIMARY));
                ui.label(RichText::new("(detected via pattern matching in output)").small().color(TEXT_DIM));
            });
            ui.add_space(8.0);

            let mut state_to_remove: Option<usize> = None;

            for (i, state_def) in state.chain_edit_states.iter_mut().enumerate() {
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
                        ui.label(RichText::new("Patterns (one per line):").small().color(TEXT_MUTED));
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
                state.chain_edit_states.remove(i);
            }

            // Add state button
            ui.add_space(8.0);
            if ui
                .button(RichText::new("+ Add State Definition").color(ACCENT_PURPLE))
                .clicked()
            {
                state.chain_edit_states.push(StateDefinitionEdit {
                    case_insensitive: true,
                    ..Default::default()
                });
            }
            ui.add_space(20.0);

            // ═══════════════════════════════════════════════════════════════
            // STEPS
            // ═══════════════════════════════════════════════════════════════

            ui.label(RichText::new("Steps:").color(TEXT_PRIMARY));
            ui.add_space(8.0);

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
                                .selected_text(if step.mode.is_empty() { "(select mode)" } else { &step.mode })
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

                        // Trigger/Skip conditions
                        if i == 0 {
                            // First step: show tooltip explaining why there are no triggers
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("ℹ").color(TEXT_DIM))
                                    .on_hover_text("The first step always runs - trigger conditions only apply to subsequent steps.");
                                ui.label(RichText::new("First step always runs").small().italics().color(TEXT_DIM));
                            });
                        } else {
                            // Build hint text from available states
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

                        // Agent override and inject_context
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Agent:").small().color(TEXT_MUTED));
                            ui.add(
                                egui::TextEdit::singleline(&mut step.agent)
                                    .font(egui::TextStyle::Monospace)
                                    .text_color(TEXT_PRIMARY)
                                    .hint_text("(mode default)")
                                    .desired_width(100.0),
                            );
                        });

                        ui.add_space(4.0);

                        // Inject context - collapsible to save space
                        egui::CollapsingHeader::new(RichText::new("Inject Context").small().color(TEXT_MUTED))
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
            ui.add_space(20.0);

            // ═══════════════════════════════════════════════════════════════
            // STATUS & ACTIONS
            // ═══════════════════════════════════════════════════════════════

            // Status message
            if let Some((msg, is_error)) = &state.chain_edit_status {
                let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
                ui.label(RichText::new(msg.as_str()).color(color));
                ui.add_space(8.0);
            }

            // Action buttons
            ui.horizontal(|ui| {
                if animated_button(ui, "Save to Config", ACCENT_GREEN, "chain_save_btn").clicked() {
                    save_chain_to_config(state, is_new);
                }
                if !is_new {
                    ui.add_space(16.0);
                    if animated_button(ui, "Delete", ACCENT_RED, "chain_delete_btn").clicked() {
                        // Show confirmation dialog instead of deleting immediately
                        *state.pending_confirmation = PendingConfirmation::DeleteChain(chain_name.to_string());
                    }
                }
            });
        });
}
