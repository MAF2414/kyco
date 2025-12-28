//! Whisper transcription functionality.

use std::path::PathBuf;
use std::process::Command;

/// Run whisper-cpp on audio file
pub fn run_whisper(
    audio_path: &PathBuf,
    model_path: &PathBuf,
    language: &str,
) -> Result<String, String> {
    let mut args = vec![
        "-m".to_string(),
        model_path.to_str().unwrap_or("model.bin").to_string(),
        "-f".to_string(),
        audio_path.to_str().unwrap_or("audio.wav").to_string(),
        "--no-timestamps".to_string(),
    ];

    // Always pass the language flag - whisper defaults to English if not specified
    // "auto" tells whisper to auto-detect the language
    args.push("-l".to_string());
    args.push(language.to_string());

    let output = Command::new("whisper-cli")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run whisper: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Whisper failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let text = stdout.trim().to_string();

    if text.is_empty() {
        return Err("No speech detected".to_string());
    }

    Ok(text)
}

/// Parse transcribed text to extract mode and prompt
///
/// Examples:
/// - "refactor this function" -> (Some("refactor"), "this function")
/// - "fix the bug" -> (Some("fix"), "the bug")
/// - "implement a new feature for authentication" -> (Some("implement"), "a new feature for authentication")
pub fn parse_voice_input(text: &str, keywords: &[String]) -> (Option<String>, String) {
    let trimmed = text.trim();
    let trimmed_lower = trimmed.to_lowercase();

    // Check if text starts with a keyword (case-insensitive match)
    for keyword in keywords {
        let keyword_lower = keyword.to_lowercase();
        if trimmed_lower.starts_with(&keyword_lower) {
            // Extract rest from original text to preserve case
            let rest = trimmed[keyword_lower.len()..].trim();
            return (Some(keyword.clone()), rest.to_string());
        }
    }

    // No keyword found - return full text as prompt (preserving original case)
    (None, trimmed.to_string())
}
