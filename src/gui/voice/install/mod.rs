//! Voice dependency installation module
//!
//! Handles installation of voice input dependencies (sox, whisper-cpp)
//! via Homebrew on macOS.
//!
//! ## Platform Support
//!
//! Currently only macOS is supported via Homebrew. The following dependencies are required:
//! - `sox`: For audio recording (provides the `rec` command)
//! - `whisper-cpp`: For speech-to-text transcription
//!
//! For other platforms:
//! - **Linux**: Install sox via your package manager (apt, dnf, pacman) and build whisper-cpp from source
//! - **Windows**: Not currently supported. Consider using WSL2 with the Linux instructions.

mod download;
mod models;
mod result;

pub use download::is_model_installed;
pub use models::{get_model_info, WhisperModel, WHISPER_MODELS};
pub use result::{check_installation_status, InstallHandle, InstallProgress, VoiceInstallResult};

use download::download_whisper_model;

use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{self, Sender};
use std::thread;

/// Start async installation of voice dependencies
///
/// Returns an `InstallHandle` that can be polled for progress updates.
/// This allows the UI to remain responsive during installation.
pub fn install_voice_dependencies_async(work_dir: &Path, model_name: &str) -> InstallHandle {
    let (tx, rx) = mpsc::channel();
    let work_dir = work_dir.to_path_buf();
    let model_name = model_name.to_string();

    thread::spawn(move || {
        install_voice_dependencies_inner(&work_dir, &model_name, tx);
    });

    InstallHandle { progress_rx: rx }
}

/// Inner installation function that runs in a background thread
fn install_voice_dependencies_inner(
    work_dir: &Path,
    model_name: &str,
    tx: Sender<InstallProgress>,
) {
    let total_steps = 4;

    let _ = tx.send(InstallProgress::Step {
        step: 1,
        total: total_steps,
        message: "Checking Homebrew...".to_string(),
    });

    if Command::new("brew").arg("--version").output().is_err() {
        let _ = tx.send(InstallProgress::Failed(VoiceInstallResult::error(
            "Homebrew not found. Please install Homebrew first: https://brew.sh\n\n\
             For other platforms:\n\
             - Linux: Install sox via package manager, build whisper-cpp from source\n\
             - Windows: Use WSL2 with Linux instructions",
        )));
        return;
    }

    let _ = tx.send(InstallProgress::Step {
        step: 2,
        total: total_steps,
        message: "Installing sox...".to_string(),
    });

    if let Err(e) = install_brew_package("sox") {
        let _ = tx.send(InstallProgress::Failed(VoiceInstallResult::error(format!(
            "Failed to install sox: {}",
            e
        ))));
        return;
    }

    let _ = tx.send(InstallProgress::Step {
        step: 3,
        total: total_steps,
        message: "Installing whisper-cpp...".to_string(),
    });

    if let Err(e) = install_brew_package("whisper-cpp") {
        let _ = tx.send(InstallProgress::Failed(VoiceInstallResult::error(format!(
            "Failed to install whisper-cpp: {}",
            e
        ))));
        return;
    }

    let _ = tx.send(InstallProgress::Step {
        step: 4,
        total: total_steps,
        message: format!(
            "Downloading {} model (this may take a while)...",
            model_name
        ),
    });

    if let Err(e) = download_whisper_model(work_dir, model_name) {
        let _ = tx.send(InstallProgress::Failed(VoiceInstallResult::error(e)));
        return;
    }

    let _ = tx.send(InstallProgress::Complete(VoiceInstallResult::success(
        format!(
            "Voice dependencies installed successfully!\nsox + whisper-cpp + {} model ready.",
            model_name
        ),
    )));
}

/// Install voice dependencies synchronously (blocking)
///
/// For non-blocking installation, use `install_voice_dependencies_async` instead.
pub fn install_voice_dependencies(work_dir: &Path, model_name: &str) -> VoiceInstallResult {
    if Command::new("brew").arg("--version").output().is_err() {
        return VoiceInstallResult::error(
            "Homebrew not found. Please install Homebrew first: https://brew.sh\n\n\
             For other platforms:\n\
             - Linux: Install sox via package manager, build whisper-cpp from source\n\
             - Windows: Use WSL2 with Linux instructions",
        );
    }

    if let Err(e) = install_brew_package("sox") {
        return VoiceInstallResult::error(format!("Failed to install sox: {}", e));
    }

    if let Err(e) = install_brew_package("whisper-cpp") {
        return VoiceInstallResult::error(format!("Failed to install whisper-cpp: {}", e));
    }

    if let Err(e) = download_whisper_model(work_dir, model_name) {
        return VoiceInstallResult::error(e);
    }

    VoiceInstallResult::success(format!(
        "Voice dependencies installed successfully!\nsox + whisper-cpp + {} model ready.",
        model_name
    ))
}

/// Install a package via Homebrew
fn install_brew_package(package: &str) -> Result<(), String> {
    let result = Command::new("brew")
        .args(["install", package])
        .output()
        .map_err(|e| e.to_string())?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        // Check if already installed (brew returns success even if already installed in newer versions)
        if !stderr.contains("already installed") && !stderr.is_empty() {
            return Err(format!("{} installation issue: {}", package, stderr));
        }
    }

    Ok(())
}
