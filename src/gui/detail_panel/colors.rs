//! Color utilities for the detail panel

use eframe::egui;

use crate::JobStatus;

use crate::gui::app::{
    ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, STATUS_DONE, STATUS_FAILED, STATUS_MERGED,
    STATUS_PENDING, STATUS_QUEUED, STATUS_REJECTED, STATUS_RUNNING, TEXT_DIM, TEXT_MUTED,
    TEXT_PRIMARY,
};
use crate::LogEventKind;

/// Get status color for a job status
pub fn status_color(status: JobStatus) -> egui::Color32 {
    match status {
        JobStatus::Pending => STATUS_PENDING,
        JobStatus::Queued => STATUS_QUEUED,
        JobStatus::Running => STATUS_RUNNING,
        JobStatus::Done => STATUS_DONE,
        JobStatus::Failed => STATUS_FAILED,
        JobStatus::Rejected => STATUS_REJECTED,
        JobStatus::Merged => STATUS_MERGED,
    }
}

/// Get log event color
pub fn log_color(kind: &LogEventKind) -> egui::Color32 {
    match kind {
        LogEventKind::Thought => TEXT_DIM,
        LogEventKind::ToolCall => ACCENT_CYAN,
        LogEventKind::ToolOutput => TEXT_MUTED,
        LogEventKind::Text => TEXT_PRIMARY,
        LogEventKind::Error => ACCENT_RED,
        LogEventKind::System => ACCENT_GREEN,
    }
}
