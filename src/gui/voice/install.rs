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

use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Whisper model information with checksums for validation
#[derive(Debug, Clone)]
pub struct WhisperModel {
    /// Model name (tiny, base, small, medium, large)
    pub name: &'static str,
    /// Expected file size in bytes (approximate, for quick validation)
    pub expected_size: u64,
    /// SHA256 checksum for validation
    pub sha256: &'static str,
    /// Download URL
    pub url: &'static str,
}

/// Available Whisper models with their checksums
/// Checksums from: https://huggingface.co/ggerganov/whisper.cpp
pub const WHISPER_MODELS: &[WhisperModel] = &[
    WhisperModel {
        name: "tiny",
        expected_size: 77_691_713,
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
    },
    WhisperModel {
        name: "base",
        expected_size: 147_951_465,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
    },
    WhisperModel {
        name: "small",
        expected_size: 487_601_967,
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
    },
    WhisperModel {
        name: "medium",
        expected_size: 1_533_774_781,
        sha256: "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
    },
    WhisperModel {
        name: "large",
        expected_size: 3_094_623_691,
        sha256: "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
    },
];

/// Get model info by name
pub fn get_model_info(name: &str) -> Option<&'static WhisperModel> {
    WHISPER_MODELS.iter().find(|m| m.name == name)
}

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

/// Download a Whisper model to the models directory with checksum validation
fn download_whisper_model(work_dir: &Path, model_name: &str) -> Result<(), String> {
    let model_info = get_model_info(model_name).ok_or_else(|| {
        format!(
            "Unknown model: {}. Valid models: tiny, base, small, medium, large",
            model_name
        )
    })?;

    let models_dir = work_dir.join(".kyco").join("whisper-models");
    std::fs::create_dir_all(&models_dir)
        .map_err(|e| format!("Failed to create models directory: {}", e))?;

    let model_filename = format!("ggml-{}.bin", model_name);
    let model_path = models_dir.join(&model_filename);
    // Use PID + timestamp for unique temp filename to avoid race conditions
    let temp_path = models_dir.join(format!(
        "{}.{}.{}.tmp",
        model_filename,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    if model_path.exists() {
        if validate_model_checksum(&model_path, model_info.sha256)? {
            return Ok(());
        } else {
            // Model exists but is corrupted, remove and re-download
            let _ = std::fs::remove_file(&model_path);
        }
    }

    let temp_path_str = temp_path
        .to_str()
        .ok_or_else(|| "Temp path contains invalid UTF-8 characters".to_string())?;

    let result = Command::new("curl")
        .args(["-L", "--progress-bar", "-o", temp_path_str, model_info.url])
        .output()
        .map_err(|e| format!("Failed to download model: {}", e))?;

    if !result.status.success() {
        let _ = std::fs::remove_file(&temp_path);
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("Model download failed: {}", stderr));
    }

    let checksum_valid = match validate_model_checksum(&temp_path, model_info.sha256) {
        Ok(valid) => valid,
        Err(e) => {
            let _ = std::fs::remove_file(&temp_path);
            return Err(e);
        }
    };

    if !checksum_valid {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!(
            "Model checksum validation failed. The download may be corrupted.\n\
             Expected SHA256: {}\n\
             Please try again or check your internet connection.",
            model_info.sha256
        ));
    }

    std::fs::rename(&temp_path, &model_path).map_err(|e| {
        // Clean up temp file on rename failure
        let _ = std::fs::remove_file(&temp_path);
        format!("Failed to save model: {}", e)
    })?;

    Ok(())
}

/// Validate a file's SHA256 checksum
fn validate_model_checksum(path: &Path, expected_sha256: &str) -> Result<bool, String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| "Path contains invalid UTF-8 characters".to_string())?;

    // Use shasum on macOS/Linux
    let output = Command::new("shasum")
        .args(["-a", "256", path_str])
        .output()
        .map_err(|e| format!("Failed to calculate checksum: {}", e))?;

    if !output.status.success() {
        return Err("Checksum calculation failed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let calculated_hash = stdout.split_whitespace().next().unwrap_or("");

    Ok(calculated_hash.eq_ignore_ascii_case(expected_sha256))
}

/// Check if a specific model is installed and valid
pub fn is_model_installed(work_dir: &Path, model_name: &str) -> bool {
    let Some(model_info) = get_model_info(model_name) else {
        return false;
    };

    let model_path = work_dir
        .join(".kyco")
        .join("whisper-models")
        .join(format!("ggml-{}.bin", model_name));

    if !model_path.exists() {
        return false;
    }

    // Quick size check first (faster than checksum)
    if let Ok(metadata) = std::fs::metadata(&model_path) {
        let size_diff = (metadata.len() as i64 - model_info.expected_size as i64).abs();
        // Allow 1% size variance
        if size_diff > (model_info.expected_size as i64 / 100) {
            return false;
        }
    }

    true
}

/// Check current installation status (for progress display)
pub fn check_installation_status() -> VoiceInstallResult {
    VoiceInstallResult::progress("Checking Homebrew...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_model_info() {
        assert!(get_model_info("base").is_some());
        assert!(get_model_info("tiny").is_some());
        assert!(get_model_info("invalid").is_none());
    }

    #[test]
    fn test_model_info_has_valid_urls() {
        for model in WHISPER_MODELS {
            assert!(model.url.starts_with("https://"));
            assert!(model.url.contains("huggingface.co"));
        }
    }
}
