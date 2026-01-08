//! Skill editor form rendering
//!
//! Displays and edits the SKILL.md content directly, along with
//! folder structure information (scripts/, references/, assets/).

use eframe::egui::{self, RichText, ScrollArea};

use super::persistence::{delete_skill_file, save_skill_file};
use super::state::{SkillEditorState, SkillFolderInfo};
use crate::gui::animations::animated_button;
use crate::gui::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_RED, ACCENT_YELLOW, BG_SECONDARY, TEXT_DIM,
    TEXT_MUTED, TEXT_PRIMARY,
};

/// Render the skill editor
pub fn render_skill_editor(ui: &mut egui::Ui, state: &mut SkillEditorState<'_>, skill_name: &str) {
    let is_new = skill_name == "__new__";
    let title = if is_new {
        "Create New Skill".to_string()
    } else {
        format!("Edit Skill: {}", skill_name)
    };

    ui.label(RichText::new(&title).monospace().color(TEXT_PRIMARY));
    ui.add_space(8.0);

    // Show source path for existing skills
    if !is_new {
        if let Some(skill) = state.config.skill.get(skill_name) {
            if let Some(ref source) = skill.source_path {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📄").color(ACCENT_PURPLE));
                    ui.label(
                        RichText::new(source.display().to_string())
                            .small()
                            .monospace()
                            .color(ACCENT_PURPLE),
                    );
                });
            }
        }
    }

    ui.add_space(8.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // For new skills, show name input
            if is_new {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Skill Name:").color(TEXT_MUTED));
                    ui.add(
                        egui::TextEdit::singleline(state.skill_edit_name)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .hint_text("my-skill (lowercase, hyphens)")
                            .desired_width(200.0),
                    );
                });
                ui.add_space(8.0);
                ui.label(
                    RichText::new("The skill will be created in .claude/skills/<name>/SKILL.md")
                        .small()
                        .color(TEXT_DIM),
                );
                ui.add_space(16.0);
            }

            // Folder structure info (for existing skills)
            if state.skill_folder_info.has_resources() {
                render_folder_structure(ui, &state.skill_folder_info);
                ui.add_space(16.0);
            }

            // SKILL.md content editor
            ui.label(RichText::new("SKILL.md Content:").color(TEXT_MUTED));
            ui.add_space(4.0);
            ui.label(
                RichText::new("YAML frontmatter + Markdown instructions")
                    .small()
                    .color(TEXT_DIM),
            );
            ui.add_space(8.0);

            egui::Frame::NONE
                .fill(BG_SECONDARY)
                .corner_radius(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(state.skill_edit_content)
                            .font(egui::TextStyle::Monospace)
                            .text_color(TEXT_PRIMARY)
                            .desired_width(f32::INFINITY)
                            .desired_rows(25),
                    );
                });

            ui.add_space(16.0);

            // Status message
            if let Some((msg, is_error)) = &state.skill_edit_status {
                let color = if *is_error { ACCENT_RED } else { ACCENT_GREEN };
                ui.label(RichText::new(msg.as_str()).color(color));
                ui.add_space(8.0);
            }

            // Action buttons
            ui.horizontal(|ui| {
                if animated_button(ui, "Save", ACCENT_GREEN, "skill_save_btn").clicked() {
                    save_skill_file(state, is_new);
                }
                if !is_new {
                    ui.add_space(16.0);
                    if animated_button(ui, "Delete", ACCENT_RED, "skill_delete_btn").clicked() {
                        delete_skill_file(state);
                    }
                }
            });

            ui.add_space(16.0);

            // Help section
            render_help_section(ui);
        });
}

/// Render folder structure information
fn render_folder_structure(ui: &mut egui::Ui, info: &SkillFolderInfo) {
    ui.label(RichText::new("Folder Structure:").color(TEXT_MUTED));
    ui.add_space(4.0);

    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            if info.has_scripts {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📜 scripts/").color(ACCENT_YELLOW));
                    ui.label(
                        RichText::new(format!("({} files)", info.scripts.len()))
                            .small()
                            .color(TEXT_DIM),
                    );
                });
                for script in &info.scripts {
                    ui.label(
                        RichText::new(format!("    • {}", script))
                            .small()
                            .monospace()
                            .color(TEXT_MUTED),
                    );
                }
            }

            if info.has_references {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📚 references/").color(ACCENT_CYAN));
                    ui.label(
                        RichText::new(format!("({} files)", info.references.len()))
                            .small()
                            .color(TEXT_DIM),
                    );
                });
                for reference in &info.references {
                    ui.label(
                        RichText::new(format!("    • {}", reference))
                            .small()
                            .monospace()
                            .color(TEXT_MUTED),
                    );
                }
            }

            if info.has_assets {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🖼 assets/").color(ACCENT_PURPLE));
                    ui.label(
                        RichText::new(format!("({} files)", info.assets.len()))
                            .small()
                            .color(TEXT_DIM),
                    );
                });
                for asset in &info.assets {
                    ui.label(
                        RichText::new(format!("    • {}", asset))
                            .small()
                            .monospace()
                            .color(TEXT_MUTED),
                    );
                }
            }
        });
}

/// Render help section
fn render_help_section(ui: &mut egui::Ui) {
    egui::CollapsingHeader::new(RichText::new("SKILL.md Format Help").small().color(TEXT_MUTED))
        .default_open(false)
        .show(ui, |ui| {
            ui.label(
                RichText::new(
                    r#"YAML Frontmatter:
---
name: skill-name
description: What this skill does
x-kyco:
  aliases: ["s", "sk"]
  session_mode: oneshot | session
  max_turns: 0
  disallowed_tools: ["Bash"]
  output_states: ["done", "needs_review"]
---

Placeholders in Instructions:
• {target} - the code/file target
• {description} - user's description
• {file} - source file path
• {line} - line number
• {ide_context} - IDE context

## System Context
Content after this header becomes the system prompt.

Folder Structure:
skill-name/
├── SKILL.md          # Required
├── scripts/          # Optional: executable scripts
├── references/       # Optional: reference docs
└── assets/           # Optional: images, files"#,
                )
                .small()
                .monospace()
                .color(TEXT_DIM),
            );
        });
}
