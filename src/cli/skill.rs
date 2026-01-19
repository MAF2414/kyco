//! Skill CRUD commands (manage SKILL.md files).
//!
//! Skills are stored as SKILL.md files in:
//! - Project-local: `.claude/skills/` or `.codex/skills/`
//! - Global: `~/.kyco/skills/`
//!
//! Skills can be searched and installed from the community registry (~50,000 skills).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::{
    create_skill_template, delete_skill, delete_skill_global, parse_skill_content, save_skill,
    save_skill_global, validate_skill, validate_skill_name, SkillAgentType, SkillDiscovery,
    SkillRegistry,
};

/// List all available skills
pub fn skill_list_command(work_dir: &Path, json: bool, agent: Option<&str>) -> Result<()> {
    let discovery = SkillDiscovery::new(Some(work_dir.to_path_buf()));

    let skills = match agent {
        Some("claude") => discovery.discover_for_agent(SkillAgentType::Claude),
        Some("codex") => discovery.discover_for_agent(SkillAgentType::Codex),
        _ => discovery.discover_all(),
    };

    let mut names: Vec<(&String, Option<&PathBuf>)> = skills
        .iter()
        .map(|(name, skill)| (name, skill.source_path.as_ref()))
        .collect();
    names.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    if json {
        let skill_list: Vec<serde_json::Value> = names
            .iter()
            .map(|(name, path)| {
                serde_json::json!({
                    "name": name,
                    "path": path.map(|p| p.display().to_string())
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&skill_list)?);
    } else {
        if names.is_empty() {
            println!("No skills found.");
            println!();
            println!("Skills are loaded from:");
            for dir in discovery.list_skill_directories() {
                println!("  - {}", dir.display());
            }
            println!();
            println!("Create a skill with: kyco skill create <name>");
        } else {
            for (name, path) in names {
                if let Some(p) = path {
                    println!("{} ({})", name, p.display());
                } else {
                    println!("{}", name);
                }
            }
        }
    }
    Ok(())
}

/// Get a skill definition
pub fn skill_get_command(work_dir: &Path, name: &str, json: bool) -> Result<()> {
    let discovery = SkillDiscovery::new(Some(work_dir.to_path_buf()));
    let skills = discovery.discover_all();

    let Some(skill) = skills.get(name) else {
        anyhow::bail!("Skill not found: {}", name);
    };

    if json {
        println!("{}", serde_json::to_string_pretty(skill)?);
    } else {
        // Print the SKILL.md format
        if let Some(ref path) = skill.source_path {
            println!("# Source: {}\n", path.display());
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read skill file: {}", path.display()))?;
            println!("{}", content);
        } else {
            println!("{}", skill.to_skill_md());
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct SkillCreateArgs {
    pub name: String,
    pub description: Option<String>,
    pub agent: Option<String>,
    pub global: bool,
    pub json: bool,
}

/// Create a new skill
pub fn skill_create_command(work_dir: &Path, args: SkillCreateArgs) -> Result<()> {
    // Validate name per agentskills.io specification
    if let Err(e) = validate_skill_name(&args.name) {
        anyhow::bail!("{}", e);
    }

    // Description is required per spec - prompt user if missing
    let description = match &args.description {
        Some(d) if !d.is_empty() => d.clone(),
        _ => {
            anyhow::bail!(
                "Description is required per agentskills.io specification.\n\
                 Use: kyco skill create {} --description \"What it does and when to use it\"",
                args.name
            );
        }
    };

    // Create skill from template
    let skill = create_skill_template(&args.name, Some(&description));

    // Validate the complete skill
    if let Err(e) = validate_skill(&skill) {
        anyhow::bail!("{}", e);
    }

    // Determine where to save
    let path = if args.global {
        // Global: save to ~/.claude/skills/ or ~/.codex/skills/
        let agent_type = match args.agent.as_deref() {
            Some("codex") => SkillAgentType::Codex,
            _ => SkillAgentType::Claude, // Default to claude
        };
        save_skill_global(&skill, agent_type)?
    } else {
        let agent_type = match args.agent.as_deref() {
            Some("codex") => SkillAgentType::Codex,
            _ => SkillAgentType::Claude, // Default to claude
        };
        save_skill(&skill, agent_type, work_dir)?
    };

    // Get skill directory (parent of SKILL.md)
    let skill_dir = path.parent().unwrap_or(&path);

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "name": skill.name,
                "path": path.display().to_string(),
                "directory": skill_dir.display().to_string(),
                "structure": {
                    "skill_md": "SKILL.md",
                    "scripts": "scripts/",
                    "references": "references/",
                    "assets": "assets/"
                }
            }))?
        );
    } else {
        println!("✓ Created skill: {}", skill.name);
        println!();
        println!("Directory: {}", skill_dir.display());
        println!("├── SKILL.md        # Skill instructions (edit this)");
        println!("├── scripts/        # Executable scripts for agents");
        println!("├── references/     # Additional documentation");
        println!("└── assets/         # Static resources (templates, images)");
        println!();
        println!("Edit SKILL.md to customize the skill instructions.");
        println!("Add scripts, references, or assets as needed.");
    }
    Ok(())
}

