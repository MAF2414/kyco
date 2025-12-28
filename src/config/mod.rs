//! Configuration loading and management

mod agent;
mod alias;
mod chain;
mod internal;
mod mode;
mod scope;
mod settings;
mod target;

pub use agent::AgentConfigToml;
pub use alias::AliasConfig;
pub use chain::{ChainStep, ModeChain, ModeOrChain, StateDefinition};
pub use internal::{InternalDefaults, INTERNAL_DEFAULTS_TOML};
pub use mode::{ClaudeModeOptions, CodexModeOptions, ModeConfig, ModeSessionType};
pub use scope::ScopeConfig;
pub use settings::{
    default_orchestrator_system_prompt, GuiSettings, OrchestratorSettings, RegistrySettings,
    Settings, VoiceSettings,
};
pub use target::TargetConfig;

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::{AgentConfig, SdkType, SessionMode};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Agent configurations
    #[serde(default)]
    pub agent: HashMap<String, AgentConfigToml>,

    /// Mode configurations (prompt builders)
    #[serde(default)]
    pub mode: HashMap<String, ModeConfig>,

    /// Mode chain configurations (sequential mode execution)
    #[serde(default)]
    pub chain: HashMap<String, ModeChain>,

    /// Scope configurations
    #[serde(default)]
    pub scope: HashMap<String, ScopeConfig>,

    /// Target configurations
    #[serde(default)]
    pub target: HashMap<String, TargetConfig>,

    /// Alias configurations
    #[serde(default)]
    pub alias: AliasConfig,

    /// General settings
    #[serde(default)]
    pub settings: Settings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent: HashMap::new(),
            mode: HashMap::new(),
            chain: HashMap::new(),
            scope: HashMap::new(),
            target: HashMap::new(),
            alias: AliasConfig::default(),
            settings: Settings::default(),
        }
    }
}

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
    fn from_file_raw(path: &Path) -> Result<Self> {
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

    /// Get the agent configuration for a given agent ID
    pub fn get_agent(&self, id: &str) -> Option<AgentConfig> {
        self.agent.get(id).map(|toml| {
            let mut mode_templates = HashMap::new();
            for (mode_name, mode_config) in &self.mode {
                if let Some(prompt) = &mode_config.prompt {
                    mode_templates.insert(
                        mode_name.clone(),
                        crate::ModeTemplate {
                            prompt_template: prompt.clone(),
                            system_prompt: mode_config.system_prompt.clone(),
                            default_agent: mode_config.agent.clone(),
                            session_mode: match mode_config.session_mode {
                                ModeSessionType::Oneshot => SessionMode::Oneshot,
                                ModeSessionType::Session => SessionMode::Session,
                            },
                            disallowed_tools: mode_config.disallowed_tools.clone(),
                            allowed_tools: mode_config.allowed_tools.clone(),
                            output_states: mode_config.output_states.clone(),
                            state_prompt: mode_config.state_prompt.clone(),
                        },
                    );
                }
            }

            let output_schema = if !self.settings.gui.output_schema.trim().is_empty() {
                Some(self.settings.gui.output_schema.clone())
            } else {
                None
            };

            let structured_output_schema =
                if !self.settings.gui.structured_output_schema.trim().is_empty() {
                    Some(self.settings.gui.structured_output_schema.clone())
                } else {
                    None
                };

            let sdk_type = toml.sdk;

            // Legacy: "print" and "repl" modes map to oneshot/session
            let session_mode = match toml.session_mode {
                SessionMode::Print | SessionMode::Oneshot => SessionMode::Oneshot,
                SessionMode::Session => SessionMode::Session,
                SessionMode::Repl => SessionMode::Session,
            };

            let permission_mode = sdk_type.default_permission_mode().to_string();

            AgentConfig {
                id: id.to_string(),
                sdk_type,
                session_mode,
                permission_mode,
                model: None,
                sandbox: None,
                max_turns: 0,
                system_prompt_mode: toml.system_prompt_mode,
                mode_templates,
                env: toml.env.clone(),
                disallowed_tools: toml.disallowed_tools.clone(),
                allowed_tools: toml.allowed_tools.clone(),
                mcp_servers: toml.mcp_servers.clone(),
                agents: toml.agents.clone(),
                plugins: if matches!(sdk_type, SdkType::Codex) {
                    Vec::new()
                } else {
                    self.settings.claude.allowed_plugin_paths.clone()
                },
                output_schema,
                structured_output_schema,
            }
        })
    }

    /// Get the agent ID for a given mode
    pub fn get_agent_for_mode(&self, mode: &str) -> String {
        self.mode
            .get(mode)
            .and_then(|m| m.agent.clone())
            .unwrap_or_else(|| "claude".to_string())
    }

    /// Get agent configuration with mode-specific tool overrides applied
    ///
    /// This merges the base agent config with mode-specific allowed/disallowed tools.
    /// Mode tools take precedence over agent tools when specified.
    pub fn get_agent_for_job(&self, agent_id: &str, mode: &str) -> Option<AgentConfig> {
        let mut agent_config = self.get_agent(agent_id)?;

        let derive_claude_permission = |disallowed: &[String]| -> String {
            let blocks_writes = disallowed.iter().any(|t| t == "Write" || t == "Edit");
            if blocks_writes {
                "default".to_string()
            } else {
                "acceptEdits".to_string()
            }
        };

        let derive_codex_sandbox = |disallowed: &[String]| -> String {
            let blocks_writes = disallowed.iter().any(|t| t == "Write" || t == "Edit");
            let blocks_bash = disallowed.iter().any(|t| t == "Bash");

            if blocks_writes || blocks_bash {
                "read-only".to_string()
            } else {
                "workspace-write".to_string()
            }
        };

        if let Some(mode_config) = self.mode.get(mode) {
            if !mode_config.allowed_tools.is_empty() {
                agent_config.allowed_tools = mode_config.allowed_tools.clone();
            }

            if !mode_config.disallowed_tools.is_empty() {
                for tool in &mode_config.disallowed_tools {
                    if !agent_config.disallowed_tools.contains(tool) {
                        agent_config.disallowed_tools.push(tool.clone());
                    }
                }
            }

            if !matches!(agent_config.session_mode, SessionMode::Repl) {
                agent_config.session_mode = match mode_config.session_mode {
                    ModeSessionType::Oneshot => SessionMode::Oneshot,
                    ModeSessionType::Session => SessionMode::Session,
                };
            }
            agent_config.max_turns = mode_config.max_turns;
            agent_config.model = mode_config.model.clone();

            match agent_config.sdk_type {
                SdkType::Codex => {
                    agent_config.sandbox = Some(
                        mode_config
                            .codex
                            .as_ref()
                            .map(|c| c.sandbox.clone())
                            .unwrap_or_else(|| {
                                derive_codex_sandbox(&agent_config.disallowed_tools)
                            }),
                    );
                }
                _ => {
                    agent_config.permission_mode = mode_config
                        .claude
                        .as_ref()
                        .map(|c| c.permission_mode.clone())
                        .unwrap_or_else(|| {
                            derive_claude_permission(&agent_config.disallowed_tools)
                        });
                }
            }
        }

        Some(agent_config)
    }

    /// Get mode configuration
    pub fn get_mode(&self, mode: &str) -> Option<&ModeConfig> {
        self.mode.get(mode)
    }

    /// Get scope configuration
    pub fn get_scope(&self, scope: &str) -> Option<&ScopeConfig> {
        self.scope.get(scope)
    }

    /// Get target configuration
    pub fn get_target(&self, target: &str) -> Option<&TargetConfig> {
        self.target.get(target)
    }

    /// Get chain configuration
    pub fn get_chain(&self, name: &str) -> Option<&ModeChain> {
        self.chain.get(name)
    }

    /// Check if a mode name is actually a chain
    pub fn is_chain(&self, name: &str) -> bool {
        self.chain.contains_key(name)
    }

    /// Get mode or chain - returns ModeOrChain enum
    pub fn get_mode_or_chain(&self, name: &str) -> Option<ModeOrChain> {
        if let Some(chain) = self.chain.get(name) {
            Some(ModeOrChain::Chain(chain.clone()))
        } else if let Some(mode) = self.mode.get(name) {
            Some(ModeOrChain::Mode(mode.clone()))
        } else {
            None
        }
    }

    /// Build prompt for a job using mode, target, and scope configs
    pub fn build_prompt(
        &self,
        mode: &str,
        target: &str,
        scope: &str,
        file: &str,
        description: &str,
    ) -> String {
        let mode_config = self.mode.get(mode);

        let template = mode_config
            .and_then(|m| m.prompt.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Execute '{mode}' on {target} in {scope} of `{file}`. {description}");

        let target_text = self
            .target
            .get(target)
            .and_then(|t| t.prompt_text.as_ref())
            .map(|s| s.as_str())
            .unwrap_or(target);

        let scope_text = self
            .scope
            .get(scope)
            .and_then(|s| s.prompt_text.as_ref())
            .map(|s| s.as_str())
            .unwrap_or(scope);

        template
            .replace("{mode}", mode)
            .replace("{target}", target_text)
            .replace("{scope}", scope_text)
            .replace("{file}", file)
            .replace("{description}", description)
    }

    /// Merge internal defaults into this config using versioned merging.
    ///
    /// Returns true if any changes were made (new items added or upgraded).
    pub fn merge_internal_defaults(&mut self) -> bool {
        let internal = match InternalDefaults::load() {
            Ok(defaults) => defaults,
            Err(e) => {
                tracing::error!("Failed to parse internal defaults: {}", e);
                return false;
            }
        };

        // Track sizes before merge to detect changes
        let agents_before = self.agent.len();
        let modes_before = self.mode.len();
        let chains_before = self.chain.len();

        // Also need to check if any versions were upgraded
        let mut version_changes = false;

        // Check for version upgrades before merge
        for (name, internal_agent) in &internal.agent {
            if let Some(existing) = self.agent.get(name) {
                if internal_agent.version > existing.version {
                    version_changes = true;
                    break;
                }
            }
        }
        if !version_changes {
            for (name, internal_mode) in &internal.mode {
                if let Some(existing) = self.mode.get(name) {
                    if internal_mode.version > existing.version {
                        version_changes = true;
                        break;
                    }
                }
            }
        }
        if !version_changes {
            for (name, internal_chain) in &internal.chain {
                if let Some(existing) = self.chain.get(name) {
                    if internal_chain.version > existing.version {
                        version_changes = true;
                        break;
                    }
                }
            }
        }

        // Perform the merge
        internal.merge_into(&mut self.agent, &mut self.mode, &mut self.chain);

        // Check if anything changed
        let size_changes = self.agent.len() != agents_before
            || self.mode.len() != modes_before
            || self.chain.len() != chains_before;

        size_changes || version_changes
    }
}

/// Generate a random token for authenticating IDE extension requests.
///
/// The token is hex-encoded and safe to embed in `config.toml`.
pub fn generate_http_token() -> String {
    let mut bytes = [0u8; 32];
    if getrandom::getrandom(&mut bytes).is_ok() {
        return hex_encode(&bytes);
    }

    // Fallback: best-effort token if OS RNG is unavailable.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id() as u128;
    let mixed = nanos ^ (pid.rotate_left(17));
    hex_encode(&mixed.to_le_bytes())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
