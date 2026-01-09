//! Skill discovery from filesystem
//!
//! Discovers skills from:
//! 1. Project-local: `.claude/skills/` or `.codex/skills/` (higher priority)
//! 2. Global: `~/.kyco/skills/` (lower priority, fallback)
//!
//! Skills can be either:
//! - **Directory-based** (recommended): `skill-name/SKILL.md` with optional scripts/, references/, assets/
//! - **Single file** (legacy): `skill-name.md`

use super::skill::SkillConfig;
use super::skill_parser::{parse_skill_file, SkillParseError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Agent type for skill discovery
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillAgentType {
    Claude,
    Codex,
}

impl SkillAgentType {
    /// Get the directory name for this agent type
    pub fn dir_name(&self) -> &'static str {
        match self {
            SkillAgentType::Claude => ".claude",
            SkillAgentType::Codex => ".codex",
        }
    }
}

/// Skill discovery configuration
pub struct SkillDiscovery {
    /// Project directory for local skills
    project_dir: Option<PathBuf>,
    /// User home directory for global skills
    home_dir: Option<PathBuf>,
}

impl SkillDiscovery {
    /// Create a new skill discovery instance
    pub fn new(project_dir: Option<PathBuf>) -> Self {
        Self {
            project_dir,
            home_dir: dirs::home_dir(),
        }
    }

    /// Create with explicit paths (for testing)
    #[cfg(test)]
    pub fn with_paths(project_dir: Option<PathBuf>, home_dir: Option<PathBuf>) -> Self {
        Self {
            project_dir,
            home_dir,
        }
    }

    /// Discover all skills for all agent types
    pub fn discover_all(&self) -> HashMap<String, SkillConfig> {
        let mut skills = HashMap::new();

        // Load global skills first (lowest priority)
        self.load_global_skills(&mut skills);

        // Load project-local skills for both agents (higher priority)
        for agent in [SkillAgentType::Claude, SkillAgentType::Codex] {
            self.load_project_skills(agent, &mut skills);
        }

        skills
    }

    /// Discover skills for a specific agent type
    pub fn discover_for_agent(&self, agent: SkillAgentType) -> HashMap<String, SkillConfig> {
        let mut skills = HashMap::new();

        // Load global skills first (lower priority)
        self.load_global_skills(&mut skills);

        // Load project-local skills (higher priority, overwrites global)
        self.load_project_skills(agent, &mut skills);

        skills
    }

    /// Load global skills from ~/.kyco/skills/
    fn load_global_skills(&self, skills: &mut HashMap<String, SkillConfig>) {
        if let Some(ref home) = self.home_dir {
            let global_dir = home.join(".kyco/skills");
            self.load_skills_from_dir(&global_dir, skills);
        }
    }

    /// Load project-local skills for a specific agent
    fn load_project_skills(&self, agent: SkillAgentType, skills: &mut HashMap<String, SkillConfig>) {
        if let Some(ref project) = self.project_dir {
            let agent_dir = project.join(agent.dir_name()).join("skills");
            self.load_skills_from_dir(&agent_dir, skills);
        }
    }