/// Delete a skill
pub fn skill_delete_command(
    work_dir: &Path,
    name: &str,
    agent: Option<&str>,
    global: bool,
) -> Result<()> {
    if global {
        // Global: delete from ~/.claude/skills/ and/or ~/.codex/skills/
        let agent_type = match agent {
            Some("codex") => SkillAgentType::Codex,
            _ => SkillAgentType::Claude,
        };
        delete_skill_global(name, agent_type)?;
        println!("Deleted global skill: {}", name);
    } else {
        let agent_type = match agent {
            Some("codex") => SkillAgentType::Codex,
            _ => SkillAgentType::Claude,
        };
        delete_skill(name, agent_type, work_dir)?;
        println!("Deleted skill: {}", name);
    }
    Ok(())
}

/// Show skill file path (for editors)
pub fn skill_path_command(work_dir: &Path, name: &str, agent: Option<&str>) -> Result<()> {
    let discovery = SkillDiscovery::new(Some(work_dir.to_path_buf()));
    let skills = discovery.discover_all();

    if let Some(skill) = skills.get(name) {
        if let Some(ref path) = skill.source_path {
            println!("{}", path.display());
            return Ok(());
        }
    }

    // Skill doesn't exist, show where it would be created
    let agent_type = match agent {
        Some("codex") => SkillAgentType::Codex,
        _ => SkillAgentType::Claude,
    };

    if let Some(path) = discovery.get_skill_path(name, agent_type) {
        println!("{}", path.display());
    } else {
        anyhow::bail!("Could not determine skill path");
    }

    Ok(())
}

