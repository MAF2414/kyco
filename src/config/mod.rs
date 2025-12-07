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
pub use settings::{GuiSettings, RegistrySettings, Settings, VoiceSettings};
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
    /// If no config exists, auto-creates one with defaults
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

        // Auto-init: create config with defaults
        Self::auto_init(dir)?;
        Self::from_file(&new_path)
    }

    /// Auto-initialize configuration when no config exists
    fn auto_init(dir: &Path) -> Result<()> {
        let config_dir = dir.join(".kyco");
        let config_path = config_dir.join("config.toml");

        // Create .kyco directory
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)
                .with_context(|| format!("Failed to create config directory: {}", config_dir.display()))?;
        }

        // Generate default config as TOML
        let default_config = Self::with_defaults();
        let config_content = toml::to_string_pretty(&default_config)
            .with_context(|| "Failed to serialize default config")?;

        // Write config file
        std::fs::write(&config_path, config_content)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

        eprintln!("Created {}", config_path.display());
        Ok(())
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
                    "--allowedTools".to_string(),
                    "Read".to_string(),
                ],
                output_format_args: vec![
                    "--output-format".to_string(),
                    "stream-json".to_string(),
                    "--verbose".to_string(),
                ],
                repl_mode_args: vec![
                    "--allowedTools".to_string(),
                    "Read".to_string(),
                ],
                default_args: vec![],
                system_prompt_mode: SystemPromptMode::Append,
                disallowed_tools: vec![],
                allowed_tools: vec!["Read".to_string()],
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
                print_mode_args: vec!["exec".to_string(), "--sandbox".to_string(), "workspace-write".to_string()],
                output_format_args: vec!["--json".to_string()],
                repl_mode_args: vec!["--full-auto".to_string()], // --full-auto includes workspace-write sandbox
                default_args: vec![],
                system_prompt_mode: SystemPromptMode::ConfigOverride,
                disallowed_tools: vec![],
                allowed_tools: vec!["Read".to_string()],
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
                allowed_tools: vec!["Read".to_string()],
                env: HashMap::new(),
            },
        );

        // Add default modes
        config.mode.insert(
            "review".to_string(),
            ModeConfig {
                agent: None,
                target_default: Some("code".to_string()),
                scope_default: Some("file".to_string()),
                prompt: Some("Review the code in `{file}` for issues, bugs, and improvements. {description}".to_string()),
                system_prompt: Some("You are a code reviewer. Focus on bugs, security issues, performance problems, and code quality. Be thorough but constructive.".to_string()),
                allowed_tools: vec!["Read".to_string(), "Glob".to_string(), "Grep".to_string()],
                disallowed_tools: vec!["Write".to_string(), "Edit".to_string()],
                aliases: vec!["r".to_string(), "rev".to_string()],
                output_states: vec!["issues_found".to_string(), "no_issues".to_string()],
            },
        );

        config.mode.insert(
            "fix".to_string(),
            ModeConfig {
                agent: None,
                target_default: Some("code".to_string()),
                scope_default: Some("file".to_string()),
                prompt: Some("Fix the issues in `{file}`. {description}".to_string()),
                system_prompt: Some("You are a code fixer. Apply the necessary fixes based on the issues identified. Make minimal changes to fix the problems.".to_string()),
                allowed_tools: vec!["Read".to_string(), "Write".to_string(), "Edit".to_string(), "Glob".to_string(), "Grep".to_string(), "Bash".to_string()],
                disallowed_tools: vec![],
                aliases: vec!["f".to_string()],
                output_states: vec!["fixed".to_string(), "unfixable".to_string()],
            },
        );

        config.mode.insert(
            "implement".to_string(),
            ModeConfig {
                agent: None,
                target_default: Some("code".to_string()),
                scope_default: Some("file".to_string()),
                prompt: Some("Implement the following in `{file}`: {description}".to_string()),
                system_prompt: Some("You are a software engineer. Implement the requested feature or functionality.".to_string()),
                allowed_tools: vec!["Read".to_string(), "Write".to_string(), "Edit".to_string(), "Glob".to_string(), "Grep".to_string(), "Bash".to_string()],
                disallowed_tools: vec![],
                aliases: vec!["i".to_string(), "impl".to_string()],
                output_states: vec!["implemented".to_string(), "blocked".to_string()],
            },
        );

        // Add default chain: review+fix
        config.chain.insert(
            "review+fix".to_string(),
            ModeChain {
                description: Some("Review code and fix any issues found".to_string()),
                steps: vec![
                    ChainStep {
                        mode: "review".to_string(),
                        trigger_on: None,
                        skip_on: None,
                        agent: None,
                        inject_context: None,
                    },
                    ChainStep {
                        mode: "fix".to_string(),
                        trigger_on: Some(vec!["issues_found".to_string()]),
                        skip_on: Some(vec!["no_issues".to_string()]),
                        agent: None,
                        inject_context: None,
                    },
                ],
                stop_on_failure: true,
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

    /// Get agent configuration with mode-specific tool overrides applied
    ///
    /// This merges the base agent config with mode-specific allowed/disallowed tools.
    /// Mode tools take precedence over agent tools when specified.
    pub fn get_agent_for_job(&self, agent_id: &str, mode: &str) -> Option<AgentConfig> {
        let mut agent_config = self.get_agent(agent_id)?;

        // Get mode config and apply tool overrides
        if let Some(mode_config) = self.mode.get(mode) {
            // Mode allowed_tools override agent allowed_tools when specified
            if !mode_config.allowed_tools.is_empty() {
                agent_config.allowed_tools = mode_config.allowed_tools.clone();
            }

            // Mode disallowed_tools are added to agent disallowed_tools
            if !mode_config.disallowed_tools.is_empty() {
                // Combine both, avoiding duplicates
                for tool in &mode_config.disallowed_tools {
                    if !agent_config.disallowed_tools.contains(tool) {
                        agent_config.disallowed_tools.push(tool.clone());
                    }
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
