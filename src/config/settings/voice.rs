//! Voice input settings for GUI

use serde::{Deserialize, Serialize};

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

    /// Global voice hotkey (dictate from any app)
    /// Format: "modifier+key" e.g., "cmd+shift+v", "ctrl+shift+v"
    #[serde(default = "default_global_voice_hotkey")]
    pub global_hotkey: String,

    /// Popup voice hotkey (start/stop recording in selection popup)
    /// Format: "modifier+key" e.g., "cmd+d", "ctrl+d"
    #[serde(default = "default_popup_voice_hotkey")]
    pub popup_hotkey: String,
}

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

fn default_global_voice_hotkey() -> String {
    #[cfg(target_os = "macos")]
    return "cmd+shift+v".to_string();
    #[cfg(not(target_os = "macos"))]
    return "ctrl+shift+v".to_string();
}

fn default_popup_voice_hotkey() -> String {
    #[cfg(target_os = "macos")]
    return "cmd+d".to_string();
    #[cfg(not(target_os = "macos"))]
    return "ctrl+d".to_string();
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
            global_hotkey: default_global_voice_hotkey(),
            popup_hotkey: default_popup_voice_hotkey(),
        }
    }
}
