//! GUI module for the main kyco application
//!
//! This module provides a native GUI that receives selections from IDE extensions
//! via HTTP server and manages job execution.
//!
//! ## Voice Input
//!
//! The GUI supports voice input with three modes:
//! - **Manual**: Click the microphone button or press Shift+V to record
//! - **Hotkey Hold**: Hold Shift+V while speaking, release to transcribe
//! - **Continuous**: Always listening for mode keywords (e.g., "refactor", "fix")
//!
//! Voice input requires additional setup:
//! 1. Audio capture library (cpal)
//! 2. Speech recognition (whisper-rs or external API)

pub mod agents;
pub mod app;
pub mod detail_panel;
pub mod diff;
pub mod executor;
pub mod http_server;
pub mod install;
pub mod jobs;
pub mod modes;
pub mod output_schema;
pub mod runner;
pub mod selection_popup;
pub mod settings;
pub mod status_bar;
pub mod voice;
pub mod voice_install;
pub mod voice_settings;

pub use app::{Agent, KycoApp, Mode, SelectionContext};
pub use executor::{start_executor, ExecutorEvent};
pub use http_server::SelectionRequest;
pub use runner::run_gui;
pub use voice::{VoiceConfig, VoiceEvent, VoiceInputMode, VoiceManager, VoiceState};
