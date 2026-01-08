//! Skill list rendering
//!
//! Displays skills discovered from the filesystem:
//! - `.claude/skills/`
//! - `.codex/skills/`
//! - `~/.kyco/skills/`

use eframe::egui::{self, RichText, ScrollArea};

use super::persistence::load_skill_for_editing;
use super::state::SkillEditorState;
use crate::gui::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_YELLOW, BG_SECONDARY, TEXT_DIM, TEXT_MUTED,
    TEXT_PRIMARY,
};

/// Render the list of available skills
pub fn render_skills_list(ui: &mut egui::Ui, state: &mut SkillEditorState<'_>) {
    ui.label(
        RichText::new("Available Skills")
            .monospace()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new("Skills are SKILL.md files that define agent instructions. Click to edit.")
            .color(TEXT_DIM),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new("📁 .claude/skills/  •  .codex/skills/  •  ~/.kyco/skills/")
            .small()
            .color(TEXT_MUTED),
    );
    ui.add_space(12.0);

    // Collect skills from config.skill (filesystem-discovered)
    let mut skills: Vec<(&String, &crate::config::SkillConfig)> =
        state.config.skill.iter().collect();
    skills.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if skills.is_empty() {
                ui.add_space(20.0);
                ui.label(
                    RichText::new("No skills found.")
                        .color(TEXT_MUTED)
                        .italics(),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Create skills by adding SKILL.md files to:")
                        .small()
                        .color(TEXT_DIM),
                );
                ui.label(
                    RichText::new("  • .claude/skills/<skill-name>/SKILL.md")
                        .small()
                        .monospace()
                        .color(TEXT_DIM),
                );
                ui.label(
                    RichText::new("  • .codex/skills/<skill-name>/SKILL.md")
                        .small()
                        .monospace()
                        .color(TEXT_DIM),
                );
                ui.add_space(20.0);
            }

            for (name, skill) in &skills {
                egui::Frame::NONE
                    .fill(BG_SECONDARY)
                    .corner_radius(4.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        let response = ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                // Skill name
                                ui.label(RichText::new(*name).monospace().color(ACCENT_GREEN));

                                // Aliases
                                if !skill.kyco.aliases.is_empty() {
                                    ui.label(
                                        RichText::new(format!(
                                            "({})",
                                            skill.kyco.aliases.join(", ")
                                        ))
                                        .small()
                                        .color(TEXT_MUTED),
                                    );
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(RichText::new("→").color(TEXT_DIM));

                                        // Show folder structure indicators
                                        if let Some(ref source) = skill.source_path {
                                            if let Some(dir) = source.parent() {
                                                let has_scripts = dir.join("scripts").is_dir();
                                                let has_refs = dir.join("references").is_dir();
                                                let has_assets = dir.join("assets").is_dir();

                                                if has_scripts || has_refs || has_assets {
                                                    ui.add_space(8.0);
                                                    let mut icons = Vec::new();
                                                    if has_scripts {
                                                        icons.push("📜");
                                                    }
                                                    if has_refs {
                                                        icons.push("📚");
                                                    }
                                                    if has_assets {
                                                        icons.push("🖼");
                                                    }
                                                    ui.label(
                                                        RichText::new(icons.join(" "))
                                                            .small()
                                                            .color(ACCENT_YELLOW),
                                                    )
                                                    .on_hover_text(format!(
                                                        "{}{}{}",
                                                        if has_scripts {
                                                            "scripts/ "
                                                        } else {
                                                            ""
                                                        },
                                                        if has_refs { "references/ " } else { "" },
                                                        if has_assets { "assets/" } else { "" }
                                                    ));
                                                }
                                            }
                                        }
                                    },
                                );
                            });

                            // Description
                            if let Some(ref desc) = skill.description {
                                ui.add_space(4.0);
                                ui.label(RichText::new(desc).small().color(TEXT_DIM));
                            }

                            // Source path
                            if let Some(ref source) = skill.source_path {
                                ui.add_space(4.0);
                                let path_str = source.display().to_string();
                                // Shorten path for display
                                let short_path = if path_str.len() > 60 {
                                    format!("...{}", &path_str[path_str.len() - 57..])
                                } else {
                                    path_str
                                };
                                ui.label(
                                    RichText::new(format!("📄 {}", short_path))
                                        .small()
                                        .color(ACCENT_PURPLE),
                                );
                            }
                        });

                        if response.response.interact(egui::Sense::click()).clicked() {
                            *state.selected_skill = Some((*name).clone());
                            load_skill_for_editing(state, name);
                        }
                    });
                ui.add_space(4.0);
            }

            ui.add_space(12.0);
            if ui
                .button(RichText::new("+ Create New Skill").color(ACCENT_CYAN))
                .clicked()
            {
                *state.selected_skill = Some("__new__".to_string());
                state.skill_edit_name.clear();
                *state.skill_edit_content = create_skill_template("");
                *state.skill_edit_status = None;
                *state.skill_folder_info = Default::default();
            }
        });
}

/// Create a new skill template
fn create_skill_template(name: &str) -> String {
    format!(
        r#"---
name: {}
description: Describe what this skill does
x-kyco:
  aliases: []
  session_mode: oneshot
---

# Instructions

Provide instructions for the agent here.

Use placeholders:
- {{target}} - the code target
- {{description}} - user's description
- {{file}} - source file path
- {{ide_context}} - IDE context injection

## System Context

Optional system prompt for the agent.
"#,
        if name.is_empty() { "new-skill" } else { name }
    )
}
