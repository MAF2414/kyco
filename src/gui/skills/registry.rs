//! Registry browser for searching and installing community skills
//!
//! Allows browsing ~50,000 skills from the community registry and installing
//! them directly to the project or global skill directories.

use eframe::egui::{self, RichText, ScrollArea, TextEdit};

use super::state::{SkillEditorState, SkillInstallLocation};
use crate::config::{parse_skill_content, save_skill, save_skill_global, SkillAgentType, SkillRegistry};
use crate::gui::theme::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_PURPLE, ACCENT_YELLOW, BG_SECONDARY, TEXT_DIM, TEXT_MUTED,
};

/// Render the registry browser
pub fn render_registry_browser(ui: &mut egui::Ui, state: &mut SkillEditorState<'_>) {
    // Lazy-load the registry
    if state.registry.is_none() {
        match SkillRegistry::load_embedded() {
            Ok(reg) => {
                let total = reg.total;
                *state.registry = Some(reg);
                tracing::info!("Loaded skill registry with {} skills", total);
            }
            Err(e) => {
                ui.label(
                    RichText::new(format!("Failed to load registry: {}", e))
                        .color(crate::gui::theme::ACCENT_RED),
                );
                return;
            }
        }
    }

    let registry = state.registry.as_ref().unwrap();

    ui.horizontal(|ui| {
        ui.label(RichText::new("🔍").size(16.0));
        let search_response = ui.add(
            TextEdit::singleline(state.registry_search_query)
                .hint_text("Search ~50,000 skills... (e.g., 'code review', 'refactor', 'test')")
                .desired_width(400.0),
        );

        if search_response.changed() || (search_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
            // Perform search
            let query = state.registry_search_query.trim();
            if query.len() >= 2 {
                let results = registry.search(query, 50);
                *state.registry_search_results = results.into_iter().cloned().collect();
            } else if query.is_empty() {
                state.registry_search_results.clear();
            }
        }

        ui.label(
            RichText::new(format!("{} skills in registry", registry.total))
                .small()
                .color(TEXT_MUTED),
        );
    });

    ui.add_space(8.0);

    // Install location toggle
    ui.horizontal(|ui| {
        ui.label(RichText::new("Install to:").small().color(TEXT_DIM));
        ui.add_space(8.0);

        let is_workspace = *state.registry_install_location == SkillInstallLocation::Workspace;
        let is_global = *state.registry_install_location == SkillInstallLocation::Global;

        if ui
            .selectable_label(
                is_workspace,
                RichText::new("📁 Workspace").small().color(
                    if is_workspace { ACCENT_CYAN } else { TEXT_MUTED }
                ),
            )
            .on_hover_text(".claude/skills/ and .codex/skills/ in current project")
            .clicked()
        {
            *state.registry_install_location = SkillInstallLocation::Workspace;
        }

        ui.add_space(8.0);

        if ui
            .selectable_label(
                is_global,
                RichText::new("🌐 Global").small().color(
                    if is_global { ACCENT_CYAN } else { TEXT_MUTED }
                ),
            )
            .on_hover_text("~/.claude/skills/ and ~/.codex/skills/ (available in all projects)")
            .clicked()
        {
            *state.registry_install_location = SkillInstallLocation::Global;
        }
    });

    ui.add_space(8.0);

    // Show install status
    if let Some((msg, is_error)) = state.registry_install_status.as_ref() {
        let color = if *is_error {
            crate::gui::theme::ACCENT_RED
        } else {
            ACCENT_GREEN
        };
        ui.label(RichText::new(msg).color(color));
        ui.add_space(4.0);
    }

    ui.add_space(8.0);

    // Results or help text
    if state.registry_search_results.is_empty() {
        if state.registry_search_query.is_empty() {
            ui.label(RichText::new("Popular searches:").color(TEXT_DIM));
            ui.horizontal(|ui| {
                for term in ["review", "refactor", "test", "debug", "document", "security"] {
                    if ui
                        .button(RichText::new(term).small().color(ACCENT_CYAN))
                        .clicked()
                    {
                        *state.registry_search_query = term.to_string();
                        let results = registry.search(term, 50);
                        *state.registry_search_results = results.into_iter().cloned().collect();
                    }
                }
            });
            ui.add_space(16.0);
            ui.label(
                RichText::new("Type at least 2 characters to search skills by name, description, or author.")
                    .small()
                    .color(TEXT_MUTED),
            );
        } else if state.registry_search_query.len() < 2 {
            ui.label(
                RichText::new("Type at least 2 characters to search...")
                    .small()
                    .color(TEXT_MUTED),
            );
        } else {
            ui.label(
                RichText::new("No skills found. Try a different search term.")
                    .color(TEXT_MUTED)
                    .italics(),
            );
        }
    } else {
        ui.label(
            RichText::new(format!(
                "Found {} skill(s) matching '{}'",
                state.registry_search_results.len(),
                state.registry_search_query
            ))
            .small()
            .color(TEXT_DIM),
        );
        ui.add_space(8.0);

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Clone results to avoid borrow issues
                let results: Vec<_> = state.registry_search_results.clone();

                for skill in &results {
                    render_registry_skill_card(ui, state, skill);
                    ui.add_space(4.0);
                }
            });
    }
}

