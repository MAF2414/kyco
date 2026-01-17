//! Settings configuration types

mod gui;
mod orchestrator;
mod registry;
mod voice;

pub use gui::{default_structured_output_schema, GuiSettings};
pub use orchestrator::{default_orchestrator_system_prompt, OrchestratorSettings};
pub(crate) use orchestrator::is_legacy_orchestrator_system_prompt;
pub use registry::RegistrySettings;
pub use voice::VoiceSettings;

use serde::{Deserialize, Serialize};

/// General settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Maximum concurrent jobs per agent (e.g., 4 means 4 Claude + 4 Codex simultaneously)
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
    1
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
