//! Configuration lookup and accessor methods

use std::borrow::Cow;
use std::collections::HashMap;

use super::chain::{ModeChain, ModeOrChainRef};
use super::mode::ModeConfig;
use super::scope::ScopeConfig;
use super::skill::SkillConfig;
use super::target::TargetConfig;
use super::Config;
use crate::{AgentConfig, SdkType};

/// Either a skill or a chain (borrowed)
#[derive(Debug, Clone, Copy)]
pub enum SkillOrChainRef<'a> {
    Skill(&'a SkillConfig),
    Chain(&'a ModeChain),
}

impl Config {
    /// Get the agent configuration for a given agent ID
    pub fn get_agent(&self, id: &str) -> Option<AgentConfig> {
        self.agent.get(id).map(|toml| {
            // Build skill templates from both modes (legacy) and skills (new)
            let mut skill_templates = HashMap::new();

            // First, add templates from legacy modes (if any)
            for (mode_name, mode_config) in &self.mode {
                if let Some(prompt) = &mode_config.prompt {
                    skill_templates.insert(
                        mode_name.clone(),
                        crate::SkillTemplate {
                            prompt_template: prompt.clone(),
                            system_prompt: mode_config.system_prompt.clone(),
                            default_agent: mode_config.agent.clone(),
                            disallowed_tools: mode_config.disallowed_tools.clone(),
                            allowed_tools: mode_config.allowed_tools.clone(),
                            output_states: mode_config.output_states.clone(),
                            state_prompt: mode_config.state_prompt.clone(),
                        },
                    );
                }
            }

            // Then, add templates from skills (overrides modes with same name)
            for (skill_name, skill_config) in &self.skill {
                skill_templates.insert(
                    skill_name.clone(),
                    crate::SkillTemplate {
                        prompt_template: skill_config.get_prompt_template().to_string(),
                        system_prompt: skill_config.get_system_prompt().map(|s| s.to_string()),
                        default_agent: skill_config.kyco.agent.clone(),
                        disallowed_tools: skill_config.kyco.disallowed_tools.clone(),
                        allowed_tools: skill_config.kyco.allowed_tools.clone(),
                        output_states: skill_config.kyco.output_states.clone(),
                        state_prompt: skill_config.kyco.state_prompt.clone(),
                    },
                );
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
            let permission_mode = sdk_type.default_permission_mode().to_string();

            AgentConfig {
                id: id.to_string(),
                sdk_type,
                permission_mode,
                model: toml.model.clone(),
                sandbox: None,
                max_turns: 0,
                system_prompt_mode: toml.system_prompt_mode,
                skill_templates,
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

    /// Get the agent ID for a given mode/skill
    ///
    /// Returns a `Cow<str>` to avoid allocation when the mode/skill has an agent configured.
    /// Checks modes first, then falls back to skills.
    pub fn get_agent_for_mode(&self, mode: &str) -> Cow<'_, str> {
        // Check modes first (legacy)
        if let Some(agent) = self.mode.get(mode).and_then(|m| m.agent.as_deref()) {
            return Cow::Borrowed(agent);
        }
        // Fall back to skills (new filesystem-based)
        if let Some(agent) = self.skill.get(mode).and_then(|s| s.kyco.agent.as_deref()) {
            return Cow::Borrowed(agent);
        }
        Cow::Borrowed("claude")
    }

    /// Get agent configuration with mode/skill-specific tool overrides applied
    ///
    /// This merges the base agent config with mode/skill-specific allowed/disallowed tools.
    /// Mode/skill tools take precedence over agent tools when specified.
    /// Checks modes first, then falls back to skills.
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

        // Check modes first (legacy)
        if let Some(mode_config) = self.mode.get(mode) {
            if !mode_config.allowed_tools.is_empty() {
                agent_config.allowed_tools.clone_from(&mode_config.allowed_tools);
            }

            if !mode_config.disallowed_tools.is_empty() {
                agent_config
                    .disallowed_tools
                    .reserve(mode_config.disallowed_tools.len());
                for tool in &mode_config.disallowed_tools {
                    if !agent_config.disallowed_tools.contains(tool) {
                        agent_config.disallowed_tools.push(tool.clone());
                    }
                }
            }

            agent_config.max_turns = mode_config.max_turns;
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
        } else if let Some(skill_config) = self.skill.get(mode) {
            // Fall back to skills (new filesystem-based)
            if !skill_config.kyco.allowed_tools.is_empty() {
                agent_config.allowed_tools.clone_from(&skill_config.kyco.allowed_tools);
            }

            if !skill_config.kyco.disallowed_tools.is_empty() {
                agent_config
                    .disallowed_tools
                    .reserve(skill_config.kyco.disallowed_tools.len());
                for tool in &skill_config.kyco.disallowed_tools {
                    if !agent_config.disallowed_tools.contains(tool) {
                        agent_config.disallowed_tools.push(tool.clone());
                    }
                }
            }

            agent_config.max_turns = skill_config.kyco.max_turns;
            if skill_config.kyco.model.is_some() {
                agent_config.model = skill_config.kyco.model.clone();
            }

            match agent_config.sdk_type {
                SdkType::Codex => {
                    agent_config.sandbox = Some(skill_config.get_codex_sandbox());
                }
                _ => {
                    agent_config.permission_mode = skill_config.get_claude_permission();
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

    /// Get mode or chain - returns a reference to avoid cloning
    pub fn get_mode_or_chain(&self, name: &str) -> Option<ModeOrChainRef<'_>> {
        if let Some(chain) = self.chain.get(name) {
            Some(ModeOrChainRef::Chain(chain))
        } else if let Some(mode) = self.mode.get(name) {
            Some(ModeOrChainRef::Mode(mode))
        } else {
            None
        }
    }

    /// Build prompt for a job using mode/skill, target, and scope configs
    ///
    /// Checks modes first, then falls back to skills.
    pub fn build_prompt(
        &self,
        mode: &str,
        target: &str,
        scope: &str,
        file: &str,
        description: &str,
    ) -> String {
        // Check modes first (legacy), then skills (new filesystem-based)
        let template = if let Some(mode_config) = self.mode.get(mode) {
            mode_config
                .prompt
                .as_ref()
                .map(|s| Cow::Borrowed(s.as_str()))
                .unwrap_or(Cow::Borrowed(
                    "Execute '{mode}' on {target} in {scope} of `{file}`. {description}",
                ))
        } else if let Some(skill_config) = self.skill.get(mode) {
            Cow::Borrowed(skill_config.get_prompt_template())
        } else {
            Cow::Borrowed("Execute '{mode}' on {target} in {scope} of `{file}`. {description}")
        };

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
            .replace("{skill}", mode) // Support both {mode} and {skill} placeholders
            .replace("{target}", target_text)
            .replace("{scope}", scope_text)
            .replace("{file}", file)
            .replace("{description}", description)
    }

    // ========================================================================
    // Skill lookup methods (new skill system)
    // ========================================================================

    /// Get skill configuration by name
    pub fn get_skill(&self, skill: &str) -> Option<&SkillConfig> {
        self.skill.get(skill)
    }

    /// Get the agent ID for a given skill
    ///
    /// Returns a `Cow<str>` to avoid allocation when the skill has an agent configured.
    pub fn get_agent_for_skill(&self, skill: &str) -> Cow<'_, str> {
        self.skill
            .get(skill)
            .and_then(|s| s.kyco.agent.as_deref())
            .map(Cow::Borrowed)
            .unwrap_or(Cow::Borrowed("claude"))
    }

    /// Get agent configuration with skill-specific tool overrides applied
    ///
    /// This merges the base agent config with skill-specific allowed/disallowed tools.
    /// Skill tools take precedence over agent tools when specified.
    pub fn get_agent_for_skill_job(&self, agent_id: &str, skill: &str) -> Option<AgentConfig> {
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

        if let Some(skill_config) = self.skill.get(skill) {
            if !skill_config.kyco.allowed_tools.is_empty() {
                agent_config
                    .allowed_tools
                    .clone_from(&skill_config.kyco.allowed_tools);
            }

            if !skill_config.kyco.disallowed_tools.is_empty() {
                agent_config
                    .disallowed_tools
                    .reserve(skill_config.kyco.disallowed_tools.len());
                for tool in &skill_config.kyco.disallowed_tools {
                    if !agent_config.disallowed_tools.contains(tool) {
                        agent_config.disallowed_tools.push(tool.clone());
                    }
                }
            }

            agent_config.max_turns = skill_config.kyco.max_turns;
            if skill_config.kyco.model.is_some() {
                agent_config.model = skill_config.kyco.model.clone();
            }

            match agent_config.sdk_type {
                SdkType::Codex => {
                    agent_config.sandbox = Some(
                        skill_config
                            .kyco
                            .codex
                            .as_ref()
                            .map(|c| c.sandbox.clone())
                            .unwrap_or_else(|| {
                                derive_codex_sandbox(&agent_config.disallowed_tools)
                            }),
                    );
                }
                _ => {
                    agent_config.permission_mode = skill_config
                        .kyco
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

    /// Get skill or chain - returns a reference to avoid cloning
    ///
    /// Checks chains first, then skills.
    pub fn get_skill_or_chain(&self, name: &str) -> Option<SkillOrChainRef<'_>> {
        if let Some(chain) = self.chain.get(name) {
            Some(SkillOrChainRef::Chain(chain))
        } else if let Some(skill) = self.skill.get(name) {
            Some(SkillOrChainRef::Skill(skill))
        } else {
            None
        }
    }

    /// Build prompt for a job using skill instructions
    ///
    /// Template placeholders:
    /// - {target} - what to process (from target config)
    /// - {scope} - the scope description
    /// - {file} - the source file path
    /// - {description} - user's description
    /// - {skill} - the skill name
    /// - {ide_context} - IDE context injection point (replaced later)
    pub fn build_skill_prompt(
        &self,
        skill: &str,
        target: &str,
        scope: &str,
        file: &str,
        description: &str,
    ) -> String {
        let skill_config = self.skill.get(skill);

        let template = skill_config
            .map(|s| s.get_prompt_template())
            .unwrap_or("Execute '{skill}' on {target} in {scope} of `{file}`. {description}");

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
            .replace("{skill}", skill)
            .replace("{target}", target_text)
            .replace("{scope}", scope_text)
            .replace("{file}", file)
            .replace("{description}", description)
    }

    /// Get the system prompt for a skill
    pub fn get_skill_system_prompt(&self, skill: &str) -> Option<&str> {
        self.skill.get(skill).and_then(|s| s.get_system_prompt())
    }
}
