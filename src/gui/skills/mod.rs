//! Skills configuration component for the GUI
//!
//! Renders the skills configuration view where users can:
//! - List all available skills (from filesystem)
//! - Create new skills
//! - Edit existing skills (SKILL.md content)
//! - Delete skills
//! - Browse and install from the community registry (~50,000 skills)
//!
//! Skills are loaded from:
//! - `.claude/skills/`
//! - `.codex/skills/`
//! - `~/.kyco/skills/`

mod editor;
mod list;
mod persistence;
mod registry;
mod state;

pub use persistence::load_skill_for_editing;
pub use state::{SkillEditorState, SkillFolderInfo, SkillInstallLocation, SkillsTab};

use eframe::egui::{self, RichText};

use super::animations::animated_button;
use super::app::ViewMode;
use super::theme::{BG_PRIMARY, TEXT_DIM, TEXT_PRIMARY, ACCENT_CYAN, TEXT_MUTED};

/// Render the skills configuration view
pub fn render_skills(ctx: &egui::Context, state: &mut SkillEditorState<'_>) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(BG_PRIMARY).inner_margin(16.0))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("SKILLS")
                            .monospace()
                            .size(18.0)
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if animated_button(ui, "Close", TEXT_DIM, "skills_close_btn").clicked() {
                            *state.view_mode = ViewMode::JobList;
                        }
                        if state.selected_skill.is_some() {
                            ui.add_space(8.0);
                            if animated_button(ui, "<- Back", TEXT_DIM, "skills_back_btn").clicked()
                            {
                                *state.selected_skill = None;
                                *state.skill_edit_status = None;
                            }
                        }
                    });
                });
                ui.add_space(8.0);

                // Tab selector (only when not editing a skill)
                if state.selected_skill.is_none() {
                    ui.horizontal(|ui| {
                        let local_selected = *state.skills_tab == SkillsTab::Local;
                        let registry_selected = *state.skills_tab == SkillsTab::Registry;

                        if ui
                            .selectable_label(local_selected, RichText::new("📁 Local Skills").color(
                                if local_selected { ACCENT_CYAN } else { TEXT_MUTED }
                            ))
                            .clicked()
                        {
                            *state.skills_tab = SkillsTab::Local;
                        }

                        ui.add_space(16.0);

                        if ui
                            .selectable_label(registry_selected, RichText::new("🌐 Registry (~50k)").color(
                                if registry_selected { ACCENT_CYAN } else { TEXT_MUTED }
                            ))
                            .clicked()
                        {
                            *state.skills_tab = SkillsTab::Registry;
                        }
                    });
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(16.0);

                if let Some(skill_name) = state.selected_skill.clone() {
                    editor::render_skill_editor(ui, state, &skill_name);
                } else {
                    match *state.skills_tab {
                        SkillsTab::Local => list::render_skills_list(ui, state),
                        SkillsTab::Registry => registry::render_registry_browser(ui, state),
                    }
                }
            });
        });
}