    /// Load all skills from a directory
    ///
    /// Supports both:
    /// - Directory-based skills: `skill-name/SKILL.md`
    /// - Single file skills (legacy): `skill-name.md`
    fn load_skills_from_dir(&self, dir: &Path, skills: &mut HashMap<String, SkillConfig>) {
        if !dir.exists() {
            debug!("Skills directory does not exist: {:?}", dir);
            return;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                warn!("Failed to read skills directory {:?}: {}", dir, e);
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Directory-based skill: look for SKILL.md inside
                let skill_md_path = path.join("SKILL.md");
                if skill_md_path.exists() {
                    match parse_skill_file(&skill_md_path) {
                        Ok(skill) => {
                            debug!("Loaded skill '{}' from {:?}", skill.name, skill_md_path);
                            skills.insert(skill.name.clone(), skill);
                        }
                        Err(e) => {
                            warn!("Failed to parse skill file {:?}: {}", skill_md_path, e);
                        }
                    }
                }
            } else if path.extension().is_some_and(|ext| ext == "md") {
                // Legacy single-file skill: skill-name.md
                match parse_skill_file(&path) {
                    Ok(skill) => {
                        debug!("Loaded skill '{}' from {:?} (legacy format)", skill.name, path);
                        skills.insert(skill.name.clone(), skill);
                    }
                    Err(e) => {
                        warn!("Failed to parse skill file {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    /// Get the path where a skill directory should be created
    pub fn get_skill_path(&self, name: &str, agent: SkillAgentType) -> Option<PathBuf> {
        self.project_dir.as_ref().map(|project| {
            project
                .join(agent.dir_name())
                .join("skills")
                .join(name)
                .join("SKILL.md")
        })
    }

    /// Get the global skill path
    pub fn get_global_skill_path(&self, name: &str) -> Option<PathBuf> {
        self.home_dir
            .as_ref()
            .map(|home| home.join(".kyco/skills").join(name).join("SKILL.md"))
    }

    /// Get the skill directory path (not the SKILL.md file)
    pub fn get_skill_dir(&self, name: &str, agent: SkillAgentType) -> Option<PathBuf> {
        self.project_dir.as_ref().map(|project| {
            project
                .join(agent.dir_name())
                .join("skills")
                .join(name)
        })
    }

    /// List all skill directories that would be searched
    pub fn list_skill_directories(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // Global directory
        if let Some(ref home) = self.home_dir {
            dirs.push(home.join(".kyco/skills"));
        }

        // Project directories
        if let Some(ref project) = self.project_dir {
            dirs.push(project.join(".claude/skills"));
            dirs.push(project.join(".codex/skills"));
        }

        dirs
    }
}

/// Save a skill to a directory structure
///
/// Creates the full Agent Skills directory structure:
/// ```text
/// .claude/skills/skill-name/
/// ├── SKILL.md
/// ├── scripts/      # Executable scripts for agents
/// ├── references/   # Additional documentation
/// └── assets/       # Static resources (templates, images)
/// ```
pub fn save_skill(
    skill: &SkillConfig,
    agent: SkillAgentType,
    project_dir: &Path,
) -> Result<PathBuf, SkillParseError> {
    let skill_dir = project_dir
        .join(agent.dir_name())
        .join("skills")
        .join(&skill.name);

    // Create skill directory with full structure per agentskills.io spec
    std::fs::create_dir_all(&skill_dir)?;
    std::fs::create_dir_all(skill_dir.join("scripts"))?;
    std::fs::create_dir_all(skill_dir.join("references"))?;
    std::fs::create_dir_all(skill_dir.join("assets"))?;

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = skill.to_skill_md();

    std::fs::write(&skill_md_path, content)?;

    Ok(skill_md_path)
}

/// Save a skill to global user location for a specific agent
///
/// Creates the full Agent Skills directory structure:
/// ```text
/// ~/.claude/skills/skill-name/  (for Claude)
/// ~/.codex/skills/skill-name/   (for Codex)
/// ├── SKILL.md
/// ├── scripts/      # Executable scripts for agents
/// ├── references/   # Additional documentation
/// └── assets/       # Static resources (templates, images)
/// ```
pub fn save_skill_global(
    skill: &SkillConfig,
    agent: SkillAgentType,
) -> Result<PathBuf, SkillParseError> {
    let home = dirs::home_dir().ok_or_else(|| {
        SkillParseError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Home directory not found",
        ))
    })?;

    let skill_dir = home
        .join(agent.dir_name())
        .join("skills")
        .join(&skill.name);

    // Create skill directory with full structure per agentskills.io spec
    std::fs::create_dir_all(&skill_dir)?;
    std::fs::create_dir_all(skill_dir.join("scripts"))?;
    std::fs::create_dir_all(skill_dir.join("references"))?;
    std::fs::create_dir_all(skill_dir.join("assets"))?;

    let skill_md_path = skill_dir.join("SKILL.md");
    let content = skill.to_skill_md();

    std::fs::write(&skill_md_path, content)?;

    Ok(skill_md_path)
}

/// Delete a skill (directory or single file)
pub fn delete_skill(
    name: &str,
    agent: SkillAgentType,
    project_dir: &Path,
) -> Result<(), SkillParseError> {
    let skill_dir = project_dir
        .join(agent.dir_name())
        .join("skills")
        .join(name);

    // Try to delete as directory first (new format)
    if skill_dir.is_dir() {
        std::fs::remove_dir_all(&skill_dir)?;
        return Ok(());
    }

    // Fall back to single file (legacy format)
    let skill_file = project_dir
        .join(agent.dir_name())
        .join("skills")
        .join(format!("{}.md", name));

    if skill_file.exists() {
        std::fs::remove_file(&skill_file)?;
    }

    Ok(())
}

/// Delete a global skill (directory or single file) for a specific agent
pub fn delete_skill_global(name: &str, agent: SkillAgentType) -> Result<(), SkillParseError> {
    let home = dirs::home_dir().ok_or_else(|| {
        SkillParseError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Home directory not found",
        ))
    })?;

    let agent_dir = home.join(agent.dir_name()).join("skills");
    let skill_dir = agent_dir.join(name);

    // Try to delete as directory first (new format)
    if skill_dir.is_dir() {
        std::fs::remove_dir_all(&skill_dir)?;
        return Ok(());
    }

    // Fall back to single file (legacy format)
    let skill_file = agent_dir.join(format!("{}.md", name));

    if skill_file.exists() {
        std::fs::remove_file(&skill_file)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Create a directory-based skill (new format)
    fn create_test_skill_dir(parent: &Path, name: &str, content: &str) {
        let skill_dir = parent.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let path = skill_dir.join("SKILL.md");
        std::fs::write(path, content).unwrap();
    }

    /// Create a single-file skill (legacy format)
    fn create_test_skill_file(dir: &Path, name: &str, content: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(format!("{}.md", name));
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn test_discover_directory_based_skill() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().to_path_buf();

        // Create a directory-based skill
        let skill_dir = project.join(".claude/skills");
        create_test_skill_dir(
            &skill_dir,
            "test-skill",
            r#"---
name: test-skill
description: A test skill
---

Instructions here."#,
        );

        let discovery = SkillDiscovery::with_paths(Some(project), None);
        let skills = discovery.discover_for_agent(SkillAgentType::Claude);

        assert_eq!(skills.len(), 1);
        assert!(skills.contains_key("test-skill"));
    }

    #[test]
    fn test_discover_legacy_single_file_skill() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().to_path_buf();

        // Create a legacy single-file skill
        let skill_dir = project.join(".claude/skills");
        create_test_skill_file(
            &skill_dir,
            "legacy-skill",
            r#"---
name: legacy-skill
description: A legacy skill
---

Instructions here."#,
        );

        let discovery = SkillDiscovery::with_paths(Some(project), None);
        let skills = discovery.discover_for_agent(SkillAgentType::Claude);

        assert_eq!(skills.len(), 1);
        assert!(skills.contains_key("legacy-skill"));
    }

    #[test]
    fn test_discover_from_global_dir() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().to_path_buf();

        // Create a global skill
        let skill_dir = home.join(".kyco/skills");
        create_test_skill_dir(
            &skill_dir,
            "global-skill",
            r#"---
name: global-skill
description: A global skill
---

Global instructions."#,
        );

        let discovery = SkillDiscovery::with_paths(None, Some(home));
        let skills = discovery.discover_all();

        assert_eq!(skills.len(), 1);
        assert!(skills.contains_key("global-skill"));
    }

