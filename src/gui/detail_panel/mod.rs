//! Detail panel module for the GUI
//!
//! Handles all detail panel functionality including:
//! - Rendering job information, prompt preview, and action buttons
//! - Activity log display
//! - Status and log color utilities

mod colors;
mod panel;
mod prompt;

pub use colors::status_color;
pub use panel::{render_detail_panel, DetailPanelAction, DetailPanelState};
