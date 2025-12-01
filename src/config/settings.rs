//! Settings configuration types

use serde::{Deserialize, Serialize};

/// General settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Maximum concurrent jobs
    #[serde(default = "default_max_concurrent_jobs")]
    pub max_concurrent_jobs: usize,

    /// Debounce interval for file watcher in milliseconds
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,

    /// Automatically run new jobs when found (no manual confirmation)
    #[serde(default = "default_auto_run")]
    pub auto_run: bool,

    /// Files/directories to exclude from scanning (glob patterns)
    /// Default: ["kyco.toml", ".kyco/**"]
    #[serde(default = "default_scan_exclude")]
    pub scan_exclude: Vec<String>,

    /// Marker prefix for comment detection
    /// Default: "@" - e.g., @docs, @fix, @claude:test
    /// Alternatives: "::", "cr:", "TODO:", etc.
    #[serde(default = "default_marker_prefix")]
    pub marker_prefix: String,

    /// Use Git worktrees for job isolation
    /// When true, each job runs in a separate Git worktree
    /// When false (default), jobs run in the main working directory
    #[serde(default = "default_use_worktree")]
    pub use_worktree: bool,

    /// GUI settings
    #[serde(default)]
    pub gui: GuiSettings,

    /// Registry settings for agent adapters
    #[serde(default)]
    pub registry: RegistrySettings,
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

    /// Voice input settings
    #[serde(default)]
    pub voice: VoiceSettings,
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
    0.01
}

fn default_silence_duration() -> f32 {
    1.5
}

fn default_max_duration() -> f32 {
    30.0
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
IMPORTANT: End your response with this YAML block:
---kyco
title: Short task title (max 60 chars)
details: What was done (2-3 sentences)
status: success|partial|failed
summary: |
  Detailed summary of findings and actions (optional, can be multiline).
  This is passed to the next agent in a chain for context.
  Include all relevant information the next step might need.
state: <state_identifier>
---

STATE IDENTIFIERS (use exactly these values):
- For review: "issues_found" or "no_issues"
- For fix: "fixed" or "unfixable"
- For tests: "tests_pass" or "tests_fail"
- For implement: "implemented" or "blocked"
- For refactor: "refactored"
- For docs: "documented"
"#
    .to_string()
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            hotkey: default_gui_hotkey(),
            default_agent: default_gui_agent(),
            default_mode: default_gui_mode(),
            output_schema: default_output_schema(),
            voice: VoiceSettings::default(),
        }
    }
}

// Default functions for Settings

fn default_max_concurrent_jobs() -> usize {
    4
}

fn default_debounce_ms() -> u64 {
    500
}

fn default_auto_run() -> bool {
    false
}

fn default_scan_exclude() -> Vec<String> {
    vec!["kyco.toml".to_string(), ".kyco/**".to_string()]
}

fn default_marker_prefix() -> String {
    "@@".to_string()
}

fn default_use_worktree() -> bool {
    false
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_concurrent_jobs: default_max_concurrent_jobs(),
            debounce_ms: default_debounce_ms(),
            auto_run: default_auto_run(),
            scan_exclude: default_scan_exclude(),
            marker_prefix: default_marker_prefix(),
            use_worktree: default_use_worktree(),
            gui: GuiSettings::default(),
            registry: RegistrySettings::default(),
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
    /// Example: ["gemini"] - gemini adapter will be skipped
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
