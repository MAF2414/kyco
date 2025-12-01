//! Voice dependency installation module
//!
//! Handles installation of voice input dependencies (sox, whisper-cpp)
//! via Homebrew on macOS.

use std::path::Path;
use std::process::Command;

/// Result of a voice dependency installation attempt
pub struct VoiceInstallResult {
    /// Status message to display
    pub message: String,
    /// Whether the result is an error
    pub is_error: bool,
    /// Whether installation is still in progress
    pub in_progress: bool,
}

impl VoiceInstallResult {
    fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: false,
            in_progress: false,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: true,
            in_progress: false,
        }
    }

    fn progress(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: false,
            in_progress: true,
        }
    }
}

/// Install voice dependencies (sox, whisper-cpp) via Homebrew
///
/// This function:
/// 1. Checks if Homebrew is available
/// 2. Installs sox for audio recording
/// 3. Installs whisper-cpp for speech-to-text
/// 4. Downloads the base Whisper model
///
/// Returns a `VoiceInstallResult` with status information.
pub fn install_voice_dependencies(work_dir: &Path) -> VoiceInstallResult {
    // Check if brew is available (macOS)
    if Command::new("brew").arg("--version").output().is_err() {
        return VoiceInstallResult::error(
            "Homebrew not found. Please install Homebrew first: https://brew.sh",
        );
    }

    // Step 1: Install sox
    match install_brew_package("sox") {
        Ok(()) => {}
        Err(e) => return VoiceInstallResult::error(format!("Failed to install sox: {}", e)),
    }

    // Step 2: Install whisper-cpp
    match install_brew_package("whisper-cpp") {
        Ok(()) => {}
        Err(e) => {
            return VoiceInstallResult::error(format!("Failed to install whisper-cpp: {}", e))
        }
    }

    // Step 3: Download whisper model (base by default)
    match download_whisper_model(work_dir) {
        Ok(()) => {}
        Err(e) => return VoiceInstallResult::error(e),
    }

    VoiceInstallResult::success(
        "Voice dependencies installed successfully!\nsox + whisper-cpp + base model ready.",
    )
}

/// Install a package via Homebrew
fn install_brew_package(package: &str) -> Result<(), String> {
    let result = Command::new("brew")
        .args(["install", package])
        .output()
        .map_err(|e| e.to_string())?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        // Check if already installed (brew returns error if already installed)
        if !stderr.contains("already installed") {
            return Err(format!("{} installation issue: {}", package, stderr));
        }
    }

    Ok(())
}

/// Download the Whisper base model to the models directory
fn download_whisper_model(work_dir: &Path) -> Result<(), String> {
    // Create models directory in .kyco
    let models_dir = work_dir.join(".kyco").join("whisper-models");
    std::fs::create_dir_all(&models_dir)
        .map_err(|e| format!("Failed to create models directory: {}", e))?;

    let model_path = models_dir.join("ggml-base.bin");
    if model_path.exists() {
        return Ok(());
    }

    // Download the model using curl
    let model_url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin";
    let result = Command::new("curl")
        .args([
            "-L",
            "-o",
            model_path.to_str().unwrap_or("ggml-base.bin"),
            model_url,
        ])
        .output()
        .map_err(|e| format!("Failed to download model: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("Model download failed: {}", stderr));
    }

    Ok(())
}

/// Check current installation status (for progress display)
pub fn check_installation_status() -> VoiceInstallResult {
    VoiceInstallResult::progress("Checking Homebrew...")
}
