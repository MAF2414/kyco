//! Voice microphone testing functionality

use eframe::egui::{self, Color32, RichText};

use crate::gui::settings::state::{SettingsState, VoiceTestStatus};
use crate::gui::theme::{ACCENT_CYAN, ACCENT_GREEN, ACCENT_RED, TEXT_DIM, TEXT_MUTED, TEXT_PRIMARY};

/// Render voice test section with microphone test button and status
pub fn render_voice_test_section(ui: &mut egui::Ui, state: &mut SettingsState<'_>) {
    ui.label(RichText::new("Test Microphone").color(TEXT_PRIMARY));
    ui.add_space(4.0);
    ui.label(
        RichText::new("Test recording and transcription. This will also request microphone permission if needed.")
            .small()
            .color(TEXT_MUTED),
    );
    ui.add_space(8.0);

    // Check dependencies status
    let sox_available = std::process::Command::new("which")
        .arg("rec")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let whisper_available = std::process::Command::new("which")
        .arg("whisper-cli")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let model_name = if state.voice_settings_model.is_empty() {
        "base"
    } else {
        state.voice_settings_model.as_str()
    };
    let model_path = state
        .work_dir
        .join(".kyco")
        .join("whisper-models")
        .join(format!("ggml-{}.bin", model_name));
    let model_available = model_path.exists();

    render_dependency_status(ui, sox_available, whisper_available, model_available, model_name);
    ui.add_space(8.0);

    let all_deps_available = sox_available && whisper_available && model_available;
    render_test_button(ui, state, all_deps_available);
    render_test_result(ui, state);
}

/// Render dependency status icons
fn render_dependency_status(
    ui: &mut egui::Ui,
    sox_available: bool,
    whisper_available: bool,
    model_available: bool,
    model_name: &str,
) {
    ui.horizontal(|ui| {
        let (sox_icon, sox_color) = if sox_available {
            ("✓", ACCENT_GREEN)
        } else {
            ("✗", ACCENT_RED)
        };
        ui.label(RichText::new(sox_icon).color(sox_color));
        ui.label(RichText::new("sox").monospace().color(TEXT_MUTED));
        ui.add_space(12.0);

        let (whisper_icon, whisper_color) = if whisper_available {
            ("✓", ACCENT_GREEN)
        } else {
            ("✗", ACCENT_RED)
        };
        ui.label(RichText::new(whisper_icon).color(whisper_color));
        ui.label(RichText::new("whisper-cli").monospace().color(TEXT_MUTED));
        ui.add_space(12.0);

        let (model_icon, model_color) = if model_available {
            ("✓", ACCENT_GREEN)
        } else {
            ("✗", ACCENT_RED)
        };
        ui.label(RichText::new(model_icon).color(model_color));
        ui.label(
            RichText::new(format!("{} model", model_name))
                .monospace()
                .color(TEXT_MUTED),
        );
    });
}

/// Render test microphone button
fn render_test_button(ui: &mut egui::Ui, state: &mut SettingsState<'_>, all_deps_available: bool) {
    let is_testing = matches!(
        state.voice_test_status,
        VoiceTestStatus::Recording | VoiceTestStatus::Transcribing
    );

    let button_text = match &*state.voice_test_status {
        VoiceTestStatus::Idle => "Test Microphone (3 sec)",
        VoiceTestStatus::Recording => "● Recording...",
        VoiceTestStatus::Transcribing => "◌ Transcribing...",
        VoiceTestStatus::Success => "Test Again",
        VoiceTestStatus::Error(_) => "Try Again",
    };

    let button_enabled = all_deps_available && !is_testing;

    ui.horizontal(|ui| {
        let button = ui.add_enabled(
            button_enabled,
            egui::Button::new(RichText::new(button_text).color(if button_enabled {
                ACCENT_CYAN
            } else {
                TEXT_MUTED
            })),
        );

        if button.clicked() {
            start_voice_test(state);
        }

        if !all_deps_available {
            ui.label(
                RichText::new("(install dependencies first)")
                    .small()
                    .color(TEXT_DIM),
            );
        }
    });
}

