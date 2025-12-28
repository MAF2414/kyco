//! IDE extension installation module
//!
//! This module handles installation of IDE extensions (VS Code, JetBrains, etc.)
//! that integrate with kyco for sending code selections.
//!
//! Extensions are downloaded from GitHub Releases.

use std::path::{Path, PathBuf};
use std::process::Command;

/// GitHub repository for downloading releases
const GITHUB_REPO: &str = "MAF2414/kyco";

/// Get the kyco cache directory for downloaded extensions
fn get_kyco_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("kyco")
        .join("extensions")
}

/// Download a file from a URL using curl
fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let dest_str = dest
        .to_str()
        .ok_or_else(|| format!("Invalid path (non-UTF8): {}", dest.display()))?;

    let output = Command::new("curl")
        .args([
            "-L", // Follow redirects
            "-f", // Fail on HTTP errors
            "-s", // Silent
            "-o", dest_str, url,
        ])
        .output()
        .map_err(|e| format!("curl failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Download failed: {}", stderr));
    }

    Ok(())
}

/// Result of an extension installation operation
pub struct ExtensionInstallResult {
    /// Human-readable message describing the result
    pub message: String,
    /// Whether the installation encountered an error
    pub is_error: bool,
}

impl ExtensionInstallResult {
    fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: false,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: true,
        }
    }
}

/// Install the VS Code extension by downloading from GitHub Releases
///
/// This function:
/// 1. Downloads the .vsix file from GitHub Releases
/// 2. Installs it using the VS Code CLI
#[allow(unused_variables)]
pub fn install_vscode_extension(work_dir: &Path) -> ExtensionInstallResult {
    let cache_dir = get_kyco_cache_dir();
    let vsix_path = cache_dir.join("kyco-vscode.vsix");

    let download_url = format!(
        "https://github.com/{}/releases/latest/download/kyco-vscode.vsix",
        GITHUB_REPO
    );

    if let Err(e) = download_file(&download_url, &vsix_path) {
        return ExtensionInstallResult::error(format!(
            "Failed to download VS Code extension: {}\n\nURL: {}",
            e, download_url
        ));
    }

    install_vsix(&vsix_path)
}

/// Install a .vsix file using the VS Code CLI
fn install_vsix(vsix_path: &Path) -> ExtensionInstallResult {
    let vsix_path_str = match vsix_path.to_str() {
        Some(s) => s,
        None => {
            return ExtensionInstallResult::error(format!(
                "Invalid extension path (non-UTF8): {}",
                vsix_path.display()
            ));
        }
    };

    let install_result = Command::new("code")
        .args(["--install-extension", vsix_path_str])
        .output();

    match install_result {
        Ok(output) if output.status.success() => ExtensionInstallResult::success(
            "VS Code extension installed! Restart VS Code to activate.\nHotkey: Cmd+Option+Y (Ctrl+Alt+Y on Windows/Linux)",
        ),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            ExtensionInstallResult::error(format!(
                "VS Code CLI failed to install extension.\n\nError: {}\n\nTry manually:\ncode --install-extension {}",
                stderr.trim(),
                vsix_path.display()
            ))
        }
        Err(_) => ExtensionInstallResult::error(format!(
            "VS Code CLI not found. Install VS Code or run manually:\ncode --install-extension {}",
            vsix_path.display()
        )),
    }
}

/// Install the JetBrains plugin by downloading from GitHub Releases
///
/// This function:
/// 1. Downloads the .zip file from GitHub Releases
/// 2. Provides instructions for manual installation in the IDE
#[allow(unused_variables)]
pub fn install_jetbrains_plugin(work_dir: &Path) -> ExtensionInstallResult {
    let cache_dir = get_kyco_cache_dir();
    let zip_path = cache_dir.join("kyco-jetbrains.zip");

    let download_url = format!(
        "https://github.com/{}/releases/latest/download/kyco-jetbrains.zip",
        GITHUB_REPO
    );

    if let Err(e) = download_file(&download_url, &zip_path) {
        return ExtensionInstallResult::error(format!(
            "Failed to download JetBrains plugin: {}\n\nURL: {}",
            e, download_url
        ));
    }

    ExtensionInstallResult::success(format!(
        "JetBrains plugin downloaded!\n\n\
        To install:\n\
        1. Open your JetBrains IDE (IntelliJ, WebStorm, etc.)\n\
        2. Go to Settings → Plugins → Gear Icon → Install Plugin from Disk\n\
        3. Select: {}\n\
        4. Restart the IDE\n\n\
        Hotkey: Ctrl+Alt+Y (Cmd+Option+Y on Mac)",
        zip_path.display()
    ))
}
