//! Installation result types and progress tracking

use std::sync::mpsc::Receiver;

/// Result of a voice dependency installation attempt
#[derive(Debug, Clone)]
pub struct VoiceInstallResult {
    /// Status message to display
    pub message: String,
    /// Whether the result is an error
    pub is_error: bool,
    /// Whether installation is still in progress
    pub in_progress: bool,
}

impl VoiceInstallResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: false,
            in_progress: false,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: true,
            in_progress: false,
        }
    }

    pub fn progress(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: false,
            in_progress: true,
        }
    }
}

/// Installation progress updates
#[derive(Debug, Clone)]
pub enum InstallProgress {
    /// Installation step started
    Step {
        step: u8,
        total: u8,
        message: String,
    },
    /// Installation completed successfully
    Complete(VoiceInstallResult),
    /// Installation failed
    Failed(VoiceInstallResult),
}

/// Async installation handle
pub struct InstallHandle {
    /// Receiver for progress updates
    pub progress_rx: Receiver<InstallProgress>,
}

/// Check current installation status (for progress display)
pub fn check_installation_status() -> VoiceInstallResult {
    VoiceInstallResult::progress("Checking Homebrew...")
}