/// Render test result (success or error)
fn render_test_result(ui: &mut egui::Ui, state: &SettingsState<'_>) {
    match &*state.voice_test_status {
        VoiceTestStatus::Success => {
            if let Some(result) = &state.voice_test_result {
                ui.add_space(8.0);
                egui::Frame::NONE
                    .fill(Color32::from_rgb(20, 40, 20))
                    .corner_radius(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("✓ Transcription:").color(ACCENT_GREEN));
                        ui.label(RichText::new(result).color(TEXT_PRIMARY));
                    });
            }
        }
        VoiceTestStatus::Error(msg) => {
            ui.add_space(8.0);
            egui::Frame::NONE
                .fill(Color32::from_rgb(40, 20, 20))
                .corner_radius(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.label(RichText::new(format!("✗ {}", msg)).color(ACCENT_RED));
                });
        }
        _ => {}
    }
}

/// Start voice test - records 3 seconds and transcribes
fn start_voice_test(state: &mut SettingsState<'_>) {
    *state.voice_test_status = VoiceTestStatus::Recording;
    *state.voice_test_result = None;

    let work_dir = state.work_dir.to_path_buf();
    let model_name = if state.voice_settings_model.is_empty() {
        "base".to_string()
    } else {
        state.voice_settings_model.clone()
    };
    let language = state.voice_settings_language.clone();

    // Run test synchronously (blocks UI briefly but avoids thread resource leak)
    // For async version, would need channels or Arc<Mutex> for state updates
    match run_voice_test_sync(&work_dir, &model_name, &language) {
        Ok(text) => {
            *state.voice_test_status = VoiceTestStatus::Success;
            *state.voice_test_result = Some(text);
        }
        Err(e) => {
            *state.voice_test_status = VoiceTestStatus::Error(e);
        }
    }
}

/// Run voice test synchronously (3 second recording + transcription)
fn run_voice_test_sync(
    work_dir: &std::path::Path,
    model_name: &str,
    language: &str,
) -> Result<String, String> {
    use std::process::Command;

    let kyco_dir = work_dir.join(".kyco");
    std::fs::create_dir_all(&kyco_dir).map_err(|e| format!("Failed to create .kyco dir: {}", e))?;

    let recording_path = kyco_dir.join("voice_test.wav");
    let model_path = kyco_dir
        .join("whisper-models")
        .join(format!("ggml-{}.bin", model_name));

    // Convert paths to strings, returning error if invalid UTF-8
    let recording_path_str = recording_path
        .to_str()
        .ok_or_else(|| "Recording path contains invalid UTF-8 characters".to_string())?;
    let model_path_str = model_path
        .to_str()
        .ok_or_else(|| "Model path contains invalid UTF-8 characters".to_string())?;

    // Record 3 seconds of audio
    let rec_result = Command::new("rec")
        .args([
            "-r",
            "16000", // 16kHz sample rate (whisper requirement)
            "-c",
            "1", // Mono
            "-b",
            "16", // 16-bit
            recording_path_str,
            "trim",
            "0",
            "3", // Record exactly 3 seconds
        ])
        .output();

    match rec_result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Recording failed: {}", stderr));
            }
        }
        Err(e) => {
            return Err(format!("Failed to start recording: {}", e));
        }
    }

    if !recording_path.exists() {
        return Err("Recording file not created".to_string());
    }

    let mut whisper_args = vec![
        "-m".to_string(),
        model_path_str.to_string(),
        "-f".to_string(),
        recording_path_str.to_string(),
        "--no-timestamps".to_string(),
    ];

    if language != "auto" {
        whisper_args.push("-l".to_string());
        whisper_args.push(language.to_string());
    }

    let whisper_result = Command::new("whisper-cli").args(&whisper_args).output();

    let _ = std::fs::remove_file(&recording_path);

    match whisper_result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Transcription failed: {}", stderr));
            }
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if text.is_empty() {
                Err("No speech detected".to_string())
            } else {
                Ok(text)
            }
        }
        Err(e) => Err(format!("Failed to run whisper: {}", e)),
    }
}
