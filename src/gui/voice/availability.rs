//! Voice feature availability checking.

use std::path::PathBuf;
use std::process::Command;

/// Check availability and return detailed status
pub fn check_availability(model_path: &PathBuf) -> (bool, String) {
    // Check for sox/rec
    let sox_check = Command::new("which").arg("rec").output();

    if sox_check.is_err() || !sox_check.unwrap().status.success() {
        return (
            false,
            "sox not found. Install with: brew install sox".to_string(),
        );
    }

    // Check for whisper (whisper-cli is the binary name from homebrew whisper-cpp)
    let whisper_check = Command::new("which").arg("whisper-cli").output();

    if whisper_check.is_err() || !whisper_check.unwrap().status.success() {
        return (
            false,
            "whisper-cli not found. Install with: brew install whisper-cpp".to_string(),
        );
    }

    // Check for whisper model
    if !model_path.exists() {
        return (
            false,
            format!(
                "Whisper model not found at {}. Click 'Install Voice Dependencies' in Settings.",
                model_path.display()
            ),
        );
    }

    (true, "Voice input ready".to_string())
}
