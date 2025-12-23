//! Voice Activity Detection (VAD) module
//!
//! Uses Silero VAD model for efficient speech detection.
//! This allows continuous listening without constantly running Whisper,
//! drastically reducing CPU usage.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

/// VAD configuration
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// Sample rate (must be 8000 or 16000 for Silero VAD)
    pub sample_rate: u32,
    /// Probability threshold for speech detection (0.0-1.0)
    pub speech_threshold: f32,
    /// Minimum speech duration to trigger recording (in ms)
    pub min_speech_duration_ms: u32,
    /// Silence duration to stop recording (in ms)
    pub silence_duration_ms: u32,
    /// Chunk size for VAD processing
    pub chunk_size: usize,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            speech_threshold: 0.5,
            min_speech_duration_ms: 250,
            silence_duration_ms: 1000,
            chunk_size: 512,
        }
    }
}

/// VAD events for the main thread
#[derive(Debug, Clone)]
pub enum VadEvent {
    /// Speech started
    SpeechStarted,
    /// Speech ended with audio buffer
    SpeechEnded {
        /// Duration of speech in milliseconds
        duration_ms: u32,
        /// Path to the recorded audio file
        audio_path: PathBuf,
    },
    /// Error occurred
    Error(String),
    /// VAD is ready and listening
    Ready,
    /// VAD stopped
    Stopped,
}

/// Commands to the VAD thread
#[derive(Debug, Clone)]
pub enum VadCommand {
    /// Start listening
    Start,
    /// Stop listening
    Stop,
    /// Update configuration
    UpdateConfig(VadConfig),
}

/// Handle for controlling the VAD listener
pub struct VadHandle {
    /// Channel to send commands
    command_tx: Sender<VadCommand>,
    /// Channel to receive events
    event_rx: Receiver<VadEvent>,
}

impl VadHandle {
    /// Start continuous listening with VAD
    pub fn start(&self) {
        let _ = self.command_tx.send(VadCommand::Start);
    }

    /// Stop listening
    pub fn stop(&self) {
        let _ = self.command_tx.send(VadCommand::Stop);
    }

    /// Poll for events (non-blocking)
    pub fn poll_events(&self) -> Vec<VadEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// Update VAD configuration
    pub fn update_config(&self, config: VadConfig) {
        let _ = self.command_tx.send(VadCommand::UpdateConfig(config));
    }
}

/// State of the VAD listener
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VadState {
    Idle,
    Listening,
    Recording,
}

/// Start a VAD listener in a background thread
///
/// Returns a handle to control the listener and receive events.
pub fn start_vad_listener(work_dir: PathBuf, config: VadConfig) -> VadHandle {
    let (command_tx, command_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    thread::spawn(move || {
        vad_listener_thread(work_dir, config, command_rx, event_tx);
    });

    VadHandle {
        command_tx,
        event_rx,
    }
}

/// VAD listener thread
fn vad_listener_thread(
    work_dir: PathBuf,
    mut config: VadConfig,
    command_rx: Receiver<VadCommand>,
    event_tx: Sender<VadEvent>,
) {
    let mut state = VadState::Idle;
    let mut recording_process: Option<Child> = None;
    let mut speech_start: Option<Instant> = None;

    // Ensure .kyco directory exists
    let kyco_dir = work_dir.join(".kyco");
    let _ = std::fs::create_dir_all(&kyco_dir);

    loop {
        // Check for commands (non-blocking with timeout)
        match command_rx.recv_timeout(Duration::from_millis(10)) {
            Ok(VadCommand::Start) => {
                if state == VadState::Idle {
                    state = VadState::Listening;
                    let _ = event_tx.send(VadEvent::Ready);
                }
            }
            Ok(VadCommand::Stop) => {
                // Stop any recording
                if let Some(mut proc) = recording_process.take() {
                    let _ = proc.kill();
                    let _ = proc.wait();
                }
                state = VadState::Idle;
                let _ = event_tx.send(VadEvent::Stopped);
            }
            Ok(VadCommand::UpdateConfig(new_config)) => {
                config = new_config;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Continue processing
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Channel closed, exit thread
                break;
            }
        }

        // Only process audio when listening
        if state == VadState::Idle {
            continue;
        }

        // For now, we use a simplified approach:
        // Start recording with sox's silence detection
        // In a full implementation, we would:
        // 1. Continuously capture audio chunks
        // 2. Run VAD on each chunk
        // 3. Only start whisper when speech is detected

        if state == VadState::Listening {
            // Start a recording process that waits for speech
            let recording_path = kyco_dir.join("vad_recording.wav");

            // sox rec with silence detection:
            // - silence 1 0.1 1% : wait for sound to start
            // - 1 1.0 1% : stop after 1 second of silence
            let result = Command::new("rec")
                .args([
                    "-r",
                    &config.sample_rate.to_string(),
                    "-c",
                    "1",
                    "-b",
                    "16",
                    recording_path.to_str().unwrap_or("recording.wav"),
                    "silence",
                    "1",
                    "0.1",
                    "1%", // Wait for sound
                    "1",
                    &format!("{:.1}", config.silence_duration_ms as f32 / 1000.0),
                    "1%", // Stop on silence
                    "trim",
                    "0",
                    "30", // Max 30 seconds
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            match result {
                Ok(child) => {
                    recording_process = Some(child);
                    state = VadState::Recording;
                    speech_start = Some(Instant::now());
                    let _ = event_tx.send(VadEvent::SpeechStarted);
                }
                Err(e) => {
                    let _ =
                        event_tx.send(VadEvent::Error(format!("Failed to start recording: {}", e)));
                    state = VadState::Idle;
                }
            }
        }

        // Check if recording finished
        if state == VadState::Recording {
            if let Some(ref mut proc) = recording_process {
                match proc.try_wait() {
                    Ok(Some(_)) => {
                        // Recording finished
                        let duration_ms = speech_start
                            .map(|s| s.elapsed().as_millis() as u32)
                            .unwrap_or(0);

                        let audio_path = kyco_dir.join("vad_recording.wav");
                        if audio_path.exists() {
                            let _ = event_tx.send(VadEvent::SpeechEnded {
                                duration_ms,
                                audio_path,
                            });
                        }

                        recording_process = None;
                        speech_start = None;
                        state = VadState::Listening; // Go back to listening
                    }
                    Ok(None) => {
                        // Still recording
                    }
                    Err(e) => {
                        let _ = event_tx.send(VadEvent::Error(format!("Recording error: {}", e)));
                        recording_process = None;
                        state = VadState::Listening;
                    }
                }
            }
        }
    }
}

/// Check if VAD dependencies are available
pub fn is_vad_available() -> bool {
    // Check for sox/rec
    Command::new("which")
        .arg("rec")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
