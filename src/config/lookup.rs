//! Configuration lookup and accessor methods

use std::collections::HashMap;

use super::chain::{ModeChain, ModeOrChain};
use super::mode::{ModeConfig, ModeSessionType};
use super::scope::ScopeConfig;
use super::target::TargetConfig;
use super::Config;
use crate::{AgentConfig, SdkType, SessionMode};

impl Config {
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
                model: toml.model.clone(),
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
            // Mode model overrides agent model only if explicitly set
            if mode_config.model.is_some() {
                agent_config.model = mode_config.model.clone();
            }

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
}
