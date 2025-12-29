//! Chain editor form rendering

mod flow_preview;
mod state_definitions;
mod steps;

use eframe::egui::{self, RichText, ScrollArea};

use super::persistence::save_chain_to_config;
use super::state::{ChainEditorState, PendingConfirmation};
use crate::gui::animations::animated_button;
use crate::gui::theme::{ACCENT_GREEN, ACCENT_RED, ACCENT_YELLOW, TEXT_MUTED, TEXT_PRIMARY};

use flow_preview::render_flow_preview;
use state_definitions::render_state_definitions;
use steps::render_steps;

// Re-export state types for external use
pub(super) use super::state;

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
            // Name field
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

            // Description field
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

            // Options checkboxes
            ui.horizontal(|ui| {
                ui.checkbox(state.chain_edit_stop_on_failure, "");
                ui.label(RichText::new("Stop on failure").color(TEXT_MUTED));
                ui.add_space(24.0);
                ui.checkbox(state.chain_edit_pass_full_response, "");
                ui.label(RichText::new("Pass full response").color(TEXT_MUTED))
                    .on_hover_text(
                        "When enabled, the complete output is passed to the next step.\n\
                         When disabled, only the summary is passed.",
                    );
            });
            ui.add_space(8.0);

            // Worktree option
            ui.horizontal(|ui| {
                ui.label(RichText::new("Use Worktree:").color(TEXT_MUTED));
                let display_text = match state.chain_edit_use_worktree {
                    None => "global",
                    Some(true) => "always",
                    Some(false) => "never",
                };
                egui::ComboBox::from_id_salt("chain_use_worktree")
                    .selected_text(display_text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(state.chain_edit_use_worktree, None, "global")
                            .on_hover_text("Use the global worktree setting");
                        ui.selectable_value(state.chain_edit_use_worktree, Some(true), "always")
                            .on_hover_text("Always run in a git worktree");
                        ui.selectable_value(state.chain_edit_use_worktree, Some(false), "never")
                            .on_hover_text("Never run in a worktree (even if global is enabled)");
                    });
            });
            ui.add_space(8.0);

            // Max loops option (for loop_to feature)
            ui.horizontal(|ui| {
                ui.label(RichText::new("Max Loops:").color(TEXT_MUTED));
                ui.add(
                    egui::DragValue::new(state.chain_edit_max_loops)
                        .range(1..=10)
                        .speed(0.1),
                )
                .on_hover_text(
                    "Maximum number of times a step with loop_to can restart the chain.\n\
                     Prevents infinite loops when issues keep recurring.",
                );
            });
            ui.add_space(16.0);

            // Flow preview
            render_flow_preview(ui, state.chain_edit_steps);

            // State definitions
            render_state_definitions(ui, state.chain_edit_states);

            // Steps
            render_steps(
                ui,
                state.chain_edit_steps,
                &available_modes,
                &available_state_ids,
            );

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
                        *state.pending_confirmation =
                            PendingConfirmation::DeleteChain(chain_name.to_string());
                    }
                }
            });
        });
}
