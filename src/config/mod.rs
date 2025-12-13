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
pub use chain::{ChainStep, ModeChain, ModeOrChain, StateDefinition};
pub use mode::{ClaudeModeOptions, CodexModeOptions, ModeConfig, ModeSessionType};
pub use scope::ScopeConfig;
pub use settings::{GuiSettings, RegistrySettings, Settings, VoiceSettings};
pub use target::TargetConfig;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{AgentConfig, SessionMode, SdkType, SystemPromptMode};

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

    /// Load configuration from a file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Load global configuration from ~/.kyco/config.toml
    /// If no config exists, auto-creates one with defaults
    pub fn load() -> Result<Self> {
        let global_path = Self::global_config_path();

        if global_path.exists() {
            return Self::from_file(&global_path);
        }

        // Auto-init: create global config with defaults
        Self::auto_init()?;
        Self::from_file(&global_path)
    }

    /// Load configuration from a directory (legacy compatibility)
    /// Now just loads the global config, ignoring the directory parameter
    pub fn from_dir(_dir: &Path) -> Result<Self> {
        Self::load()
    }

    /// Auto-initialize global configuration when no config exists
    fn auto_init() -> Result<()> {
        let config_dir = Self::global_config_dir();
        let config_path = Self::global_config_path();

        // Create ~/.kyco directory
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)
                .with_context(|| format!("Failed to create config directory: {}", config_dir.display()))?;
        }

        // Generate default config as TOML
        // Note: http_token is intentionally left empty for local development.
        // Auth is only enforced when http_token is explicitly set.
        let default_config = Self::with_defaults();
        let config_content = toml::to_string_pretty(&default_config)
            .with_context(|| "Failed to serialize default config")?;

        // Write config file
        std::fs::write(&config_path, &config_content)
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
                sdk: SdkType::Claude,
                session_mode: SessionMode::Oneshot,
                binary: None,
                print_mode_args: vec![],
                output_format_args: vec![],
                repl_mode_args: vec![],
                default_args: vec![],
                system_prompt_mode: SystemPromptMode::Append,
                disallowed_tools: vec![],
                allowed_tools: vec![],
                env: HashMap::new(),
                mcp_servers: HashMap::new(),
                agents: HashMap::new(),
            },
        );

        // Add default Codex agent
        config.agent.insert(
            "codex".to_string(),
            AgentConfigToml {
                aliases: vec!["x".to_string(), "cx".to_string()],
                sdk: SdkType::Codex,
                session_mode: SessionMode::Oneshot,
                binary: None,
                print_mode_args: vec![],
                output_format_args: vec![],
                repl_mode_args: vec![],
                default_args: vec![],
                system_prompt_mode: SystemPromptMode::Append,
                disallowed_tools: vec![],
                allowed_tools: vec![],
                env: HashMap::new(),
                mcp_servers: HashMap::new(),
                agents: HashMap::new(),
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
                system_prompt: Some("You are a code reviewer. Focus on bugs, security issues, performance problems, and code quality. Be thorough but constructive. Do NOT modify any files.".to_string()),
                session_mode: ModeSessionType::Oneshot,
                max_turns: 10,
                model: None,
                disallowed_tools: vec!["Write".to_string(), "Edit".to_string(), "Bash".to_string()],
                claude: Some(ClaudeModeOptions {
                    permission_mode: "default".to_string(),
                }),
                codex: Some(CodexModeOptions {
                    sandbox: "read-only".to_string(),
                }),
                aliases: vec!["r".to_string(), "rev".to_string()],
                output_states: vec!["issues_found".to_string(), "no_issues".to_string()],
                state_prompt: None,
                allowed_tools: vec![], // Legacy, deprecated
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
                session_mode: ModeSessionType::Oneshot,
                max_turns: 30,
                model: None,
                disallowed_tools: vec!["Bash(git push)".to_string(), "Bash(git push --force)".to_string()],
                claude: Some(ClaudeModeOptions {
                    permission_mode: "acceptEdits".to_string(),
                }),
                codex: Some(CodexModeOptions {
                    sandbox: "workspace-write".to_string(),
                }),
                aliases: vec!["f".to_string()],
                output_states: vec!["fixed".to_string(), "unfixable".to_string()],
                state_prompt: None,
                allowed_tools: vec![], // Legacy, deprecated
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
                session_mode: ModeSessionType::Oneshot,
                max_turns: 50,
                model: None,
                disallowed_tools: vec!["Bash(git push)".to_string(), "Bash(git push --force)".to_string()],
                claude: Some(ClaudeModeOptions {
                    permission_mode: "acceptEdits".to_string(),
                }),
                codex: Some(CodexModeOptions {
                    sandbox: "workspace-write".to_string(),
                }),
                aliases: vec!["i".to_string(), "impl".to_string()],
                output_states: vec!["implemented".to_string(), "blocked".to_string()],
                state_prompt: None,
                allowed_tools: vec![], // Legacy, deprecated
            },
        );

        config.mode.insert(
            "plan".to_string(),
            ModeConfig {
                agent: None,
                target_default: Some("code".to_string()),
                scope_default: Some("file".to_string()),
                prompt: Some(
                    "Create a detailed implementation plan for `{file}`. {description}".to_string(),
                ),
                system_prompt: Some(
                    "You are in planning mode. Propose a concrete, step-by-step plan with relevant files/functions, risks, and how you would validate the change. Do NOT modify any files or run commands."
                        .to_string(),
                ),
                session_mode: ModeSessionType::Oneshot,
                max_turns: 15,
                model: None,
                disallowed_tools: vec!["Write".to_string(), "Edit".to_string(), "Bash".to_string()],
                claude: Some(ClaudeModeOptions {
                    permission_mode: "plan".to_string(),
                }),
                codex: Some(CodexModeOptions {
                    sandbox: "read-only".to_string(),
                }),
                aliases: vec!["p".to_string()],
                output_states: vec!["plan_ready".to_string(), "needs_clarification".to_string()],
                state_prompt: None,
                allowed_tools: vec![], // Legacy, deprecated
            },
        );

        // Chat mode - interactive session
        config.mode.insert(
            "chat".to_string(),
            ModeConfig {
                agent: None,
                target_default: None,
                scope_default: None,
                prompt: Some("{description}".to_string()),
                system_prompt: Some("You are a helpful assistant for this codebase. You can read and explore the code to answer questions.".to_string()),
                session_mode: ModeSessionType::Session, // Persistent conversation!
                max_turns: 0, // Unlimited
                model: None,
                disallowed_tools: vec!["Bash(git push)".to_string()],
                claude: Some(ClaudeModeOptions {
                    permission_mode: "default".to_string(), // Ask for edits
                }),
                codex: Some(CodexModeOptions {
                    sandbox: "workspace-write".to_string(),
                }),
                aliases: vec!["c".to_string()],
                output_states: vec![],
                state_prompt: None,
                allowed_tools: vec![], // Legacy, deprecated
            },
        );

        // Add default chain: review+fix
        config.chain.insert(
            "review+fix".to_string(),
            ModeChain {
                description: Some("Review code and fix any issues found".to_string()),
                states: vec![
                    StateDefinition {
                        id: "issues_found".to_string(),
                        description: Some("Issues were found in the code review".to_string()),
                        patterns: vec![
                            "issues found".to_string(),
                            "problems found".to_string(),
                            "bugs found".to_string(),
                            "needs fixing".to_string(),
                            "should be fixed".to_string(),
                        ],
                        is_regex: false,
                        case_insensitive: true,
                    },
                    StateDefinition {
                        id: "no_issues".to_string(),
                        description: Some("No issues were found".to_string()),
                        patterns: vec![
                            "no issues".to_string(),
                            "looks good".to_string(),
                            "code is clean".to_string(),
                            "no problems".to_string(),
                        ],
                        is_regex: false,
                        case_insensitive: true,
                    },
                ],
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
                pass_full_response: true,
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

            // Get output schema from GUI settings
            let output_schema = if !self.settings.gui.output_schema.trim().is_empty() {
                Some(self.settings.gui.output_schema.clone())
            } else {
                None
            };

            let structured_output_schema = if !self.settings.gui.structured_output_schema.trim().is_empty() {
                Some(self.settings.gui.structured_output_schema.clone())
            } else {
                None
            };

            // SDK type
            let sdk_type = toml.sdk;

            // Normalize session mode (legacy "print" == oneshot)
            let session_mode = match toml.session_mode {
                SessionMode::Print | SessionMode::Oneshot => SessionMode::Oneshot,
                SessionMode::Session => SessionMode::Session,
                // Legacy: "repl" used to spawn a separate terminal window. SDK sessions cover this use case.
                SessionMode::Repl => SessionMode::Session,
            };

            // Default permission mode based on SDK type
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
                // Legacy fields
                cli_type: Some(toml.sdk),
                mode: Some(toml.session_mode),
                binary: toml.binary.clone(),
                print_mode_args: toml.print_mode_args.clone(),
                output_format_args: toml.output_format_args.clone(),
                repl_mode_args: toml.repl_mode_args.clone(),
                default_args: toml.default_args.clone(),
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

            // Apply per-mode execution settings (but don't override Terminal REPL agents)
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
                            .unwrap_or_else(|| derive_codex_sandbox(&agent_config.disallowed_tools)),
                    );
                }
                _ => {
                    // Default to Claude-style permission modes for non-Codex SDKs
                    agent_config.permission_mode = mode_config
                        .claude
                        .as_ref()
                        .map(|c| c.permission_mode.clone())
                        .unwrap_or_else(|| derive_claude_permission(&agent_config.disallowed_tools));
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
