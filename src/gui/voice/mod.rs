//! Voice input module for GUI
//!
//! This module provides:
//! 1. Microphone button in selection popup for manual voice recording
//! 2. Hotkey-triggered voice recording with automatic transcription
//! 3. Continuous listening mode with VAD + keyword detection for hands-free operation
//!
//! Architecture:
//! - VoiceState: Current state of voice input (idle, recording, transcribing, listening)
//! - VoiceConfig: Configuration for voice features
//! - VoiceManager: Handles audio capture and transcription coordination
//! - VoiceActionRegistry: Maps wakewords to modes/actions
//! - VAD: Voice Activity Detection for efficient continuous listening
//!
//! Implementation:
//! - Uses `sox` (rec command) for audio recording
//! - Uses `whisper-cli` (from whisper-cpp) for transcription
//! - Uses Silero VAD for voice activity detection

pub mod actions;
pub mod install;
pub mod settings;
pub mod vad;

pub use actions::{VoiceAction, VoiceActionRegistry, WakewordMatch};
pub use install::{
    get_model_info, install_voice_dependencies, install_voice_dependencies_async,
    is_model_installed, InstallHandle, InstallProgress, VoiceInstallResult, WhisperModel,
    WHISPER_MODELS,
};
pub use settings::{render_voice_settings, VoiceSettingsState};
pub use vad::{is_vad_available, start_vad_listener, VadConfig, VadEvent, VadHandle};

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Voice input state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceState {
    /// No voice activity
    #[default]
    Idle,
    /// Recording audio from microphone (manual or hotkey triggered)
    Recording,
    /// Transcribing recorded audio
    Transcribing,
    /// Continuous listening for keywords (always-on mode)
    Listening,
    /// Error state (e.g., no microphone permission)
    Error,
}

impl VoiceState {
    /// Returns true if voice input is actively recording
    pub fn is_recording(&self) -> bool {
        matches!(self, VoiceState::Recording)
    }

    /// Returns true if voice processing is busy
    pub fn is_busy(&self) -> bool {
        matches!(self, VoiceState::Recording | VoiceState::Transcribing)
    }

    /// Returns true if listening for keywords
    pub fn is_listening(&self) -> bool {
        matches!(self, VoiceState::Listening)
    }
}

impl std::fmt::Display for VoiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoiceState::Idle => write!(f, "Idle"),
            VoiceState::Recording => write!(f, "Recording"),
            VoiceState::Transcribing => write!(f, "Transcribing"),
            VoiceState::Listening => write!(f, "Listening"),
            VoiceState::Error => write!(f, "Error"),
        }
    }
}

/// Voice input mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceInputMode {
    /// Voice input disabled
    #[default]
    Disabled,
    /// Manual: Click microphone button or press hotkey to record
    Manual,
    /// Hotkey: Holding the hotkey records, releasing transcribes
    HotkeyHold,
    /// Continuous: Always listening for mode keywords, then records prompt
    Continuous,
}

impl std::fmt::Display for VoiceInputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoiceInputMode::Disabled => write!(f, "Disabled"),
            VoiceInputMode::Manual => write!(f, "Manual (click/hotkey)"),
            VoiceInputMode::HotkeyHold => write!(f, "Hold hotkey to record"),
            VoiceInputMode::Continuous => write!(f, "Always listening"),
        }
    }
}

/// Voice configuration
#[derive(Debug, Clone)]
pub struct VoiceConfig {
    /// Voice input mode
    pub mode: VoiceInputMode,
    /// Keywords to listen for in continuous mode (mode names by default)
    /// DEPRECATED: Use action_registry instead
    pub keywords: Vec<String>,
    /// Voice action registry - maps wakewords to modes
    pub action_registry: VoiceActionRegistry,
    /// Whisper model to use for transcription (tiny, base, small, medium, large)
    pub whisper_model: String,
    /// Language for transcription (auto, en, de, etc.)
    pub language: String,
    /// Silence threshold to stop recording (0.0-1.0)
    pub silence_threshold: f32,
    /// Silence duration to stop recording (in seconds)
    pub silence_duration: f32,
    /// Maximum recording duration (in seconds)
    pub max_duration: f32,
    /// VAD configuration for continuous listening
    pub vad_config: VadConfig,
    /// Use VAD for continuous listening (more efficient)
    pub use_vad: bool,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        let action_registry = VoiceActionRegistry::default();
        let keywords = action_registry.get_all_wakewords();

        Self {
            mode: VoiceInputMode::Disabled,
            keywords,
            action_registry,
            whisper_model: "base".to_string(),
            language: "auto".to_string(),
            silence_threshold: 0.01,
            silence_duration: 1.5,
            max_duration: 30.0,
            vad_config: VadConfig::default(),
            use_vad: true,
        }
    }
}