/// Install a built-in skill (copy from assets to project)
///
/// Skills are installed to ALL agent directories so all agents have access:
/// - `.claude/skills/<name>/SKILL.md`
/// - `.codex/skills/<name>/SKILL.md`
/// - `~/.kyco/skills/<name>/SKILL.md` (if --global is specified)
pub fn skill_install_command(
    work_dir: &Path,
    name: &str,
    _agent: Option<&str>, // Ignored - always installs to all agents
    global: bool,
) -> Result<()> {
    // Validate name per agentskills.io specification
    if let Err(e) = validate_skill_name(name) {
        anyhow::bail!("{}", e);
    }

    // Create skill from template with default description
    // TODO: In the future, this could copy from bundled assets or download from a registry
    let description = format!(
        "A template skill for {}. Edit this description to explain what it does and when to use it.",
        name
    );
    let skill = create_skill_template(name, Some(&description));

    let mut installed_paths = Vec::new();

    // Install to .claude/skills/
    let claude_path = save_skill(&skill, SkillAgentType::Claude, work_dir)?;
    installed_paths.push(claude_path);

    // Install to .codex/skills/
    let codex_path = save_skill(&skill, SkillAgentType::Codex, work_dir)?;
    installed_paths.push(codex_path);

    // Optionally install to global ~/.claude/skills/ and ~/.codex/skills/
    if global {
        let global_claude_path = save_skill_global(&skill, SkillAgentType::Claude)?;
        installed_paths.push(global_claude_path);
        let global_codex_path = save_skill_global(&skill, SkillAgentType::Codex)?;
        installed_paths.push(global_codex_path);
    }

    println!("✓ Installed skill '{}' to:", name);
    for path in &installed_paths {
        let skill_dir = path.parent().unwrap_or(path);
        println!("  - {}/", skill_dir.display());
    }
    println!();
    println!("Each skill directory contains:");
    println!("  ├── SKILL.md        # Skill instructions (edit this)");
    println!("  ├── scripts/        # Executable scripts for agents");
    println!("  ├── references/     # Additional documentation");
    println!("  └── assets/         # Static resources");
    println!();
    println!("Edit SKILL.md to customize the skill instructions.");
    if !global {
        println!("Tip: Use --global to also install to ~/.claude/skills/ and ~/.codex/skills/ for system-wide access.");
    }

    Ok(())
}

// =============================================================================
// Registry Commands (search & install from community registry)
// =============================================================================

