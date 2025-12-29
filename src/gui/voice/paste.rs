//! Auto-paste functionality for voice transcription
//!
//! This module provides clipboard + auto-paste functionality that works
//! even when another application (like Claude Code TUI) has focus.
//!
//! Uses arboard for clipboard and osascript for simulating Cmd+V on macOS.

use arboard::Clipboard;
use std::process::Command;

/// Copy text to clipboard and optionally auto-paste into the focused application
pub fn copy_and_paste(text: &str, auto_paste: bool) -> Result<(), String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to copy to clipboard: {}", e))?;

    if auto_paste {
        paste_from_clipboard()?;
    }

    Ok(())
}

/// Simulate Cmd+V (paste) keystroke on macOS
#[cfg(target_os = "macos")]
pub fn paste_from_clipboard() -> Result<(), String> {
    // Small delay to ensure clipboard is ready
    std::thread::sleep(std::time::Duration::from_millis(50));

    let output = Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to keystroke "v" using command down"#,
        ])
        .output()
        .map_err(|e| format!("Failed to run osascript: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("osascript failed: {}", stderr));
    }

    Ok(())
}

/// Simulate Ctrl+V (paste) keystroke on Linux
#[cfg(target_os = "linux")]
pub fn paste_from_clipboard() -> Result<(), String> {
    // Try xdotool first (most common)
    let xdotool_result = Command::new("xdotool").args(["key", "ctrl+v"]).output();

    match xdotool_result {
        Ok(output) if output.status.success() => Ok(()),
        _ => {
            // Fallback to ydotool (for Wayland)
            let ydotool_result = Command::new("ydotool")
                .args(["key", "29:1", "47:1", "47:0", "29:0"]) // Ctrl+V key codes
                .output();

            match ydotool_result {
                Ok(output) if output.status.success() => Ok(()),
                _ => Err("Auto-paste requires xdotool (X11) or ydotool (Wayland). Text copied to clipboard.".to_string()),
            }
        }
    }
}

/// Windows is not supported for auto-paste yet
#[cfg(target_os = "windows")]
pub fn paste_from_clipboard() -> Result<(), String> {
    Err(
        "Auto-paste not supported on Windows yet. Text copied to clipboard - use Ctrl+V to paste."
            .to_string(),
    )
}

/// Fallback for other platforms
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn paste_from_clipboard() -> Result<(), String> {
    Err("Auto-paste not supported on this platform. Text copied to clipboard.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_to_clipboard() {
        // Only test clipboard copy, not paste (requires GUI)
        let result = copy_and_paste("test text", false);
        if let Err(err) = result {
            eprintln!("Skipping clipboard test: {err}");
        }
    }
}
