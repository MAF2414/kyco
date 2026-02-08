//! Settings state struct for the GUI
//!
//! Contains the mutable state used during settings editing.

use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use crate::config::Config;
use crate::gui::app::ViewMode;
use crate::gui::voice::VoiceActionRegistry;

/// Voice test status for the settings UI
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum VoiceTestStatus {
    #[default]
    Idle,
    Recording,
    Transcribing,
    Success,
    Error(String),
}

/// State for settings editing UI
pub struct SettingsState<'a> {
    pub settings_max_concurrent: &'a mut String,
    pub settings_auto_run: &'a mut bool,
    pub settings_auto_allow: &'a mut bool,
    pub settings_use_worktree: &'a mut bool,
    pub settings_output_schema: &'a mut String,
    pub settings_structured_output_schema: &'a mut String,
    pub settings_status: &'a mut Option<(String, bool)>,

    pub voice_settings_mode: &'a mut String,
    pub voice_settings_keywords: &'a mut String,
    pub voice_settings_model: &'a mut String,
    pub voice_settings_language: &'a mut String,
    pub voice_settings_silence_threshold: &'a mut String,
    pub voice_settings_silence_duration: &'a mut String,
    pub voice_settings_max_duration: &'a mut String,
    pub voice_settings_global_hotkey: &'a mut String,
    pub voice_settings_popup_hotkey: &'a mut String,
    pub voice_install_status: &'a mut Option<(String, bool)>,
    pub voice_install_in_progress: &'a mut bool,
    /// Handle for async voice installation (set when installation starts)
    pub voice_install_handle: &'a mut Option<crate::gui::voice::install::InstallHandle>,

    pub voice_test_status: &'a mut VoiceTestStatus,
    pub voice_test_result: &'a mut Option<String>,

    pub vad_enabled: &'a mut bool,
    pub vad_speech_threshold: &'a mut String,
    pub vad_silence_duration_ms: &'a mut String,

    pub voice_action_registry: &'a VoiceActionRegistry,

    pub extension_status: &'a mut Option<(String, bool)>,

    pub view_mode: &'a mut ViewMode,
    pub config: &'a mut Config,
    pub work_dir: &'a Path,

    /// Flag to indicate voice config was changed and VoiceManager needs to be updated
    pub voice_config_changed: &'a mut bool,

    /// Shared max concurrent jobs (updates executor in real-time)
    pub max_concurrent_jobs_shared: &'a Arc<AtomicUsize>,

    /// Orchestrator settings
    pub orchestrator_cli_agent: &'a mut String,
    pub orchestrator_cli_command: &'a mut String,
    pub orchestrator_system_prompt: &'a mut String,
}
