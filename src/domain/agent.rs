use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The type of CLI being used (determines output parsing and command structure)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CliType {
    /// Claude Code CLI
    #[default]
    Claude,
    /// OpenAI Codex CLI
    Codex,
    /// Google Gemini CLI
    Gemini,
    /// Custom CLI (uses generic parsing)
    Custom,
}

impl CliType {
    /// Returns the default binary/executable name for this CLI type.
    ///
    /// This is used when constructing commands to invoke the underlying AI CLI tool.
    ///
    /// # Returns
    /// - `Claude` -> `"claude"`
    /// - `Codex` -> `"codex"`
    /// - `Gemini` -> `"gemini"`
    /// - `Custom` -> `"agent"`
    pub fn default_binary(&self) -> &'static str {
        match self {
            CliType::Claude => "claude",
            CliType::Codex => "codex",
            CliType::Gemini => "gemini",
            CliType::Custom => "agent",
        }
    }

    /// Get the default print mode arguments for this CLI type
    pub fn default_print_args(&self) -> Vec<String> {
        match self {
            CliType::Claude => vec!["-p".to_string()],
            CliType::Codex => vec!["exec".to_string()],
            CliType::Gemini => vec![], // Gemini doesn't have a standard print mode
            CliType::Custom => vec![],
        }
    }

    /// Get the default output format arguments for this CLI type
    pub fn default_output_format_args(&self) -> Vec<String> {
        match self {
            CliType::Claude => vec!["--output-format".to_string(), "stream-json".to_string(), "--verbose".to_string()],
            CliType::Codex => vec!["--json".to_string()],
            CliType::Gemini => vec![],
            CliType::Custom => vec![],
        }
    }
}

/// How to handle the system prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemPromptMode {
    /// Append to the default system prompt
    #[default]
    Append,
    /// Replace the default system prompt entirely
    Replace,
    /// Use config override (for Codex-style CLIs)
    ConfigOverride,
}

/// Agent execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    /// Print mode - non-interactive, structured JSON output
    /// Uses print_mode_args + output_format_args
    #[default]
    Print,
    /// REPL mode - interactive PTY session
    /// Uses repl_mode_args, streams terminal output
    Repl,
}

/// Configuration for a specific mode's agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeTemplate {
    /// The prompt template for this mode
    pub prompt_template: String,

    /// Additional arguments to pass to the agent for this mode
    #[serde(default)]
    pub extra_args: Vec<String>,

    /// System prompt additions for this mode
    pub system_prompt: Option<String>,
}

/// Configuration for an agent (e.g., Claude Code)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique identifier (e.g., "claude")
    pub id: String,

    /// The type of CLI (determines output parsing and command structure)
    #[serde(default)]
    pub cli_type: CliType,

    /// Execution mode (print or repl)
    #[serde(default)]
    pub mode: AgentMode,

    /// The binary to execute (e.g., "claude")
    pub binary: String,

    /// Arguments for print/non-interactive mode (e.g., ["-p"] for Claude, ["exec"] for Codex)
    #[serde(default)]
    pub print_mode_args: Vec<String>,

    /// Arguments for output format (e.g., ["--output-format", "stream-json"] for Claude)
    #[serde(default)]
    pub output_format_args: Vec<String>,

    /// Arguments for REPL/interactive mode (no -p, just the prompt)
    #[serde(default)]
    pub repl_mode_args: Vec<String>,

    /// Default arguments to pass to the binary (legacy, prefer print_mode_args + output_format_args)
    #[serde(default)]
    pub default_args: Vec<String>,

    /// How to handle system prompts
    #[serde(default)]
    pub system_prompt_mode: SystemPromptMode,

    /// Mode-specific templates
    #[serde(default)]
    pub mode_templates: HashMap<String, ModeTemplate>,

    /// Environment variables to set
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Tools to disallow (e.g., ["Bash(git commit)", "Bash(git push)"])
    #[serde(default)]
    pub disallowed_tools: Vec<String>,

    /// Tools to explicitly allow (if empty, all tools are allowed)
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Output schema to append to system prompt (for structured GUI output)
    #[serde(default)]
    pub output_schema: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self::claude_default()
    }
}

impl AgentConfig {
    /// Create a default Claude Code agent configuration
    pub fn claude_default() -> Self {
        Self {
            id: "claude".to_string(),
            cli_type: CliType::Claude,
            mode: AgentMode::Print,
            binary: "claude".to_string(),
            print_mode_args: vec!["-p".to_string(), "--permission-mode".to_string(), "bypassPermissions".to_string()],
            output_format_args: vec!["--output-format".to_string(), "stream-json".to_string(), "--verbose".to_string()],
            repl_mode_args: vec!["--permission-mode".to_string(), "bypassPermissions".to_string()],
            default_args: vec![],
            system_prompt_mode: SystemPromptMode::Append,
            mode_templates: Self::default_mode_templates(),
            env: HashMap::new(),
            disallowed_tools: vec![],
            allowed_tools: Vec::new(),
            output_schema: None,
        }
    }

