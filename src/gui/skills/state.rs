//! State for skill editing UI
//!
//! Skills are loaded from the filesystem (`.claude/skills/`, `.codex/skills/`, `~/.kyco/skills/`)
//! and displayed/edited as SKILL.md files.
//!
//! Registry browsing allows searching and installing from ~50,000 community skills.

use std::path::{Path, PathBuf};

use crate::config::{Config, RegistrySkill, SkillConfig, SkillRegistry};
use crate::gui::app::ViewMode;

/// Information about a skill's folder structure
#[derive(Debug, Default, Clone)]
pub struct SkillFolderInfo {
    /// Path to the skill directory (parent of SKILL.md)
    pub dir_path: Option<PathBuf>,
    /// Whether scripts/ directory exists
    pub has_scripts: bool,
    /// Whether references/ directory exists
    pub has_references: bool,
    /// Whether assets/ directory exists
    pub has_assets: bool,
    /// List of script files
    pub scripts: Vec<String>,
    /// List of reference files
    pub references: Vec<String>,
    /// List of asset files
    pub assets: Vec<String>,
}

impl SkillFolderInfo {
    /// Load folder info from a skill's source path
    pub fn from_skill(skill: &SkillConfig) -> Self {
        let Some(ref source_path) = skill.source_path else {
            return Self::default();
        };

        let Some(dir_path) = source_path.parent() else {
            return Self::default();
        };

        let mut info = Self {
            dir_path: Some(dir_path.to_path_buf()),
            ..Default::default()
        };

        // Check for scripts/
        let scripts_dir = dir_path.join("scripts");
        if scripts_dir.is_dir() {
            info.has_scripts = true;
            info.scripts = Self::list_files(&scripts_dir);
        }

        // Check for references/
        let refs_dir = dir_path.join("references");
        if refs_dir.is_dir() {
            info.has_references = true;
            info.references = Self::list_files(&refs_dir);
        }

        // Check for assets/
        let assets_dir = dir_path.join("assets");
        if assets_dir.is_dir() {
            info.has_assets = true;
            info.assets = Self::list_files(&assets_dir);
        }

        info
    }

    fn list_files(dir: &Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .ok()
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|e| e.file_name().to_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if this skill has any additional resources
    pub fn has_resources(&self) -> bool {
        self.has_scripts || self.has_references || self.has_assets
    }
}

/// Which tab is active in the skills view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkillsTab {
    #[default]
    Local,
    Registry,
}

/// Where to install skills from the registry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkillInstallLocation {
    /// Install to project workspace (.claude/skills/, .codex/skills/)
    #[default]
    Workspace,
    /// Install to global location (~/.kyco/skills/)
    Global,
}

/// State for skill editing UI
pub struct SkillEditorState<'a> {
    /// Currently selected skill name (None = list view, Some("__new__") = create new)
    pub selected_skill: &'a mut Option<String>,
    /// The raw SKILL.md content being edited
    pub skill_edit_content: &'a mut String,
    /// Status message (message, is_error)
    pub skill_edit_status: &'a mut Option<(String, bool)>,
    /// Folder info for the selected skill
    pub skill_folder_info: &'a mut SkillFolderInfo,
    /// Name field for new skills
    pub skill_edit_name: &'a mut String,
    /// Current view mode
    pub view_mode: &'a mut ViewMode,
    /// Config with discovered skills
    pub config: &'a Config,
    /// Working directory for skill operations
    pub work_dir: &'a Path,
    /// Current tab (Local/Registry)
    pub skills_tab: &'a mut SkillsTab,
    /// Registry search query
    pub registry_search_query: &'a mut String,
    /// Registry search results (cached)
    pub registry_search_results: &'a mut Vec<RegistrySkill>,
    /// Loaded registry (lazily loaded)
    pub registry: &'a mut Option<SkillRegistry>,
    /// Registry install status message
    pub registry_install_status: &'a mut Option<(String, bool)>,
    /// Where to install skills (Workspace or Global)
    pub registry_install_location: &'a mut SkillInstallLocation,
}
