//! Update checking module for Kyco
//!
//! Checks GitHub releases for newer versions and provides update notifications.
//! Supports auto-installation and periodic checks.

mod check;
mod install;

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::{Duration, Instant};

pub use install::{install_update, open_url};

/// Current version from Cargo.toml
pub(crate) const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository for update checks
pub(crate) const GITHUB_REPO: &str = "MAF2414/kyco";

/// How often to check for updates (5 minutes)
const CHECK_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Information about an available update
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    /// The new version available
    pub version: String,
    /// URL to the GitHub release page
    pub release_url: String,
    /// Direct download URL for the current platform
    pub download_url: String,
    /// Release notes (body from GitHub release)
    pub release_notes: Option<String>,
}

/// Status of the update check
#[derive(Debug, Clone)]
pub enum UpdateStatus {
    /// Not yet checked
    NotChecked,
    /// Currently checking
    Checking,
    /// Check complete, no update available
    UpToDate,
    /// Update available
    UpdateAvailable(UpdateInfo),
    /// Currently downloading/installing
    Installing(String),
    /// Installation complete - restart required
    InstallComplete(String),
    /// Error during check or install
    Error(String),
}

/// Update checker that runs in a background thread with periodic checks
pub struct UpdateChecker {
    /// Receiver for update check results
    rx: Receiver<UpdateStatus>,
    /// Sender to trigger new checks
    check_tx: Sender<()>,
    /// Current status
    status: UpdateStatus,
    /// Last check time
    last_check: Instant,
}

impl UpdateChecker {
    /// Create a new update checker and start checking in background
    pub fn new() -> Self {
        let (status_tx, status_rx) = channel();
        let (check_tx, check_rx) = channel::<()>();

        thread::spawn(move || {
            check::update_checker_loop(status_tx, check_rx);
        });

        Self {
            rx: status_rx,
            check_tx,
            status: UpdateStatus::Checking,
            last_check: Instant::now(),
        }
    }

    /// Poll for update check results (non-blocking)
    /// Also triggers periodic re-checks every 5 minutes
    pub fn poll(&mut self) -> &UpdateStatus {
        while let Ok(status) = self.rx.try_recv() {
            self.status = status;
        }

        if self.last_check.elapsed() >= CHECK_INTERVAL {
            self.trigger_check();
        }

        &self.status
    }

    /// Manually trigger a new check
    pub fn trigger_check(&mut self) {
        self.last_check = Instant::now();
        let _ = self.check_tx.send(());
    }

    /// Get the current status
    pub fn status(&self) -> &UpdateStatus {
        &self.status
    }

    /// Check if an update is available
    pub fn has_update(&self) -> bool {
        matches!(self.status, UpdateStatus::UpdateAvailable(_))
    }

    /// Get update info if available
    pub fn update_info(&self) -> Option<&UpdateInfo> {
        match &self.status {
            UpdateStatus::UpdateAvailable(info) => Some(info),
            _ => None,
        }
    }
}

impl Default for UpdateChecker {
    fn default() -> Self {
        Self::new()
    }
}