    /// Create a default Codex CLI agent configuration
    pub fn codex_default() -> Self {
        Self {
            id: "codex".to_string(),
            cli_type: CliType::Codex,
            mode: AgentMode::Print,
            binary: "codex".to_string(),
            print_mode_args: vec!["exec".to_string()],
            output_format_args: vec!["--json".to_string()],
            repl_mode_args: vec!["--full-auto".to_string()],
            default_args: vec![],
            system_prompt_mode: SystemPromptMode::ConfigOverride,
            mode_templates: Self::default_mode_templates(),
            env: HashMap::new(),
            disallowed_tools: vec![],
            allowed_tools: Vec::new(),
            output_schema: None,
        }
    }

    /// Create a default Gemini CLI agent configuration
    pub fn gemini_default() -> Self {
        Self {
            id: "gemini".to_string(),
            cli_type: CliType::Gemini,
            mode: AgentMode::Print,
            binary: "gemini".to_string(),
            print_mode_args: vec![],
            output_format_args: vec![],
            repl_mode_args: vec![],
            default_args: vec![],
            system_prompt_mode: SystemPromptMode::Replace, // Uses GEMINI.md files
            mode_templates: Self::default_mode_templates(),
            env: HashMap::new(),
            disallowed_tools: vec![],
            allowed_tools: Vec::new(),
            output_schema: None,
        }
    }

    /// Get the effective arguments for running this agent in print mode
    pub fn get_run_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Add print mode args first
        args.extend(self.print_mode_args.clone());

        // Add output format args
        args.extend(self.output_format_args.clone());

        // Add any legacy default args
        args.extend(self.default_args.clone());

        args
    }

    /// Get the effective arguments for running this agent in REPL mode
    pub fn get_repl_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Add REPL mode args
        args.extend(self.repl_mode_args.clone());

        // Add any legacy default args
        args.extend(self.default_args.clone());

        args
    }

    /// Default mode templates
    fn default_mode_templates() -> HashMap<String, ModeTemplate> {
        let mut templates = HashMap::new();

        templates.insert(
            "refactor".to_string(),
            ModeTemplate {
                prompt_template: "Refactor the {scope_type} `{target}` in `{file}`. {description}"
                    .to_string(),
                extra_args: vec![],
                system_prompt: Some(
                    "You are running in KYCo 'refactor' mode. You may read the entire repo. \
                     Make code changes only within the marked scope. Write idiomatic code. \
                     Do not change function signatures unless explicitly requested."
                        .to_string(),
                ),
            },
        );

        templates.insert(
            "tests".to_string(),
            ModeTemplate {
                prompt_template:
                    "Write unit tests for {scope_type} `{target}` in `{file}`. {description}"
                        .to_string(),
                extra_args: vec![],
                system_prompt: Some(
                    "You are running in KYCo 'tests' mode. You may read the entire repo. \
                     Write comprehensive unit tests. Use the existing test framework and patterns \
                     found in the codebase."
                        .to_string(),
                ),
            },
        );

        templates.insert(
            "docs".to_string(),
            ModeTemplate {
                prompt_template:
                    "Write documentation for {scope_type} `{target}` in `{file}`. {description}"
                        .to_string(),
                extra_args: vec![],
                system_prompt: Some(
                    "You are running in KYCo 'docs' mode. You may read the entire repo. \
                     Write clear, concise documentation. Follow existing documentation patterns \
                     in the codebase."
                        .to_string(),
                ),
            },
        );

        templates.insert(
            "review".to_string(),
            ModeTemplate {
                prompt_template:
                    "Review {scope_type} `{target}` in `{file}`. {description}".to_string(),
                extra_args: vec![],
                system_prompt: Some(
                    "You are running in KYCo 'review' mode. You may read the entire repo. \
                     Analyze the code for bugs, performance issues, and code quality. \
                     Suggest improvements but do not make changes unless explicitly asked."
                        .to_string(),
                ),
            },
        );

        templates
    }

    /// Get the mode template for a given mode, falling back to a generic template
    pub fn get_mode_template(&self, mode: &str) -> ModeTemplate {
        self.mode_templates.get(mode).cloned().unwrap_or_else(|| {
            ModeTemplate {
                prompt_template: "Execute '{mode}' on {scope_type} `{target}` in `{file}`. {description}".to_string(),
                extra_args: vec![],
                system_prompt: Some(format!(
                    "You are running in KYCo '{mode}' mode. You may read the entire repo. \
                     Make changes only within the marked scope.",
                    mode = mode
                )),
            }
        })
    }
}
