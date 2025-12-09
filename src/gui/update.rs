//! Update checking module for Kyco
//!
//! Checks GitHub releases for newer versions and provides update notifications.

use semver::Version;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// Current version from Cargo.toml
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository for update checks
const GITHUB_REPO: &str = "MAF2414/kyco";

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
    /// Error during check
    Error(String),
}

/// Update checker that runs in a background thread
pub struct UpdateChecker {
    /// Receiver for update check results
    rx: Receiver<UpdateStatus>,
    /// Current status
    status: UpdateStatus,
}

impl UpdateChecker {
    /// Create a new update checker and start checking in background
    pub fn new() -> Self {
        let (tx, rx) = channel();

        // Start background check
        thread::spawn(move || {
            check_for_updates(tx);
        });

        Self {
            rx,
            status: UpdateStatus::Checking,
        }
    }

    /// Poll for update check results (non-blocking)
    pub fn poll(&mut self) -> &UpdateStatus {
        // Check if we have a result
        if let Ok(status) = self.rx.try_recv() {
            self.status = status;
        }
        &self.status
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

/// Perform the actual update check (runs in background thread)
fn check_for_updates(tx: Sender<UpdateStatus>) {
    let result = do_check();
    let _ = tx.send(result);
}

/// Execute the update check
fn do_check() -> UpdateStatus {
    // Fetch latest release from GitHub API
    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);

    let response = match ureq::get(&url)
        .set("User-Agent", "kyco-update-checker")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
    {
        Ok(r) => r,
        Err(e) => return UpdateStatus::Error(format!("Failed to fetch release info: {}", e)),
    };

    // Parse JSON response
    let body = match response.into_string() {
        Ok(b) => b,
        Err(e) => return UpdateStatus::Error(format!("Failed to read response: {}", e)),
    };

    let json: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return UpdateStatus::Error(format!("Failed to parse JSON: {}", e)),
    };

    // Extract version (tag_name, strip 'v' prefix)
    let tag_name = match json.get("tag_name").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return UpdateStatus::Error("No tag_name in release".to_string()),
    };

    let latest_version_str = tag_name.strip_prefix('v').unwrap_or(tag_name);

    // Parse versions
    let current = match Version::parse(CURRENT_VERSION) {
        Ok(v) => v,
        Err(e) => return UpdateStatus::Error(format!("Invalid current version: {}", e)),
    };

    let latest = match Version::parse(latest_version_str) {
        Ok(v) => v,
        Err(e) => return UpdateStatus::Error(format!("Invalid latest version: {}", e)),
    };

    // Compare versions
    if latest <= current {
        return UpdateStatus::UpToDate;
    }

    // Extract release info
    let release_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let release_notes = json.get("body").and_then(|v| v.as_str()).map(String::from);

    // Determine platform-specific download URL
    let download_url = get_platform_download_url(latest_version_str);

    UpdateStatus::UpdateAvailable(UpdateInfo {
        version: latest_version_str.to_string(),
        release_url,
        download_url,
        release_notes,
    })
}

/// Get the download URL for the current platform
fn get_platform_download_url(version: &str) -> String {
    let base = format!("https://github.com/{}/releases/download/v{}", GITHUB_REPO, version);

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return format!("{}/kyco-macos-arm64", base);
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return format!("{}/kyco-macos-x64", base);
    }

    #[cfg(target_os = "linux")]
    {
        return format!("{}/kyco-linux-x64", base);
    }

    #[cfg(target_os = "windows")]
    {
        return format!("{}/kyco-windows-x64.exe", base);
    }

    // Fallback for other platforms
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        format!("https://github.com/{}/releases/latest", GITHUB_REPO)
    }
}

/// Open a URL in the default browser
pub fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
}
