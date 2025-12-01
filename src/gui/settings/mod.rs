//! Settings module for the GUI
//!
//! Renders the settings configuration view where users can:
//! - Configure general settings (max concurrent jobs, debounce, etc.)
//! - Configure output schema for agent prompts
//! - Install IDE extensions
//! - Configure voice input settings
//! - View HTTP server status

mod helpers;
mod panel;
mod save;
mod sections;
mod state;

pub use panel::render_settings;
pub use state::SettingsState;
