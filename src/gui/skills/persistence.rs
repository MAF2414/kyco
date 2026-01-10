//! Skill file persistence (save/delete/load operations)
//!
//! Skills are stored as SKILL.md files in:
//! - `.claude/skills/<skill-name>/SKILL.md`
//! - `.codex/skills/<skill-name>/SKILL.md`
//! - `~/.kyco/skills/<skill-name>/SKILL.md`

use super::state::{SkillEditorState, SkillFolderInfo};
use crate::config::{parse_skill_content, validate_skill};

/// Load skill data for editing
pub fn load_skill_for_editing(state: &mut SkillEditorState<'_>, name: &str) {
    if let Some(skill) = state.config.skill.get(name) {
        // Load the raw SKILL.md content from file
        if let Some(ref source_path) = skill.source_path {
            match std::fs::read_to_string(source_path) {
                Ok(content) => {
                    *state.skill_edit_content = content;
                }
                Err(e) => {
                    // If we can't read the file, generate from the parsed skill
                    *state.skill_edit_content = skill.to_skill_md();
                    *state.skill_edit_status = Some((
                        format!("Warning: Could not read file, showing generated content: {}", e),
                        true,
                    ));
                }
            }
        } else {
            // No source path - generate from skill config
            *state.skill_edit_content = skill.to_skill_md();
        }

        // Load folder info
        *state.skill_folder_info = SkillFolderInfo::from_skill(skill);
        *state.skill_edit_status = None;
    }
}

/// Save skill to filesystem
///
/// For new skills, creates `.claude/skills/<name>/SKILL.md`
/// For existing skills, saves to the original source path
pub fn save_skill_file(state: &mut SkillEditorState<'_>, is_new: bool) {
    let content = state.skill_edit_content.trim();

    if content.is_empty() {
        *state.skill_edit_status = Some(("SKILL.md content cannot be empty".to_string(), true));
        return;
    }

    // Parse the content to validate and extract the name
    let skill = match parse_skill_content(content) {
        Ok(skill) => skill,
        Err(e) => {
            *state.skill_edit_status = Some((format!("Invalid SKILL.md format: {}", e), true));
            return;
        }
    };

    // Validate skill per agentskills.io specification
    if let Err(e) = validate_skill(&skill) {
        *state.skill_edit_status = Some((e.to_string(), true));
        return;
    }

    if is_new {
        // Create new skill in BOTH .claude/skills/ AND .codex/skills/
        let claude_dir = state.work_dir.join(".claude/skills").join(&skill.name);
        let codex_dir = state.work_dir.join(".codex/skills").join(&skill.name);

        let mut created_paths = Vec::new();
        let mut errors = Vec::new();

        for skill_dir in [&claude_dir, &codex_dir] {
            if let Err(e) = std::fs::create_dir_all(skill_dir) {
                errors.push(format!("{}: {}", skill_dir.display(), e));
                continue;
            }

            let skill_path = skill_dir.join("SKILL.md");
            if let Err(e) = std::fs::write(&skill_path, content) {
                errors.push(format!("{}: {}", skill_path.display(), e));
            } else {
                created_paths.push(skill_path.display().to_string());
            }
        }

        if created_paths.is_empty() {
            *state.skill_edit_status = Some((
                format!("Failed to create skill: {}", errors.join(", ")),
                true,
            ));
            return;
        }

        let status_msg = if errors.is_empty() {
            format!("Skill '{}' created in:\n  • {}", skill.name, created_paths.join("\n  • "))
        } else {
            format!(
                "Skill '{}' partially created:\n  • {}\nErrors: {}",
                skill.name,
                created_paths.join("\n  • "),
                errors.join(", ")
            )
        };

        *state.skill_edit_status = Some((status_msg, !errors.is_empty()));
        *state.selected_skill = Some(skill.name);
    } else {
        // Save to existing source path
        let skill_name = state.selected_skill.as_ref().unwrap();
        if let Some(existing_skill) = state.config.skill.get(skill_name) {
            if let Some(ref source_path) = existing_skill.source_path {
                if let Err(e) = std::fs::write(source_path, content) {
                    *state.skill_edit_status = Some((format!("Failed to save: {}", e), true));
                    return;
                }
                *state.skill_edit_status = Some(("Skill saved!".to_string(), false));
            } else {
                *state.skill_edit_status = Some(("No source path for this skill".to_string(), true));
            }
        }
    }
}

/// Delete skill from filesystem
pub fn delete_skill_file(state: &mut SkillEditorState<'_>) {
    let Some(ref skill_name) = state.selected_skill.clone() else {
        return;
    };

    if skill_name == "__new__" {
        *state.selected_skill = None;
        return;
    }

    if let Some(skill) = state.config.skill.get(skill_name) {
        if let Some(ref source_path) = skill.source_path {
            // Get the skill directory (parent of SKILL.md)
            if let Some(skill_dir) = source_path.parent() {
                // Check if it's a directory-based skill
                if skill_dir.file_name().is_some_and(|n| n == skill_name.as_str()) {
                    // Delete the entire directory
                    if let Err(e) = std::fs::remove_dir_all(skill_dir) {
                        *state.skill_edit_status = Some((format!("Failed to delete: {}", e), true));
                        return;
                    }
                } else {
                    // Legacy single-file skill
                    if let Err(e) = std::fs::remove_file(source_path) {
                        *state.skill_edit_status = Some((format!("Failed to delete: {}", e), true));
                        return;
                    }
                }
                *state.skill_edit_status = Some(("Skill deleted!".to_string(), false));
                *state.selected_skill = None;
            }
        }
    }
}