/// Events from voice processing
#[derive(Debug, Clone)]
pub enum VoiceEvent {
    /// Recording started
    RecordingStarted,
    /// Recording stopped, audio captured
    RecordingStopped { duration_secs: f32 },
    /// Transcription completed
    TranscriptionComplete { text: String },
    /// Keyword detected in continuous mode
    KeywordDetected { keyword: String, full_text: String },
    /// Wakeword matched - triggers a specific mode with prompt
    WakewordMatched {
        /// The wakeword that was matched
        wakeword: String,
        /// The mode to trigger
        mode: String,
        /// The prompt (text after wakeword)
        prompt: String,
    },
    /// Error occurred
    Error { message: String },
    /// State changed
    StateChanged(VoiceState),
    /// VAD detected speech start
    VadSpeechStarted,
    /// VAD detected speech end
    VadSpeechEnded,
}

/// Commands to voice processing
#[derive(Debug, Clone)]
pub enum VoiceCommand {
    /// Start recording
    StartRecording,
    /// Stop recording and transcribe
    StopRecording,
    /// Cancel current operation
    Cancel,
    /// Start continuous listening
    StartListening { keywords: Vec<String> },
    /// Stop continuous listening
    StopListening,
    /// Update configuration
    UpdateConfig(VoiceConfig),
}

/// Voice manager that uses sox and whisper-cpp
pub struct VoiceManager {
    /// Current state
    pub state: VoiceState,
    /// Configuration
    pub config: VoiceConfig,
    /// Working directory for temp files
    work_dir: PathBuf,
    /// Recording process handle
    recording_process: Option<Child>,
    /// Path to current recording
    recording_path: Option<PathBuf>,
    /// Event receiver (from transcription thread)
    event_rx: Option<Receiver<VoiceEvent>>,
    /// Event sender (for transcription thread)
    event_tx: Option<Sender<VoiceEvent>>,
    /// Last error message
    pub last_error: Option<String>,
    /// Last transcribed text
    pub last_transcription: Option<String>,
    /// Cached availability status
    availability_cache: Option<(bool, String)>,
}

impl Default for VoiceManager {
    fn default() -> Self {
        Self::new(VoiceConfig::default())
    }
}

