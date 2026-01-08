//! Configuration file I/O operations

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs2::FileExt;

use super::skill_discovery::SkillDiscovery;
use super::Config;

impl Config {
    /// Get the global config directory path (~/.kyco/)
    pub fn global_config_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".kyco")
    }

    /// Get the global config file path (~/.kyco/config.toml)
    pub fn global_config_path() -> PathBuf {
        Self::global_config_dir().join("config.toml")
    }

    /// Load configuration from a file without merging internal defaults.
    ///
    /// Use this when you need the raw config as stored in the file.
    /// For most use cases, prefer `from_file()` which merges internal defaults.
    pub(super) fn from_file_raw(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Load configuration from a file.
    ///
    /// This automatically merges internal defaults (modes, chains, agents)
    /// into the loaded config. New internal items are added, and items with
    /// higher versions replace existing ones.
    ///
    /// Note: This does NOT save the merged config. Use `Config::load()` if
    /// you want automatic saving after merge.
    pub fn from_file(path: &Path) -> Result<Self> {
        let mut config = Self::from_file_raw(path)?;

        // Always merge internal defaults so user gets new modes/chains/agents
        config.merge_internal_defaults();

        Ok(config)
    }

    /// Save configuration to a file with atomic write and file locking.
    ///
    /// This ensures:
    /// 1. Exclusive lock prevents concurrent writes from CLI and GUI
    /// 2. Atomic write (temp file + rename) prevents corruption on crash
    /// 3. Parent directory is created if needed
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let content = toml::to_string_pretty(self).with_context(|| "Failed to serialize config")?;

        // Create lock file (separate from config to avoid issues with rename)
        let lock_path = path.with_extension("toml.lock");
        let lock_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .with_context(|| format!("Failed to create lock file: {}", lock_path.display()))?;

        // Acquire exclusive lock (blocks until available)
        lock_file
            .lock_exclusive()
            .with_context(|| "Failed to acquire config lock")?;

        // Write to temp file first (atomic write pattern)
        let temp_path = path.with_extension("toml.tmp");
        let mut temp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .with_context(|| format!("Failed to create temp file: {}", temp_path.display()))?;

        temp_file
            .write_all(content.as_bytes())
            .with_context(|| "Failed to write config content")?;

        temp_file
            .sync_all()
            .with_context(|| "Failed to sync config file")?;

        // Atomic rename (overwrites existing file)
        std::fs::rename(&temp_path, path)
            .with_context(|| format!("Failed to rename config file: {}", path.display()))?;

        // Lock is automatically released when lock_file is dropped
        Ok(())
    }

    /// Load global configuration from ~/.kyco/config.toml
    /// If no config exists, auto-creates one with defaults.
    /// Also merges internal defaults (versioned) and saves if changes were made.
    pub fn load() -> Result<Self> {
        let global_path = Self::global_config_path();

        if !global_path.exists() {
            Self::auto_init()?;
        }

        // Use from_file_raw() to get the config without merging first
        let mut config = Self::from_file_raw(&global_path)?;

        // Merge internal defaults and save if changes were made
        if config.merge_internal_defaults() {
            // Save the updated config with new internal modes/chains/agents
            if let Err(e) = config.save_to_file(&global_path) {
                tracing::warn!("Failed to save config after merging internal defaults: {}", e);
            }
        }

        Ok(config)
    }

    /// Load configuration from a directory (legacy compatibility)
    /// Now just loads the global config, ignoring the directory parameter
    pub fn from_dir(_dir: &Path) -> Result<Self> {
        Self::load()
    }

    /// Auto-initialize global configuration when no config exists
    ///
    /// Uses file locking to prevent race conditions when multiple processes
    /// try to auto-init simultaneously.
    fn auto_init() -> Result<()> {
        let config_dir = Self::global_config_dir();
        let config_path = Self::global_config_path();

        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).with_context(|| {
                format!(
                    "Failed to create config directory: {}",
                    config_dir.display()
                )
            })?;
        }

        // Create lock file and acquire exclusive lock to prevent race conditions
        let lock_path = config_path.with_extension("toml.lock");
        let lock_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .with_context(|| format!("Failed to create lock file: {}", lock_path.display()))?;

        lock_file
            .lock_exclusive()
            .with_context(|| "Failed to acquire config lock for auto-init")?;

        // Re-check if config exists after acquiring lock (another process may have created it)
        if config_path.exists() {
            // Lock is released when lock_file is dropped
            return Ok(());
        }

        // http_token intentionally left empty for local development.
        // Auth is only enforced when http_token is explicitly set.
        let default_config = Self::with_defaults();
        let config_content = toml::to_string_pretty(&default_config)
            .with_context(|| "Failed to serialize default config")?;

        // Write to temp file first (atomic write pattern)
        let temp_path = config_path.with_extension("toml.tmp");
        let mut temp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .with_context(|| format!("Failed to create temp file: {}", temp_path.display()))?;

        temp_file
            .write_all(config_content.as_bytes())
            .with_context(|| "Failed to write config content")?;

        temp_file
            .sync_all()
            .with_context(|| "Failed to sync config file")?;

        // Atomic rename
        std::fs::rename(&temp_path, &config_path)
            .with_context(|| format!("Failed to rename config file: {}", config_path.display()))?;

        eprintln!("Created {}", config_path.display());
        // Lock is released when lock_file is dropped
        Ok(())
    }

    /// Create a config with sensible defaults from embedded internal defaults.
    ///
    /// This loads all built-in agents, modes, and chains from the embedded
    /// `assets/internal/defaults.toml` file.
    pub fn with_defaults() -> Self {
        let mut config = Self::default();
        config.merge_internal_defaults();
        config
    }

    /// Discover and load skills from the filesystem.
    ///
    /// Skills are loaded from:
    /// 1. Project-local: `.claude/skills/` and `.codex/skills/` (higher priority)
    /// 2. Global: `~/.kyco/skills/` (lower priority, fallback)
    ///
    /// Project-local skills override global skills with the same name.
    pub fn discover_skills(&mut self, project_dir: Option<&Path>) {
        let discovery = SkillDiscovery::new(project_dir.map(|p| p.to_path_buf()));
        self.skill = discovery.discover_all();
        tracing::debug!("Discovered {} skills", self.skill.len());
    }

    /// Load global configuration and discover skills for a project.
    ///
    /// This is the recommended way to load config when you know the project directory.
    /// It loads the global config, merges internal defaults, and discovers skills
    /// from both global and project-local directories.
    pub fn load_with_project(project_dir: Option<&Path>) -> Result<Self> {
        let mut config = Self::load()?;
        config.discover_skills(project_dir);
        Ok(config)
    }
}
