//! Settings configuration types

use serde::{Deserialize, Serialize};

/// General settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Maximum concurrent jobs
    #[serde(default = "default_max_concurrent_jobs")]
    pub max_concurrent_jobs: usize,

    /// Automatically run new jobs when found (no manual confirmation)
    #[serde(default = "default_auto_run")]
    pub auto_run: bool,

    /// Use Git worktrees for job isolation
    /// When true, each job runs in a separate Git worktree
    /// When false (default), jobs run in the main working directory
    #[serde(default = "default_use_worktree")]
    pub use_worktree: bool,

    /// Maximum concurrent jobs per file (only applies when use_worktree = false)
    /// When set to 1 (default), only one job can run on a file at a time.
    /// This prevents agents from overwriting each other's changes.
    /// Higher values allow parallel edits but risk lost changes.
    #[serde(default = "default_max_jobs_per_file")]
    pub max_jobs_per_file: usize,

    /// GUI settings
    #[serde(default)]
    pub gui: GuiSettings,

    /// Registry settings for agent adapters
    #[serde(default)]
    pub registry: RegistrySettings,

    /// Claude-specific settings
    #[serde(default)]
    pub claude: ClaudeSettings,
}

/// Claude-specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClaudeSettings {
    /// Allowlisted local plugin paths to load into Claude Agent SDK sessions.
    ///
    /// Security note: plugins are Node.js code that runs inside the KYCO bridge process.
    /// Only load plugins you trust, and keep this list as small as possible.
    #[serde(default)]
    pub allowed_plugin_paths: Vec<String>,
}

/// GUI-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSettings {
    /// Global hotkey to trigger the popup
    /// Format: "modifier+modifier+key" (e.g., "cmd+shift+k", "ctrl+shift+space")
    /// Modifiers: cmd/super, ctrl, alt, shift
    /// Default: "cmd+shift+k" on macOS, "ctrl+shift+k" on Windows/Linux
    #[serde(default = "default_gui_hotkey")]
    pub hotkey: String,

    /// Default agent for GUI tasks
    #[serde(default = "default_gui_agent")]
    pub default_agent: String,

    /// Default mode for GUI tasks
    #[serde(default = "default_gui_mode")]
    pub default_mode: String,

    /// Output schema that agents should include in their response
    /// This YAML block helps structure the output for better GUI display
    #[serde(default = "default_output_schema")]
    pub output_schema: String,

    /// Optional JSON Schema used for SDK structured output.
    ///
    /// When set, Claude and Codex will be asked to produce JSON matching this schema.
    /// Leave empty to keep the YAML summary footer behavior.
    #[serde(default)]
    pub structured_output_schema: String,

    /// Local HTTP server port for IDE extensions
    /// Default: 9876
    #[serde(default = "default_gui_http_port")]
    pub http_port: u16,

    /// Shared secret required for IDE extension requests (sent as `X-KYCO-Token`)
    ///
    /// If empty, the server will accept unauthenticated requests (not recommended).
    #[serde(default)]
    pub http_token: String,

    /// Voice input settings
    #[serde(default)]
    pub voice: VoiceSettings,

    /// Orchestrator settings for external CLI sessions
    #[serde(default)]
    pub orchestrator: OrchestratorSettings,
}

/// Orchestrator settings for external CLI sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorSettings {
    /// The CLI command to use for the orchestrator.
    /// Use `{prompt_file}` as placeholder for the system prompt file path.
    /// Examples:
    /// - "claude --append-system-prompt \"$(cat {prompt_file})\""
    /// - "codex \"$(cat {prompt_file})\""
    /// - "aider --model gpt-4"
    /// If empty, uses default based on gui.default_agent
    #[serde(default)]
    pub cli_command: String,

    /// Custom system prompt for the orchestrator.
    /// If empty, uses the built-in default orchestrator prompt.
    #[serde(default)]
    pub system_prompt: String,
}

impl Default for OrchestratorSettings {
    fn default() -> Self {
        Self {
            cli_command: String::new(),
            system_prompt: String::new(),
        }
    }
}

/// Voice input settings for GUI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSettings {
    /// Voice input mode:
    /// - "disabled": No voice input
    /// - "manual": Click microphone button to record
    /// - "hotkey_hold": Hold hotkey to record, release to transcribe
    /// - "continuous": Always listening for mode keywords
    #[serde(default = "default_voice_mode")]
    pub mode: String,

    /// Keywords to listen for in continuous mode
    /// Default: mode names (refactor, fix, tests, etc.)
    #[serde(default = "default_voice_keywords")]
    pub keywords: Vec<String>,

    /// Whisper model for transcription (tiny, base, small, medium, large)
    #[serde(default = "default_whisper_model")]
    pub whisper_model: String,

    /// Language for transcription (auto, en, de, fr, etc.)
    #[serde(default = "default_voice_language")]
    pub language: String,

    /// Silence threshold to stop recording (0.0-1.0)
    #[serde(default = "default_silence_threshold")]
    pub silence_threshold: f32,

    /// Silence duration to stop recording (in seconds)
    #[serde(default = "default_silence_duration")]
    pub silence_duration: f32,

    /// Maximum recording duration (in seconds)
    #[serde(default = "default_max_duration")]
    pub max_duration: f32,
}