/// Search for skills in the community registry
pub fn skill_search_command(query: &str, limit: usize, json: bool) -> Result<()> {
    let registry = SkillRegistry::load_embedded()
        .context("Failed to load skill registry")?;

    let results = registry.search(query, limit);

    if json {
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "author": s.author,
                    "description": s.description,
                    "stars": s.stars,
                    "githubUrl": s.github_url,
                    "hasMarketplace": s.has_marketplace
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else {
        if results.is_empty() {
            println!("No skills found matching '{}'", query);
            println!();
            println!("Try broader search terms or browse popular skills:");
            println!("  kyco skill search review");
            println!("  kyco skill search refactor");
            println!("  kyco skill search test");
        } else {
            println!("Found {} skill(s) matching '{}':\n", results.len(), query);
            for skill in &results {
                let stars = if skill.stars > 0 {
                    format!(" ★{}", skill.stars)
                } else {
                    String::new()
                };
                let marketplace = if skill.has_marketplace { " [marketplace]" } else { "" };
                println!(
                    "  {}/{}{}{}",
                    skill.author, skill.name, stars, marketplace
                );
                if !skill.description.is_empty() {
                    // Truncate description to first line, max 80 chars (UTF-8 safe)
                    println!("    {}", truncate_description(&skill.description, 80));
                }
                println!();
            }
            println!("Install a skill with:");
            println!("  kyco skill install-from-registry <author>/<name>");
        }
    }

    Ok(())
}

/// Show details about a skill in the registry
pub fn skill_info_command(full_name: &str, json: bool) -> Result<()> {
    let registry = SkillRegistry::load_embedded()
        .context("Failed to load skill registry")?;

    // Try author/name format first, then just name
    let skill = registry
        .get_by_full_name(full_name)
        .or_else(|| registry.get_by_name(full_name))
        .ok_or_else(|| anyhow::anyhow!("Skill not found: {}", full_name))?;

    if json {
        println!("{}", serde_json::to_string_pretty(skill)?);
    } else {
        println!("Skill: {}/{}", skill.author, skill.name);
        println!("Stars: {}", skill.stars);
        println!("Forks: {}", skill.forks);
        if skill.has_marketplace {
            println!("Listed: Yes (marketplace)");
        }
        println!();
        println!("Description:");
        println!("  {}", skill.description);
        println!();
        println!("GitHub: {}", skill.github_url);
        if let Some(url) = skill.raw_skill_url() {
            println!("Raw URL: {}", url);
        }
        println!();
        println!("Install with:");
        println!("  kyco skill install-from-registry {}/{}", skill.author, skill.name);
    }

    Ok(())
}

/// Install a skill from the registry by downloading from GitHub
pub fn skill_install_from_registry_command(
    work_dir: &Path,
    full_name: &str,
    agent: Option<&str>,
    global: bool,
) -> Result<()> {
    let registry = SkillRegistry::load_embedded()
        .context("Failed to load skill registry")?;

    // Find the skill
    let skill = registry
        .get_by_full_name(full_name)
        .or_else(|| registry.get_by_name(full_name))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Skill not found: {}\n\nSearch for skills with: kyco skill search <query>",
                full_name
            )
        })?;

    println!("Installing {}/{}...", skill.author, skill.name);

    // Get the raw URL for the SKILL.md
    let raw_url = skill
        .raw_skill_url()
        .ok_or_else(|| anyhow::anyhow!("Could not determine download URL for skill"))?;

    // Download the SKILL.md content
    let content = download_skill_content(&raw_url)
        .with_context(|| format!("Failed to download skill from {}", raw_url))?;

    // Parse the downloaded content to get a SkillConfig
    let mut skill_config = parse_skill_content(&content)
        .with_context(|| "Failed to parse downloaded SKILL.md")?;

    // Use the registry skill name (might differ from the parsed name)
    if skill_config.name.is_empty() {
        skill_config.name = skill.name.clone();
    }

    // Sanitize skill name for directory (replace spaces/special chars with hyphens)
    skill_config.name = sanitize_skill_name(&skill_config.name);

    // Save to appropriate locations
    let mut installed_paths = Vec::new();

    if global {
        // Global: install to ~/.claude/skills/ and/or ~/.codex/skills/
        match agent {
            Some("claude") => {
                let path = save_skill_global(&skill_config, SkillAgentType::Claude)?;
                installed_paths.push(path);
            }
            Some("codex") => {
                let path = save_skill_global(&skill_config, SkillAgentType::Codex)?;
                installed_paths.push(path);
            }
            _ => {
                // Install to both by default
                let claude_path = save_skill_global(&skill_config, SkillAgentType::Claude)?;
                installed_paths.push(claude_path);
                let codex_path = save_skill_global(&skill_config, SkillAgentType::Codex)?;
                installed_paths.push(codex_path);
            }
        }
    } else {
        // Workspace: install to .claude/skills/ and/or .codex/skills/
        match agent {
            Some("claude") => {
                let path = save_skill(&skill_config, SkillAgentType::Claude, work_dir)?;
                installed_paths.push(path);
            }
            Some("codex") => {
                let path = save_skill(&skill_config, SkillAgentType::Codex, work_dir)?;
                installed_paths.push(path);
            }
            _ => {
                // Install to both by default
                let claude_path = save_skill(&skill_config, SkillAgentType::Claude, work_dir)?;
                installed_paths.push(claude_path);
                let codex_path = save_skill(&skill_config, SkillAgentType::Codex, work_dir)?;
                installed_paths.push(codex_path);
            }
        }
    }

    println!("✓ Installed {}/{} to:", skill.author, skill.name);
    for path in &installed_paths {
        println!("  - {}", path.display());
    }

    if !skill.description.is_empty() {
        println!();
        println!("Description: {}", truncate_description(&skill.description, 100));
    }

    println!();
    println!("Use the skill with: kyco job start --skill {} --file <file>", skill_config.name);

    Ok(())
}

/// Download skill content from a URL
fn download_skill_content(url: &str) -> Result<String> {
    let response = ureq::get(url)
        .call()
        .with_context(|| format!("HTTP request failed: {}", url))?;

    if response.status() != 200 {
        anyhow::bail!(
            "Failed to download skill: HTTP {} - {}",
            response.status(),
            response.status_text()
        );
    }

    let content = response
        .into_string()
        .context("Failed to read response body")?;

    Ok(content)
}

