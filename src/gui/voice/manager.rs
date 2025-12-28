//! Voice manager implementation using sox and whisper-cpp.

use std::path::PathBuf;
use std::process::Child;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use super::availability::check_availability;
use super::recording::{
    cancel_recording_process, start_recording_process, stop_recording_process,
    terminate_recording_process,
};
use super::transcription::run_whisper;
use super::types::{VoiceConfig, VoiceEvent, VoiceState};

/// Voice manager that uses sox and whisper-cpp
pub struct VoiceManager {
    pub state: VoiceState,
    pub config: VoiceConfig,
    work_dir: PathBuf,
    recording_process: Option<Child>,
    recording_path: Option<PathBuf>,
    event_rx: Option<Receiver<VoiceEvent>>,
    event_tx: Option<Sender<VoiceEvent>>,
    pub last_error: Option<String>,
    pub last_transcription: Option<String>,
    availability_cache: Option<(bool, String)>,
    is_manual_recording: bool,
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
            is_manual_recording: false,
        }
    }

    /// Set the working directory
    pub fn set_work_dir(&mut self, work_dir: PathBuf) {
        self.work_dir = work_dir;
        self.availability_cache = None;
    }

    /// Check if voice features are available
    pub fn is_available(&mut self) -> bool {
        if let Some((available, _)) = &self.availability_cache {
            return *available;
        }
        let (available, message) = check_availability(&self.get_model_path());
        self.availability_cache = Some((available, message));
        available
    }

    /// Get availability status message
    pub fn availability_status(&mut self) -> String {
        if let Some((_, ref message)) = self.availability_cache {
            return message.clone();
        }
        let (_, message) = check_availability(&self.get_model_path());
        message
    }

    fn get_model_path(&self) -> PathBuf {
        let model_name = format!("ggml-{}.bin", self.config.whisper_model);
        self.work_dir.join(".kyco").join("whisper-models").join(model_name)
    }

    fn get_recording_path(&self) -> PathBuf {
        self.work_dir.join(".kyco").join("voice_recording.wav")
    }

    /// Start recording (manual mode - no wakeword detection)
    pub fn start_recording(&mut self) {
        self.start_recording_internal(true);
    }

    fn start_recording_internal(&mut self, is_manual: bool) {
        if !self.is_available() {
            self.state = VoiceState::Error;
            self.last_error = Some(self.availability_status());
            return;
        }

        let kyco_dir = self.work_dir.join(".kyco");
        if let Err(e) = std::fs::create_dir_all(&kyco_dir) {
            self.state = VoiceState::Error;
            self.last_error = Some(format!("Failed to create .kyco directory: {}", e));
            return;
        }

        let recording_path = self.get_recording_path();
        match start_recording_process(&recording_path, self.config.max_duration) {
            Ok(child) => {
                self.recording_process = Some(child);
                self.recording_path = Some(recording_path);
                self.state = VoiceState::Recording;
                self.last_error = None;
                self.is_manual_recording = is_manual;
            }
            Err(e) => {
                self.state = VoiceState::Error;
                self.last_error = Some(e);
            }
        }
    }

    /// Stop recording and start transcription
    pub fn stop_recording(&mut self) {
        if let Some(process) = self.recording_process.take() {
            stop_recording_process(process);
        }

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

    fn transcribe_async(&mut self, audio_path: PathBuf) {
        let model_path = self.get_model_path();
        let language = self.config.language.clone();
        let event_tx = self.event_tx.clone();
        let from_manual = self.is_manual_recording;

        thread::spawn(move || {
            let result = run_whisper(&audio_path, &model_path, &language);
            if let Some(tx) = event_tx {
                match result {
                    Ok(text) => {
                        let _ = tx.send(VoiceEvent::TranscriptionComplete { text, from_manual });
                    }
                    Err(e) => {
                        let _ = tx.send(VoiceEvent::Error { message: e });
                    }
                }
            }
            let _ = std::fs::remove_file(&audio_path);
        });
    }

    /// Cancel current operation
    pub fn cancel(&mut self) {
        if let Some(process) = self.recording_process.take() {
            cancel_recording_process(process);
        }
        if let Some(path) = self.recording_path.take() {
            let _ = std::fs::remove_file(&path);
        }
        self.state = VoiceState::Idle;
        self.last_error = None;
    }

    /// Clear error state and return to idle
    pub fn clear_error(&mut self) {
        if self.state == VoiceState::Error {
            self.state = VoiceState::Idle;
            self.last_error = None;
        }
    }

    /// Reset the voice manager to a clean state
    pub fn reset(&mut self) {
        self.cancel();
        self.last_error = None;
        self.last_transcription = None;
        self.availability_cache = None;
    }

    /// Start continuous listening mode (with wakeword detection)
    pub fn start_listening(&mut self) {
        if !self.is_available() {
            self.state = VoiceState::Error;
            self.last_error = Some(self.availability_status());
            return;
        }
        self.start_recording_internal(false);
        self.state = VoiceState::Listening;
    }

    /// Stop continuous listening
    pub fn stop_listening(&mut self) {
        self.stop_recording();
    }

    /// Poll for events (call each frame)
    pub fn poll_events(&mut self) -> Vec<VoiceEvent> {
        let mut events = Vec::new();

        if self.state == VoiceState::Recording {
            if let Some(ref mut process) = self.recording_process {
                match process.try_wait() {
                    Ok(Some(_status)) => {
                        self.recording_process = None;
                        if let Some(recording_path) = self.recording_path.take() {
                            if recording_path.exists() {
                                self.state = VoiceState::Transcribing;
                                self.transcribe_async(recording_path);
                                events.push(VoiceEvent::RecordingStopped { duration_secs: 0.0 });
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        self.state = VoiceState::Error;
                        self.last_error = Some(format!("Recording error: {}", e));
                        self.recording_process = None;
                    }
                }
            }
        }

        if let Some(ref rx) = self.event_rx {
            while let Ok(event) = rx.try_recv() {
                match &event {
                    VoiceEvent::StateChanged(state) => self.state = *state,
                    VoiceEvent::TranscriptionComplete { text, .. } => {
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

impl Drop for VoiceManager {
    fn drop(&mut self) {
        if let Some(process) = self.recording_process.take() {
            terminate_recording_process(process);
        }
        if let Some(path) = self.recording_path.take() {
            let _ = std::fs::remove_file(&path);
        }
    }
}