/// Render a single skill card from the registry
fn render_registry_skill_card(
    ui: &mut egui::Ui,
    state: &mut SkillEditorState<'_>,
    skill: &crate::config::RegistrySkill,
) {
    egui::Frame::NONE
        .fill(BG_SECONDARY)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Author/Name
                ui.label(
                    RichText::new(format!("{}/", skill.author))
                        .monospace()
                        .color(TEXT_MUTED),
                );
                ui.label(
                    RichText::new(&skill.name)
                        .monospace()
                        .color(ACCENT_GREEN),
                );

                // Stars
                if skill.stars > 0 {
                    ui.label(
                        RichText::new(format!("★{}", skill.stars))
                            .small()
                            .color(ACCENT_YELLOW),
                    );
                }

                // Marketplace badge
                if skill.has_marketplace {
                    ui.label(
                        RichText::new("[marketplace]")
                            .small()
                            .color(ACCENT_PURPLE),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Install button
                    if ui
                        .button(RichText::new("📥 Install").small().color(ACCENT_CYAN))
                        .clicked()
                    {
                        install_skill_from_registry(state, skill);
                    }

                    // GitHub link (copy to clipboard)
                    if ui
                        .button(RichText::new("🔗").small().color(TEXT_DIM))
                        .on_hover_text(format!("Copy URL: {}", skill.github_url))
                        .clicked()
                    {
                        ui.ctx().copy_text(skill.github_url.clone());
                    }
                });
            });

            // Description
            if !skill.description.is_empty() {
                ui.add_space(4.0);
                // Truncate long descriptions (UTF-8 safe)
                let desc = truncate_str(&skill.description, 200);
                ui.label(RichText::new(desc).small().color(TEXT_DIM));
            }
        });
}

/// Install a skill from the registry
fn install_skill_from_registry(
    state: &mut SkillEditorState<'_>,
    skill: &crate::config::RegistrySkill,
) {
    *state.registry_install_status = Some((format!("Installing {}...", skill.name), false));

    // Get raw URL
    let Some(raw_url) = skill.raw_skill_url() else {
        *state.registry_install_status = Some((
            format!("Could not determine download URL for {}", skill.name),
            true,
        ));
        return;
    };

    // Download in a blocking way (TODO: make async)
    let content = match download_skill_content(&raw_url) {
        Ok(c) => c,
        Err(e) => {
            *state.registry_install_status = Some((format!("Download failed: {}", e), true));
            return;
        }
    };

    // Parse the content
    let mut skill_config = match parse_skill_content(&content) {
        Ok(c) => c,
        Err(e) => {
            *state.registry_install_status = Some((format!("Parse failed: {}", e), true));
            return;
        }
    };

    // Use registry name if parsed name is empty
    if skill_config.name.is_empty() {
        skill_config.name = skill.name.clone();
    }

    // Sanitize skill name for directory (replace spaces/special chars with hyphens)
    skill_config.name = sanitize_skill_name(&skill_config.name);

    // Install to both agent directories based on selected location
    let (claude_result, codex_result) = match *state.registry_install_location {
        SkillInstallLocation::Workspace => {
            // Install to .claude/skills/ and .codex/skills/ in workspace
            (
                save_skill(&skill_config, SkillAgentType::Claude, state.work_dir),
                save_skill(&skill_config, SkillAgentType::Codex, state.work_dir),
            )
        }
        SkillInstallLocation::Global => {
            // Install to ~/.claude/skills/ and ~/.codex/skills/
            (
                save_skill_global(&skill_config, SkillAgentType::Claude),
                save_skill_global(&skill_config, SkillAgentType::Codex),
            )
        }
    };

    let location_label = match *state.registry_install_location {
        SkillInstallLocation::Workspace => "workspace",
        SkillInstallLocation::Global => "global",
    };

    match (claude_result, codex_result) {
        (Ok(p1), Ok(p2)) => {
            *state.registry_install_status = Some((
                format!(
                    "✓ Installed {} ({}) to:\n  • {}\n  • {}",
                    skill.name,
                    location_label,
                    p1.display(),
                    p2.display()
                ),
                false,
            ));
        }
        (Err(e), _) | (_, Err(e)) => {
            *state.registry_install_status = Some((format!("Install failed: {}", e), true));
        }
    }
}

/// Download skill content from URL (blocking)
fn download_skill_content(url: &str) -> anyhow::Result<String> {
    let response = ureq::get(url).call()?;
    if response.status() != 200 {
        anyhow::bail!("HTTP {}", response.status());
    }
    Ok(response.into_string()?)
}

/// Truncate a string to max_chars characters (UTF-8 safe)
fn truncate_str(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}

/// Sanitize skill name for use as directory name
///
/// - Converts to lowercase
/// - Replaces spaces and special characters with hyphens
/// - Collapses multiple hyphens into one
/// - Trims leading/trailing hyphens
fn sanitize_skill_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    // Collapse multiple hyphens and trim
    let mut result = String::new();
    let mut last_was_hyphen = true; // Start true to skip leading hyphens
    for c in sanitized.chars() {
        if c == '-' {
            if !last_was_hyphen {
                result.push(c);
                last_was_hyphen = true;
            }
        } else {
            result.push(c);
            last_was_hyphen = false;
        }
    }

    // Remove trailing hyphen
    if result.ends_with('-') {
        result.pop();
    }

    result
}
