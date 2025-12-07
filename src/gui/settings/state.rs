//! Settings state struct for the GUI
//!
//! Contains the mutable state used during settings editing.

use std::path::Path;

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
    // General settings
    pub settings_max_concurrent: &'a mut String,
    pub settings_debounce_ms: &'a mut String,
    pub settings_auto_run: &'a mut bool,
    pub settings_marker_prefix: &'a mut String,
    pub settings_use_worktree: &'a mut bool,
    pub settings_scan_exclude: &'a mut String,
    pub settings_output_schema: &'a mut String,
    pub settings_status: &'a mut Option<(String, bool)>,

    // Voice settings
    pub voice_settings_mode: &'a mut String,
    pub voice_settings_keywords: &'a mut String,
    pub voice_settings_model: &'a mut String,
    pub voice_settings_language: &'a mut String,
    pub voice_settings_silence_threshold: &'a mut String,
    pub voice_settings_silence_duration: &'a mut String,
    pub voice_settings_max_duration: &'a mut String,
    pub voice_install_status: &'a mut Option<(String, bool)>,
    pub voice_install_in_progress: &'a mut bool,

    // Voice test state
    pub voice_test_status: &'a mut VoiceTestStatus,
    pub voice_test_result: &'a mut Option<String>,

    // VAD settings
    pub vad_enabled: &'a mut bool,
    pub vad_speech_threshold: &'a mut String,
    pub vad_silence_duration_ms: &'a mut String,

    // Voice action registry (read-only, from config)
    pub voice_action_registry: &'a VoiceActionRegistry,

    // Extension status
    pub extension_status: &'a mut Option<(String, bool)>,

    // Common state
    pub view_mode: &'a mut ViewMode,
    pub config: &'a mut Config,
    pub work_dir: &'a Path,
}