/// Truncate description for display
fn truncate_description(desc: &str, max_len: usize) -> String {
    let first_line = desc.lines().next().unwrap_or(desc);
    truncate_chars(first_line, max_len)
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }

    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }

    if max_chars <= 3 {
        return s.chars().take(max_chars).collect();
    }

    let truncated: String = s.chars().take(max_chars - 3).collect();
    format!("{}...", truncated)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_skill_create_and_list() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();

        // Create a skill (description is now required per agentskills.io spec)
        let args = SkillCreateArgs {
            name: "test-skill".to_string(),
            description: Some("A test skill for unit testing. Use when testing the skill system.".to_string()),
            agent: Some("claude".to_string()),
            global: false,
            json: false,
        };
        skill_create_command(work_dir, args).unwrap();

        // Verify full directory structure per agentskills.io spec
        let skill_dir = work_dir.join(".claude/skills/test-skill");
        assert!(skill_dir.is_dir(), "Skill directory should exist");
        assert!(skill_dir.join("SKILL.md").exists(), "SKILL.md should exist");
        assert!(skill_dir.join("scripts").is_dir(), "scripts/ should exist");
        assert!(skill_dir.join("references").is_dir(), "references/ should exist");
        assert!(skill_dir.join("assets").is_dir(), "assets/ should exist");

        // List should find it
        let discovery = SkillDiscovery::new(Some(work_dir.to_path_buf()));
        let skills = discovery.discover_all();
        assert!(skills.contains_key("test-skill"));
    }

    #[test]
    fn test_skill_create_requires_description() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();

        // Try to create a skill without description - should fail
        let args = SkillCreateArgs {
            name: "no-description".to_string(),
            description: None,
            agent: Some("claude".to_string()),
            global: false,
            json: false,
        };
        let result = skill_create_command(work_dir, args);
        assert!(result.is_err(), "Should fail without description");
    }

    #[test]
    fn test_skill_name_validation() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();

        // Invalid names should fail
        let invalid_names = vec![
            "Invalid",       // uppercase
            "-starts-dash",  // leading hyphen
            "ends-dash-",    // trailing hyphen
            "double--dash",  // consecutive hyphens
            "has space",     // space
            "has_underscore", // underscore
        ];

        for name in invalid_names {
            let args = SkillCreateArgs {
                name: name.to_string(),
                description: Some("A description".to_string()),
                agent: Some("claude".to_string()),
                global: false,
                json: false,
            };
            let result = skill_create_command(work_dir, args);
            assert!(result.is_err(), "Name '{}' should be rejected", name);
        }
    }

    #[test]
    fn test_skill_delete() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path();

        // Create a skill (description is now required)
        let args = SkillCreateArgs {
            name: "to-delete".to_string(),
            description: Some("A skill to be deleted. Use when testing deletion.".to_string()),
            agent: Some("claude".to_string()),
            global: false,
            json: false,
        };
        skill_create_command(work_dir, args).unwrap();

        // Verify it was created
        let skill_dir = work_dir.join(".claude/skills/to-delete");
        assert!(skill_dir.exists(), "Skill directory should exist before delete");

        // Delete it
        skill_delete_command(work_dir, "to-delete", Some("claude"), false).unwrap();

        // Verify it's gone (entire directory)
        assert!(!skill_dir.exists(), "Skill directory should be deleted");
    }

    #[test]
    fn test_truncate_description_utf8_safe() {
        // Contains multibyte characters (Korean).
        let s = "dotnet CLI를 사용하여 .NET 솔루션/프로젝트를 빌드합니다. 컴파일, 종속성 복원 또는 아티팩트 빌드 작업 시 사용합니다.";
        let truncated = truncate_description(s, 80);
        assert!(!truncated.is_empty());
        assert!(truncated.chars().count() <= 80);
    }
}
