//! Update checking module for Kyco
//!
//! Checks GitHub releases for newer versions and provides update notifications.
//! Supports auto-installation and periodic checks.

use semver::Version;
use std::io::Read;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::{Duration, Instant};

/// Current version from Cargo.toml
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository for update checks
const GITHUB_REPO: &str = "MAF2414/kyco";

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

        // Start background checker thread
        thread::spawn(move || {
            update_checker_loop(status_tx, check_rx);
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
        // Check if we have a result
        while let Ok(status) = self.rx.try_recv() {
            self.status = status;
        }

        // Trigger periodic check every 5 minutes
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

/// Background thread that handles update checking
fn update_checker_loop(tx: Sender<UpdateStatus>, rx: Receiver<()>) {
    // Initial check
    let result = do_check();
    let _ = tx.send(result);

    // Wait for periodic check signals
    while rx.recv().is_ok() {
        let _ = tx.send(UpdateStatus::Checking);
        let result = do_check();
        let _ = tx.send(result);
    }
}

/// Execute the update check
fn do_check() -> UpdateStatus {
    // Fetch latest release from GitHub API
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

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
    let base = format!(
        "https://github.com/{}/releases/download/v{}",
        GITHUB_REPO, version
    );

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

/// Check if we have write permission to the executable's directory
fn has_write_permission(path: &std::path::Path) -> bool {
    if let Some(parent) = path.parent() {
        // Try to create a temp file to check write permission
        let test_path = parent.join(".kyco_write_test");
        if std::fs::write(&test_path, "test").is_ok() {
            let _ = std::fs::remove_file(&test_path);
            return true;
        }
    }
    false
}

/// Download and install update, returns status message
pub fn install_update(info: &UpdateInfo) -> Result<String, String> {
    // Get the path to the current executable
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;

    // Check if we need elevated privileges
    if !has_write_permission(&current_exe) {
        #[cfg(target_os = "macos")]
        {
            return install_update_with_admin_macos(info, &current_exe);
        }
        #[cfg(not(target_os = "macos"))]
        {
            return Err("Permission denied. Please run as administrator or move kyco to a user-writable location.".to_string());
        }
    }

    install_update_direct(info, &current_exe)
}

/// Install update directly (when we have write permissions)
fn install_update_direct(
    info: &UpdateInfo,
    current_exe: &std::path::Path,
) -> Result<String, String> {
    // Download the new binary
    let response = ureq::get(&info.download_url)
        .set("User-Agent", "kyco-update-installer")
        .call()
        .map_err(|e| format!("Failed to download update: {}", e))?;

    // Read binary data
    let mut binary_data = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut binary_data)
        .map_err(|e| format!("Failed to read download: {}", e))?;

    // Create backup of current binary
    let backup_path = current_exe.with_extension("backup");
    if backup_path.exists() {
        std::fs::remove_file(&backup_path)
            .map_err(|e| format!("Failed to remove old backup: {}", e))?;
    }
    std::fs::rename(current_exe, &backup_path)
        .map_err(|e| format!("Failed to backup current binary: {}", e))?;

    // Write new binary
    std::fs::write(current_exe, &binary_data).map_err(|e| {
        // Try to restore backup on failure
        let _ = std::fs::rename(&backup_path, current_exe);
        format!("Failed to write new binary: {}", e)
    })?;

    // Set executable permissions (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(current_exe)
            .map_err(|e| format!("Failed to get permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(current_exe, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    // Clean up backup
    let _ = std::fs::remove_file(&backup_path);

    Ok(format!(
        "Updated to v{}! Please restart kyco.",
        info.version
    ))
}

/// Install update using macOS admin privileges via osascript
#[cfg(target_os = "macos")]
fn install_update_with_admin_macos(
    info: &UpdateInfo,
    current_exe: &std::path::Path,
) -> Result<String, String> {
    use std::process::Command;

    let exe_path = current_exe.to_string_lossy();
    let download_url = &info.download_url;

    // Create a shell script that will be run with admin privileges
    // This script: downloads the binary, backs up the current one, replaces it, and sets permissions
    let script = format!(
        r#"
        set -e
        TEMP_FILE=$(mktemp)
        curl -sL "{download_url}" -o "$TEMP_FILE"
        if [ -f "{exe_path}.backup" ]; then rm "{exe_path}.backup"; fi
        mv "{exe_path}" "{exe_path}.backup"
        mv "$TEMP_FILE" "{exe_path}"
        chmod 755 "{exe_path}"
        rm -f "{exe_path}.backup"
        "#,
        download_url = download_url,
        exe_path = exe_path
    );

    // Use osascript to run the script with administrator privileges
    // This will show the native macOS password dialog
    let output = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            r#"do shell script "{}" with administrator privileges"#,
            script
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', " ")
        ))
        .output()
        .map_err(|e| format!("Failed to request admin privileges: {}", e))?;

    if output.status.success() {
        Ok(format!(
            "Updated to v{}! Please restart kyco.",
            info.version
        ))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("User canceled") || stderr.contains("(-128)") {
            Err("Update cancelled by user.".to_string())
        } else {
            Err(format!("Update failed: {}", stderr))
        }
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
