//! Voice input types and configuration.

use super::vad::VadConfig;
use super::VoiceActionRegistry;

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
    Disabled,
    /// Manual: Click microphone button or press hotkey to record
    #[default]
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
            mode: VoiceInputMode::Manual,
            keywords,
            action_registry,
            whisper_model: "base".to_string(),
            language: "auto".to_string(),
            silence_threshold: 0.1, // 10% - less sensitive to background noise
            silence_duration: 2.5,  // seconds - avoid cutting off mid-speech
            max_duration: 300.0,    // 5 minutes - safety limit for manual recording
            vad_config: VadConfig::default(),
            use_vad: false, // VAD coming soon - disabled for now
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
    /// - `from_manual`: true if from manual recording (button press), false if from continuous listening
    TranscriptionComplete { text: String, from_manual: bool },
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
