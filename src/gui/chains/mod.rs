//! Chain settings component for the GUI
//!
//! Renders the chains configuration view where users can:
//! - List all available chains
//! - Create new chains
//! - Edit existing chains (steps, triggers, etc.)
//! - Delete chains

mod editor;
mod list;
mod persistence;
pub mod state;

pub use state::{ChainEditorState, ChainStepEdit, PendingConfirmation, StateDefinitionEdit};

use eframe::egui::{self, RichText, Vec2};

use super::animations::animated_button;
use super::app::{ViewMode, ACCENT_RED, ACCENT_YELLOW, BG_PRIMARY, BG_SECONDARY, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

/// Render the chains configuration view
pub fn render_chains(ctx: &egui::Context, state: &mut ChainEditorState<'_>) {
    // Render confirmation dialog if pending
    render_confirmation_dialog(ctx, state);

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(BG_PRIMARY).inner_margin(16.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("CHAINS")
                            .monospace()
                            .size(18.0)
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if animated_button(ui, "Close", TEXT_DIM, "chains_close_btn").clicked() {
                            *state.view_mode = ViewMode::JobList;
                        }
                        if state.selected_chain.is_some() {
                            ui.add_space(8.0);
                            if animated_button(ui, "<- Back", TEXT_DIM, "chains_back_btn").clicked() {
                                *state.selected_chain = None;
                                *state.chain_edit_status = None;
                                *state.pending_confirmation = PendingConfirmation::None;
                            }
                        }
                    });
                });
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(16.0);

                if let Some(chain_name) = state.selected_chain.clone() {
                    editor::render_chain_editor(ui, state, &chain_name);
                } else {
                    list::render_chains_list(ui, state);
                }
            });
        });
}

/// Render confirmation dialog as a modal window
fn render_confirmation_dialog(ctx: &egui::Context, state: &mut ChainEditorState<'_>) {
    let pending = state.pending_confirmation.clone();

    match pending {
        PendingConfirmation::None => {}
        PendingConfirmation::DeleteChain(ref chain_name) => {
            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .fixed_size(Vec2::new(350.0, 150.0))
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("⚠ Delete Chain?")
                            .size(16.0)
                            .color(ACCENT_YELLOW),
                    );
                    ui.add_space(12.0);

                    egui::Frame::NONE
                        .fill(BG_SECONDARY)
                        .corner_radius(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new(format!(
                                "Are you sure you want to delete the chain \"{}\"?",
                                chain_name
                            )).color(TEXT_PRIMARY));
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("This action cannot be undone.")
                                    .small()
                                    .color(TEXT_MUTED),
                            );
                        });

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        if animated_button(ui, "Cancel", TEXT_DIM, "confirm_cancel_btn").clicked() {
                            *state.pending_confirmation = PendingConfirmation::None;
                        }
                        ui.add_space(8.0);
                        if animated_button(ui, "Delete", ACCENT_RED, "confirm_delete_btn").clicked() {
                            persistence::delete_chain_from_config(state);
                            *state.pending_confirmation = PendingConfirmation::None;
                        }
                    });
                });
        }
        PendingConfirmation::DiscardChanges => {
            egui::Window::new("Unsaved Changes")
                .collapsible(false)
                .resizable(false)
                .fixed_size(Vec2::new(350.0, 150.0))
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("⚠ Unsaved Changes")
                            .size(16.0)
                            .color(ACCENT_YELLOW),
                    );
                    ui.add_space(12.0);

                    egui::Frame::NONE
                        .fill(BG_SECONDARY)
                        .corner_radius(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new(
                                "You have unsaved changes. Do you want to discard them?"
                            ).color(TEXT_PRIMARY));
                        });

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        if animated_button(ui, "Keep Editing", TEXT_DIM, "keep_editing_btn").clicked() {
                            *state.pending_confirmation = PendingConfirmation::None;
                        }
                        ui.add_space(8.0);
                        if animated_button(ui, "Discard", ACCENT_RED, "discard_btn").clicked() {
                            *state.selected_chain = None;
                            *state.chain_edit_status = None;
                            *state.pending_confirmation = PendingConfirmation::None;
                        }
                    });
                });
        }
    }
}
