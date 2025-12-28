//! Update check logic

use semver::Version;
use std::sync::mpsc::{Receiver, Sender};

use super::{CURRENT_VERSION, GITHUB_REPO, UpdateInfo, UpdateStatus};

/// Background thread that handles update checking
pub(super) fn update_checker_loop(tx: Sender<UpdateStatus>, rx: Receiver<()>) {
    let result = do_check();
    let _ = tx.send(result);

    while rx.recv().is_ok() {
        let _ = tx.send(UpdateStatus::Checking);
        let result = do_check();
        let _ = tx.send(result);
    }
}

/// Execute the update check
fn do_check() -> UpdateStatus {
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

    let current = match Version::parse(CURRENT_VERSION) {
        Ok(v) => v,
        Err(e) => return UpdateStatus::Error(format!("Invalid current version: {}", e)),
    };

    let latest = match Version::parse(latest_version_str) {
        Ok(v) => v,
        Err(e) => return UpdateStatus::Error(format!("Invalid latest version: {}", e)),
    };

    if latest <= current {
        return UpdateStatus::UpToDate;
    }

    let release_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let release_notes = json.get("body").and_then(|v| v.as_str()).map(String::from);

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
