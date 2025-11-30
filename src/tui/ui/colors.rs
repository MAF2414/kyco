//! Color constants and shared styles for the TUI

use ratatui::style::Color;

// VIVID color constants - direct usage for clarity
pub const CYAN: Color = Color::Cyan;
pub const GREEN: Color = Color::LightGreen;
pub const YELLOW: Color = Color::Yellow;
pub const RED: Color = Color::LightRed;
pub const MAGENTA: Color = Color::Magenta;
pub const BLUE: Color = Color::LightBlue;
pub const WHITE: Color = Color::White;
pub const GRAY: Color = Color::Gray;
pub const DARK_GRAY: Color = Color::DarkGray;

// Background color - Reset ensures terminal default background is used consistently
// This prevents visual artifacts (gray veil) from stale buffer content bleeding through
pub const BG: Color = Color::Reset;
