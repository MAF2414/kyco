//! GUI Theme: "Terminal Phosphor" - Retro CRT monitor aesthetic
//!
//! Color constants for the KYCo GUI, inspired by vintage CRT monitors.

use eframe::egui::Color32;

// ═══════════════════════════════════════════════════════════════════════════
// BACKGROUNDS
// ═══════════════════════════════════════════════════════════════════════════

/// Background: Deep charcoal with subtle blue tint (like a powered-off CRT)
pub const BG_PRIMARY: Color32 = Color32::from_rgb(18, 20, 24);
/// Secondary background for panels
pub const BG_SECONDARY: Color32 = Color32::from_rgb(24, 28, 34);
/// Accent highlight background
pub const BG_HIGHLIGHT: Color32 = Color32::from_rgb(32, 40, 52);
/// Selected item background
pub const BG_SELECTED: Color32 = Color32::from_rgb(40, 50, 65);

// ═══════════════════════════════════════════════════════════════════════════
// TEXT COLORS
// ═══════════════════════════════════════════════════════════════════════════

/// Primary text: Warm amber phosphor glow
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(255, 176, 0);
/// Secondary text: Dimmed amber
pub const TEXT_DIM: Color32 = Color32::from_rgb(180, 130, 50);
/// Muted text
pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 85, 60);

// ═══════════════════════════════════════════════════════════════════════════
// STATUS COLORS
// ═══════════════════════════════════════════════════════════════════════════

pub const STATUS_PENDING: Color32 = Color32::from_rgb(150, 150, 150);
pub const STATUS_QUEUED: Color32 = Color32::from_rgb(100, 180, 255);
/// Orange - waiting for file lock
pub const STATUS_BLOCKED: Color32 = Color32::from_rgb(255, 165, 0);
pub const STATUS_RUNNING: Color32 = Color32::from_rgb(255, 200, 50);
pub const STATUS_DONE: Color32 = Color32::from_rgb(80, 255, 120);
pub const STATUS_FAILED: Color32 = Color32::from_rgb(255, 80, 80);
pub const STATUS_REJECTED: Color32 = Color32::from_rgb(180, 100, 100);
pub const STATUS_MERGED: Color32 = Color32::from_rgb(150, 100, 255);

// ═══════════════════════════════════════════════════════════════════════════
// ACCENT COLORS
// ═══════════════════════════════════════════════════════════════════════════

pub const ACCENT_CYAN: Color32 = Color32::from_rgb(0, 255, 200);
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(80, 255, 120);
pub const ACCENT_RED: Color32 = Color32::from_rgb(255, 80, 80);
pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(200, 120, 255);
pub const ACCENT_YELLOW: Color32 = Color32::from_rgb(255, 200, 50);
