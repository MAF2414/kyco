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
pub mod animations;
pub mod app;
mod app_achievements;
mod app_diff;
mod app_eframe;
mod app_helpers;
mod app_input;
mod app_jobs;
mod app_new;
mod app_orchestrator;
mod app_popup;
mod app_render;
mod app_selection;
mod app_stats;
mod app_theme;
mod app_types;
mod app_update;
mod app_voice;
pub mod chains;
pub mod detail_panel;
pub mod diff;
pub mod executor;
pub mod groups;
pub mod hotkey;
pub mod http_server;
pub mod install;
pub mod jobs;
pub mod skills;
pub mod output_schema;
pub mod permission;
pub mod runner;
pub mod selection;
pub mod settings;
pub mod status_bar;
pub mod theme;
mod toast;
pub mod update;
pub mod voice;

pub use app::KycoApp;
pub use app_types::{Mode, ViewMode};
pub use executor::{ExecutorEvent, start_executor};
pub use groups::{ComparisonAction, ComparisonState};
pub use http_server::{BatchFile, BatchRequest, SelectionRequest};
pub use permission::{
    PermissionAction, PermissionPopupState, PermissionRequest, render_permission_popup,
};
pub use runner::run_gui;
pub use selection::SelectionContext;
pub use update::{UpdateChecker, UpdateInfo, UpdateStatus};
pub use voice::{VoiceConfig, VoiceEvent, VoiceInputMode, VoiceManager, VoiceState};
