//! Recording control functionality for voice input.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Start a recording process with sox/rec
///
/// Returns the child process on success, or an error message on failure.
pub fn start_recording_process(
    recording_path: &PathBuf,
    max_duration: f32,
) -> Result<Child, String> {
    Command::new("rec")
        .args([
            "-r",
            "16000", // 16kHz sample rate (whisper requirement)
            "-c",
            "1", // Mono
            "-b",
            "16", // 16-bit
            recording_path.to_str().unwrap_or("recording.wav"),
            "trim",
            "0",
            &format!("{}", max_duration),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start recording: {}", e))
}

/// Stop a recording process gracefully
///
/// First tries SIGTERM for graceful shutdown (allows sox to finalize the WAV file),
/// then falls back to kill.
pub fn stop_recording_process(mut process: Child) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &process.id().to_string()])
            .output();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let _ = process.kill();
    let _ = process.wait();
}

/// Cancel a recording process (immediate kill)
pub fn cancel_recording_process(mut process: Child) {
    let _ = process.kill();
    let _ = process.wait();
}

/// Gracefully terminate a recording process on drop
///
/// This variant uses a shorter delay suitable for cleanup during drop.
pub fn terminate_recording_process(mut process: Child) {
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &process.id().to_string()])
            .output();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let _ = process.kill();
    let _ = process.wait();
}
