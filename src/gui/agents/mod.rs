//! Agent settings component for the GUI
//!
//! Renders the agents configuration view where users can:
//! - List all available agents
//! - Create new agents
//! - Edit existing agents (binary, cli_type, mode args, etc.)
//! - Delete agents

mod editor;
mod list;
mod persistence;
mod state;

pub use state::AgentEditorState;

use eframe::egui::{self, RichText};

use super::app::{ViewMode, BG_PRIMARY, TEXT_DIM, TEXT_PRIMARY};

/// Render the agents configuration view
pub fn render_agents(ctx: &egui::Context, state: &mut AgentEditorState<'_>) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(BG_PRIMARY).inner_margin(16.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("AGENTS")
                            .monospace()
                            .size(18.0)
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("x Close").color(TEXT_DIM))
                            .clicked()
                        {
                            *state.view_mode = ViewMode::JobList;
                        }
                        if state.selected_agent.is_some() {
                            ui.add_space(8.0);
                            if ui
                                .button(RichText::new("<- Back").color(TEXT_DIM))
                                .clicked()
                            {
                                *state.selected_agent = None;
                                *state.agent_edit_status = None;
                            }
                        }
                    });
                });
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(16.0);

                if let Some(agent_name) = state.selected_agent.clone() {
                    editor::render_agent_editor(ui, state, &agent_name);
                } else {
                    list::render_agents_list(ui, state);
                }
            });
        });
}
