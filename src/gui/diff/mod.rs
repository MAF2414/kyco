//! Diff view module for displaying git diffs
//!
//! This module provides a modern diff popup UI with line numbers,
//! background highlighting, and clear visual structure.

mod render;
mod state;

use eframe::egui::Color32;

// Re-export public API
pub use render::{render_diff_content, render_diff_popup};
pub use state::DiffState;

// Module-internal re-exports for tests
#[cfg(test)]
use render::parse_hunk_header;
#[cfg(test)]
use state::extract_file_path;

// Background colors for diff lines
const BG_ADDED: Color32 = Color32::from_rgb(30, 50, 35);
const BG_REMOVED: Color32 = Color32::from_rgb(55, 30, 35);
const BG_HUNK: Color32 = Color32::from_rgb(30, 45, 55);

#[cfg(test)]
mod tests;