impl VoiceManager {
    /// Create a new voice manager
    pub fn new(config: VoiceConfig) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            state: VoiceState::Idle,
            config,
            work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            recording_process: None,
            recording_path: None,
            event_rx: Some(rx),
            event_tx: Some(tx),
            last_error: None,
            last_transcription: None,
            availability_cache: None,
        }
    }

    /// Set the working directory
    pub fn set_work_dir(&mut self, work_dir: PathBuf) {
        self.work_dir = work_dir;
        // Invalidate cache when work_dir changes
        self.availability_cache = None;
    }

    /// Check if voice features are available
    ///
    /// This checks for:
    /// 1. sox/rec command available
    /// 2. whisper command available
    /// 3. Whisper model file exists
    pub fn is_available(&mut self) -> bool {
        // Return cached result if available
        if let Some((available, _)) = &self.availability_cache {
            return *available;
        }

        let (available, message) = self.check_availability();
        self.availability_cache = Some((available, message));
        available
    }

    /// Check availability and return detailed status
    fn check_availability(&self) -> (bool, String) {
        // Check for sox/rec
        let sox_check = Command::new("which")
            .arg("rec")
            .output();

        if sox_check.is_err() || !sox_check.unwrap().status.success() {
            return (false, "sox not found. Install with: brew install sox".to_string());
        }

        // Check for whisper (whisper-cli is the binary name from homebrew whisper-cpp)
        let whisper_check = Command::new("which")
            .arg("whisper-cli")
            .output();

        if whisper_check.is_err() || !whisper_check.unwrap().status.success() {
            return (false, "whisper-cli not found. Install with: brew install whisper-cpp".to_string());
        }

        // Check for whisper model
        let model_path = self.get_model_path();
        if !model_path.exists() {
            return (false, format!(
                "Whisper model not found at {}. Click 'Install Voice Dependencies' in Settings.",
                model_path.display()
            ));
        }

        (true, "Voice input ready".to_string())
    }

    /// Get availability status message
    pub fn availability_status(&mut self) -> String {
        if let Some((_, ref message)) = self.availability_cache {
            return message.clone();
        }

        let (_, message) = self.check_availability();
        message
    }

    /// Get path to whisper model
    fn get_model_path(&self) -> PathBuf {
        let model_name = format!("ggml-{}.bin", self.config.whisper_model);
        self.work_dir.join(".kyco").join("whisper-models").join(model_name)
    }

    /// Get path for temporary recording
    fn get_recording_path(&self) -> PathBuf {
        self.work_dir.join(".kyco").join("voice_recording.wav")
    }

    /// Start recording
    pub fn start_recording(&mut self) {
        if !self.is_available() {
            self.state = VoiceState::Error;
            self.last_error = Some(self.availability_status());
            return;
        }

        // Ensure .kyco directory exists
        let kyco_dir = self.work_dir.join(".kyco");
        if let Err(e) = std::fs::create_dir_all(&kyco_dir) {
            self.state = VoiceState::Error;
            self.last_error = Some(format!("Failed to create .kyco directory: {}", e));
            return;
        }

        let recording_path = self.get_recording_path();

        // Start sox recording
        // rec -r 16000 -c 1 -b 16 output.wav silence 1 0.1 1% 1 1.5 1%
        // This records at 16kHz mono (required by whisper), and stops on silence
        let silence_duration = self.config.silence_duration;
        let max_duration = self.config.max_duration;

        let result = Command::new("rec")
            .args([
                "-r", "16000",           // 16kHz sample rate (whisper requirement)
                "-c", "1",               // Mono
                "-b", "16",              // 16-bit
                recording_path.to_str().unwrap_or("recording.wav"),
                "trim", "0", &format!("{}", max_duration),  // Max duration
                "silence", "1", "0.1", "1%",  // Start on sound
                "1", &format!("{}", silence_duration), "1%",  // Stop on silence
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match result {
            Ok(child) => {
                self.recording_process = Some(child);
                self.recording_path = Some(recording_path);
                self.state = VoiceState::Recording;
                self.last_error = None;
            }
            Err(e) => {
                self.state = VoiceState::Error;
                self.last_error = Some(format!("Failed to start recording: {}", e));
            }
        }
    }

    /// Stop recording and start transcription
    pub fn stop_recording(&mut self) {
        // Stop the recording process gracefully
        if let Some(mut process) = self.recording_process.take() {
            // First try SIGTERM for graceful shutdown (allows sox to finalize the WAV file)
            #[cfg(unix)]
            {
                // Use the kill command to send SIGTERM
                let _ = Command::new("kill")
                    .args(["-TERM", &process.id().to_string()])
                    .output();
                // Give sox a moment to finalize the file
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            // Then ensure process is terminated (fallback)
            let _ = process.kill();
            let _ = process.wait();
        }

        // Start transcription if we have a recording
        if let Some(recording_path) = self.recording_path.take() {
            if recording_path.exists() {
                self.state = VoiceState::Transcribing;
                self.transcribe_async(recording_path);
            } else {
                self.state = VoiceState::Error;
                self.last_error = Some("Recording file not found".to_string());
            }
        } else {
            self.state = VoiceState::Idle;
        }
    }

    /// Run transcription in background thread
    fn transcribe_async(&mut self, audio_path: PathBuf) {
        let model_path = self.get_model_path();
        let language = self.config.language.clone();
        let event_tx = self.event_tx.clone();

        thread::spawn(move || {
            let result = Self::run_whisper(&audio_path, &model_path, &language);

            if let Some(tx) = event_tx {
                match result {
                    Ok(text) => {
                        let _ = tx.send(VoiceEvent::TranscriptionComplete { text });
                    }
                    Err(e) => {
                        let _ = tx.send(VoiceEvent::Error { message: e });
                    }
                }
            }

            // Clean up the recording file
            let _ = std::fs::remove_file(&audio_path);
        });
    }

    /// Run whisper-cpp on audio file
    fn run_whisper(audio_path: &PathBuf, model_path: &PathBuf, language: &str) -> Result<String, String> {
        let mut args = vec![
            "-m".to_string(),
            model_path.to_str().unwrap_or("model.bin").to_string(),
            "-f".to_string(),
            audio_path.to_str().unwrap_or("audio.wav").to_string(),
            "--no-timestamps".to_string(),
        ];

        // Add language if not auto
        if language != "auto" {
            args.push("-l".to_string());
            args.push(language.to_string());
        }

        let output = Command::new("whisper-cli")
            .args(&args)
            .output()
            .map_err(|e| format!("Failed to run whisper: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Whisper failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse whisper output - it outputs the transcription directly
        let text = stdout.trim().to_string();

        if text.is_empty() {
            return Err("No speech detected".to_string());
        }

        Ok(text)
    }

    /// Cancel current operation
    pub fn cancel(&mut self) {
        // Kill any running recording process
        if let Some(mut process) = self.recording_process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }

        // Clean up recording file
        if let Some(path) = self.recording_path.take() {
            let _ = std::fs::remove_file(&path);
        }

        self.state = VoiceState::Idle;
        self.last_error = None;
    }

    /// Clear error state and return to idle
    ///
    /// Use this to recover from an error state without starting a new operation.
    pub fn clear_error(&mut self) {
        if self.state == VoiceState::Error {
            self.state = VoiceState::Idle;
            self.last_error = None;
        }
    }

    /// Reset the voice manager to a clean state
    ///
    /// Cancels any ongoing operation, clears errors, and returns to idle.
    pub fn reset(&mut self) {
        self.cancel();
        self.last_error = None;
        self.last_transcription = None;
        self.availability_cache = None;
    }

    /// Start continuous listening mode
    pub fn start_listening(&mut self) {
        if !self.is_available() {
            self.state = VoiceState::Error;
            self.last_error = Some(self.availability_status());
            return;
        }

        // For now, continuous listening just starts a recording
        // A full implementation would run whisper in streaming mode
        self.start_recording();
        self.state = VoiceState::Listening;
    }

    /// Stop continuous listening
    pub fn stop_listening(&mut self) {
        self.stop_recording();
    }

    /// Poll for events (call each frame)
    pub fn poll_events(&mut self) -> Vec<VoiceEvent> {
        let mut events = Vec::new();

        // Check if recording process finished on its own (silence detection)
        if self.state == VoiceState::Recording {
            if let Some(ref mut process) = self.recording_process {
                match process.try_wait() {
                    Ok(Some(_status)) => {
                        // Recording finished (silence detected)
                        self.recording_process = None;
                        if let Some(recording_path) = self.recording_path.take() {
                            if recording_path.exists() {
                                self.state = VoiceState::Transcribing;
                                self.transcribe_async(recording_path);
                                events.push(VoiceEvent::RecordingStopped { duration_secs: 0.0 });
                            }
                        }
                    }
                    Ok(None) => {
                        // Still recording
                    }
                    Err(e) => {
                        self.state = VoiceState::Error;
                        self.last_error = Some(format!("Recording error: {}", e));
                        self.recording_process = None;
                    }
                }
            }
        }

        // Poll for events from transcription thread
        if let Some(ref rx) = self.event_rx {
            while let Ok(event) = rx.try_recv() {
                match &event {
                    VoiceEvent::StateChanged(state) => {
                        self.state = *state;
                    }
                    VoiceEvent::TranscriptionComplete { text } => {
                        self.last_transcription = Some(text.clone());
                        self.state = VoiceState::Idle;
                    }
                    VoiceEvent::Error { message } => {
                        self.last_error = Some(message.clone());
                        self.state = VoiceState::Error;
                    }
                    _ => {}
                }
                events.push(event);
            }
        }

        events
    }

    /// Update configuration
    pub fn update_config(&mut self, config: VoiceConfig) {
        // Invalidate cache if model changed
        if self.config.whisper_model != config.whisper_model {
            self.availability_cache = None;
        }
        self.config = config;
    }

    /// Toggle recording (for microphone button)
    pub fn toggle_recording(&mut self) {
        match self.state {
            VoiceState::Idle | VoiceState::Error => self.start_recording(),
            VoiceState::Recording => self.stop_recording(),
            _ => {}
        }
    }

    /// Toggle continuous listening
    pub fn toggle_listening(&mut self) {
        match self.state {
            VoiceState::Idle => self.start_listening(),
            VoiceState::Listening => self.stop_listening(),
            _ => {}
        }
    }

    /// Take the last transcription (consumes it)
    pub fn take_transcription(&mut self) -> Option<String> {
        self.last_transcription.take()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_voice_input_with_keyword() {
        let keywords = vec!["refactor".to_string(), "fix".to_string()];

        let (mode, prompt) = parse_voice_input("refactor this function", &keywords);
        assert_eq!(mode, Some("refactor".to_string()));
        assert_eq!(prompt, "this function");

        let (mode, prompt) = parse_voice_input("Fix the bug in auth", &keywords);
        assert_eq!(mode, Some("fix".to_string()));
        assert_eq!(prompt, "the bug in auth");
    }

    #[test]
    fn test_parse_voice_input_without_keyword() {
        let keywords = vec!["refactor".to_string()];

        let (mode, prompt) = parse_voice_input("hello world", &keywords);
        assert_eq!(mode, None);
        assert_eq!(prompt, "hello world");
    }

    #[test]
    fn test_parse_voice_input_preserves_case() {
        let keywords = vec!["fix".to_string()];

        // Input has mixed case - prompt should preserve original case
        let (mode, prompt) = parse_voice_input("FIX the AuthController Bug", &keywords);
        assert_eq!(mode, Some("fix".to_string()));
        assert_eq!(prompt, "the AuthController Bug");
    }

    #[test]
    fn test_voice_state_display() {
        assert_eq!(VoiceState::Recording.to_string(), "Recording");
        assert_eq!(VoiceState::Listening.to_string(), "Listening");
    }
}