    #[test]
    fn test_project_skills_override_global() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        let home = temp.path().join("home");

        // Create global skill
        let global_dir = home.join(".kyco/skills");
        create_test_skill_dir(
            &global_dir,
            "shared",
            r#"---
name: shared
description: Global version
---

Global."#,
        );

        // Create project skill with same name
        let project_skills = project.join(".claude/skills");
        create_test_skill_dir(
            &project_skills,
            "shared",
            r#"---
name: shared
description: Project version
---

Project."#,
        );

        let discovery = SkillDiscovery::with_paths(Some(project), Some(home));
        let skills = discovery.discover_for_agent(SkillAgentType::Claude);

        assert_eq!(skills.len(), 1);
        let skill = skills.get("shared").unwrap();
        assert_eq!(skill.description, Some("Project version".to_string()));
    }

    #[test]
    fn test_save_and_delete_skill() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().to_path_buf();

        let skill = SkillConfig {
            name: "test-save".to_string(),
            description: Some("Test saving".to_string()),
            instructions: "Do something.".to_string(),
            kyco: Default::default(),
            source_path: None,
        };

        // Save
        let path = save_skill(&skill, SkillAgentType::Claude, &project).unwrap();
        assert!(path.exists());
        assert!(path.ends_with("test-save/SKILL.md"));

        // Verify full directory structure per agentskills.io spec
        let skill_dir = path.parent().unwrap();
        assert!(skill_dir.is_dir());
        assert_eq!(skill_dir.file_name().unwrap(), "test-save");
        assert!(skill_dir.join("scripts").is_dir(), "scripts/ should exist");
        assert!(skill_dir.join("references").is_dir(), "references/ should exist");
        assert!(skill_dir.join("assets").is_dir(), "assets/ should exist");

        // Delete
        delete_skill("test-save", SkillAgentType::Claude, &project).unwrap();
        assert!(!skill_dir.exists());
    }
}