// Default functions for VoiceSettings

fn default_voice_mode() -> String {
    "disabled".to_string()
}

fn default_voice_keywords() -> Vec<String> {
    vec![
        "refactor".to_string(),
        "fix".to_string(),
        "tests".to_string(),
        "docs".to_string(),
        "review".to_string(),
        "optimize".to_string(),
        "implement".to_string(),
        "explain".to_string(),
    ]
}

fn default_whisper_model() -> String {
    "base".to_string()
}

fn default_voice_language() -> String {
    "auto".to_string()
}

fn default_silence_threshold() -> f32 {
    0.1 // 10% - higher value = less sensitive to background noise
}

fn default_silence_duration() -> f32 {
    2.5 // seconds - longer pause detection to avoid cutting off mid-speech
}

fn default_max_duration() -> f32 {
    300.0 // 5 minutes - safety limit for manual recording
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            mode: default_voice_mode(),
            keywords: default_voice_keywords(),
            whisper_model: default_whisper_model(),
            language: default_voice_language(),
            silence_threshold: default_silence_threshold(),
            silence_duration: default_silence_duration(),
            max_duration: default_max_duration(),
        }
    }
}

// Default functions for GuiSettings

fn default_gui_hotkey() -> String {
    #[cfg(target_os = "macos")]
    return "cmd+option+k".to_string();
    #[cfg(not(target_os = "macos"))]
    return "ctrl+alt+k".to_string();
}

fn default_gui_agent() -> String {
    "claude".to_string()
}

fn default_gui_mode() -> String {
    "implement".to_string()
}

fn default_output_schema() -> String {
    r#"
IMPORTANT: End your response with a structured YAML summary block:
---
title: Short task title (max 60 chars)
commit_subject: Suggested git commit subject (max 72 chars)
commit_body: |
  Suggested git commit body (optional, can be multiline)
details: What was done (2-3 sentences)
status: success|partial|failed
summary: |
  Detailed summary of findings and actions (optional, can be multiline).
  This is passed to the next agent in a chain for context.
state: <state_identifier>
---

STATE VALUES: issues_found, no_issues, fixed, unfixable, tests_pass, tests_fail, implemented, blocked, refactored, documented
"#
    .to_string()
}

fn default_gui_http_port() -> u16 {
    9876
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            hotkey: default_gui_hotkey(),
            default_agent: default_gui_agent(),
            default_mode: default_gui_mode(),
            output_schema: default_output_schema(),
            structured_output_schema: String::new(),
            http_port: default_gui_http_port(),
            http_token: String::new(),
            voice: VoiceSettings::default(),
            orchestrator: OrchestratorSettings::default(),
        }
    }
}

// Default functions for Settings

fn default_max_concurrent_jobs() -> usize {
    4
}

fn default_auto_run() -> bool {
    true
}

fn default_use_worktree() -> bool {
    false
}

fn default_max_jobs_per_file() -> usize {
    1 // Only one job per file to prevent agents overwriting each other's changes
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_concurrent_jobs: default_max_concurrent_jobs(),
            auto_run: default_auto_run(),
            use_worktree: default_use_worktree(),
            max_jobs_per_file: default_max_jobs_per_file(),
            gui: GuiSettings::default(),
            registry: RegistrySettings::default(),
            claude: ClaudeSettings::default(),
        }
    }
}

/// Registry settings for agent adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySettings {
    /// List of enabled adapter IDs
    /// If empty (default), all adapters are enabled
    /// Example: ["claude", "codex"] - only these adapters will be registered
    #[serde(default)]
    pub enabled_adapters: Vec<String>,

    /// List of disabled adapter IDs
    /// These adapters will not be registered even if available
    /// Example: ["claude-terminal"] - terminal adapter will be skipped
    #[serde(default)]
    pub disabled_adapters: Vec<String>,

    /// Suffix used for terminal/REPL mode adapter IDs
    /// Default: "-terminal" (e.g., "claude-terminal")
    #[serde(default = "default_terminal_suffix")]
    pub terminal_suffix: String,
}

fn default_terminal_suffix() -> String {
    "-terminal".to_string()
}

impl Default for RegistrySettings {
    fn default() -> Self {
        Self {
            enabled_adapters: Vec::new(),
            disabled_adapters: Vec::new(),
            terminal_suffix: default_terminal_suffix(),
        }
    }
}
