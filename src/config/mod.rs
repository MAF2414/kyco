//! Configuration loading and management

mod agent;
mod alias;
mod chain;
mod mode;
mod scope;
mod settings;
mod target;

pub use agent::AgentConfigToml;
pub use alias::AliasConfig;
pub use chain::{ChainStep, ModeChain, ModeOrChain};
pub use mode::ModeConfig;
pub use scope::ScopeConfig;
pub use settings::{GuiSettings, Settings, VoiceSettings};
pub use target::TargetConfig;

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{AgentConfig, AgentMode, CliType, SystemPromptMode};

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
    /// Load configuration from a file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Load configuration from a directory
    /// Looks for: .kyco/config.toml (preferred) or kyco.toml (legacy)
    pub fn from_dir(dir: &Path) -> Result<Self> {
        // Prefer new location: .kyco/config.toml
        let new_path = dir.join(".kyco/config.toml");
        if new_path.exists() {
            return Self::from_file(&new_path);
        }

        // Fallback to legacy: kyco.toml
        let legacy_path = dir.join("kyco.toml");
        if legacy_path.exists() {
            return Self::from_file(&legacy_path);
        }

        // Return default config
        Ok(Self::with_defaults())
    }

    /// Create a config with sensible defaults
    pub fn with_defaults() -> Self {
        let mut config = Self::default();

        // Add default Claude agent
        config.agent.insert(
            "claude".to_string(),
            AgentConfigToml {
                aliases: vec!["c".to_string(), "cl".to_string()],
                cli_type: CliType::Claude,
                mode: AgentMode::Print,
                binary: "claude".to_string(),
                print_mode_args: vec![
                    "-p".to_string(),
                    "--permission-mode".to_string(),
                    "bypassPermissions".to_string(),
                ],
                output_format_args: vec![
                    "--output-format".to_string(),
                    "stream-json".to_string(),
                    "--verbose".to_string(),
                ],
                repl_mode_args: vec![
                    "--permission-mode".to_string(),
                    "bypassPermissions".to_string(),
                ],
                default_args: vec![],
                system_prompt_mode: SystemPromptMode::Append,
                disallowed_tools: vec![],
                allowed_tools: vec![],
                env: HashMap::new(),
            },
        );

        // Add default Codex agent
        config.agent.insert(
            "codex".to_string(),
            AgentConfigToml {
                aliases: vec!["x".to_string(), "cx".to_string()],
                cli_type: CliType::Codex,
                mode: AgentMode::Print,
                binary: "codex".to_string(),
                print_mode_args: vec!["exec".to_string()],
                output_format_args: vec!["--json".to_string()],
                repl_mode_args: vec!["--full-auto".to_string()],
                default_args: vec![],
                system_prompt_mode: SystemPromptMode::ConfigOverride,
                disallowed_tools: vec![],
                allowed_tools: vec![],
                env: HashMap::new(),
            },
        );

        // Add default Gemini agent
        config.agent.insert(
            "gemini".to_string(),
            AgentConfigToml {
                aliases: vec!["g".to_string(), "gm".to_string()],
                cli_type: CliType::Gemini,
                mode: AgentMode::Print,
                binary: "gemini".to_string(),
                print_mode_args: vec![],
                output_format_args: vec![],
                repl_mode_args: vec![],
                default_args: vec![],
                system_prompt_mode: SystemPromptMode::Replace,
                disallowed_tools: vec![],
                allowed_tools: vec![],
                env: HashMap::new(),
            },
        );

        config
    }

    /// Get the agent configuration for a given agent ID
    pub fn get_agent(&self, id: &str) -> Option<AgentConfig> {
        self.agent.get(id).map(|toml| {
            // Build mode_templates from the mode config
            let mut mode_templates = HashMap::new();
            for (mode_name, mode_config) in &self.mode {
                if let Some(prompt) = &mode_config.prompt {
                    mode_templates.insert(
                        mode_name.clone(),
                        crate::ModeTemplate {
                            prompt_template: prompt.clone(),
                            extra_args: vec![],
                            system_prompt: mode_config.system_prompt.clone(),
                        },
                    );
                }
            }

            // Get output schema from GUI settings
            let output_schema = if !self.settings.gui.output_schema.trim().is_empty() {
                Some(self.settings.gui.output_schema.clone())
            } else {
                None
            };

            AgentConfig {
                id: id.to_string(),
                cli_type: toml.cli_type,
                mode: toml.mode,
                binary: toml.binary.clone(),
                print_mode_args: toml.print_mode_args.clone(),
                output_format_args: toml.output_format_args.clone(),
                repl_mode_args: toml.repl_mode_args.clone(),
                default_args: toml.default_args.clone(),
                system_prompt_mode: toml.system_prompt_mode,
                mode_templates,
                env: toml.env.clone(),
                disallowed_tools: toml.disallowed_tools.clone(),
                allowed_tools: toml.allowed_tools.clone(),
                output_schema,
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

        // Get prompt template from mode or use default
        let template = mode_config
            .and_then(|m| m.prompt.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Execute '{mode}' on {target} in {scope} of `{file}`. {description}");

        // Get target description
        let target_text = self
            .target
            .get(target)
            .and_then(|t| t.prompt_text.as_ref())
            .map(|s| s.as_str())
            .unwrap_or(target);

        // Get scope description
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
}
