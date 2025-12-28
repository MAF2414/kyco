//! Voice input module for GUI
//!
//! This module provides:
//! 1. Microphone button in selection popup for manual voice recording
//! 2. Hotkey-triggered voice recording with automatic transcription
//! 3. Continuous listening mode with VAD + keyword detection for hands-free operation
//!
//! Architecture:
//! - VoiceState: Current state of voice input (idle, recording, transcribing, listening)
//! - VoiceConfig: Configuration for voice features
//! - VoiceManager: Handles audio capture and transcription coordination
//! - VoiceActionRegistry: Maps wakewords to modes/actions
//! - VAD: Voice Activity Detection for efficient continuous listening
//!
//! Implementation:
//! - Uses `sox` (rec command) for audio recording
//! - Uses `whisper-cli` (from whisper-cpp) for transcription
//! - Uses Silero VAD for voice activity detection

pub mod actions;
mod availability;
pub mod install;
mod manager;
pub mod paste;
mod recording;
pub mod settings;
mod transcription;
mod types;
pub mod vad;

#[cfg(test)]
mod tests;

// Re-export from actions
pub use actions::{VoiceAction, VoiceActionRegistry, WakewordMatch};

// Re-export from install
pub use install::{
    InstallHandle, InstallProgress, VoiceInstallResult, WHISPER_MODELS, WhisperModel,
    get_model_info, install_voice_dependencies, install_voice_dependencies_async,
    is_model_installed,
};

// Re-export from manager
pub use manager::VoiceManager;

// Re-export from transcription
pub use transcription::parse_voice_input;

// Re-export from paste
pub use paste::{copy_and_paste, paste_from_clipboard};

// Re-export from settings
pub use settings::{VoiceSettingsState, render_voice_settings};

// Re-export from types
pub use types::{VoiceCommand, VoiceConfig, VoiceEvent, VoiceInputMode, VoiceState};

// Re-export from vad
pub use vad::{VadConfig, VadEvent, VadHandle, is_vad_available, start_vad_listener};
