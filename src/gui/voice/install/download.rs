//! Whisper model download and validation

use std::path::Path;
use std::process::Command;

use super::models::{get_model_info, WhisperModel};

/// Download a Whisper model to the models directory with checksum validation
pub(super) fn download_whisper_model(work_dir: &Path, model_name: &str) -> Result<(), String> {
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

    is_model_valid(work_dir, model_name, model_info)
}

fn is_model_valid(work_dir: &Path, model_name: &str, model_info: &WhisperModel) -> bool {
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
