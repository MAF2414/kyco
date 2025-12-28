//! Detail panel module for the GUI
//!
//! Handles all detail panel functionality including:
//! - Rendering job information, prompt preview, and action buttons
//! - Activity log display
//! - Status and log color utilities

mod actions;
mod activity_log;
mod chain;
mod colors;
mod markdown;
mod panel;
mod prompt;
mod result;
mod types;

pub use colors::status_color;
pub use panel::render_detail_panel;
pub use types::{ActivityLogFilters, DetailPanelAction, DetailPanelState};
