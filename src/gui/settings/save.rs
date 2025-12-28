//! Settings save functionality
//!
//! Handles validation and persistence of settings to config file.

use std::sync::atomic::Ordering;

use super::state::SettingsState;
use crate::config::Config;

/// Save settings to config file
///
/// Uses proper TOML serialization to avoid config file corruption.
pub fn save_settings_to_config(state: &mut SettingsState<'_>) {
    let max_concurrent = match state.settings_max_concurrent.trim().parse::<usize>() {
        Ok(n) if n > 0 => n,
        _ => {
            *state.settings_status = Some((
                "Invalid max concurrent jobs (must be > 0)".to_string(),
                true,
            ));
            return;
        }
    };

    let silence_threshold = match state.voice_settings_silence_threshold.trim().parse::<f32>() {
        Ok(n) if (0.0..=1.0).contains(&n) => n,
        _ => {
            *state.settings_status = Some((
                "Invalid silence threshold (must be 0.0-1.0)".to_string(),
                true,
            ));
            return;
        }
    };

    let silence_duration = match state.voice_settings_silence_duration.trim().parse::<f32>() {
        Ok(n) if n > 0.0 => n,
        _ => {
            *state.settings_status =
                Some(("Invalid silence duration (must be > 0)".to_string(), true));
            return;
        }
    };

    let max_duration = match state.voice_settings_max_duration.trim().parse::<f32>() {
        Ok(n) if n > 0.0 => n,
        _ => {
            *state.settings_status = Some(("Invalid max duration (must be > 0)".to_string(), true));
            return;
        }
    };

    let voice_keywords: Vec<String> = state
        .voice_settings_keywords
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Clone config and apply changes to the clone first.
    // Only update the original state after successful file save to prevent
    // inconsistency between in-memory state and persisted config on save failure.
    let mut new_config = state.config.clone();

    new_config.settings.max_concurrent_jobs = max_concurrent;
    new_config.settings.auto_run = *state.settings_auto_run;
    new_config.settings.use_worktree = *state.settings_use_worktree;
    new_config.settings.gui.output_schema = state.settings_output_schema.clone();
    new_config.settings.gui.structured_output_schema =
        state.settings_structured_output_schema.clone();

    new_config.settings.gui.voice.mode = state.voice_settings_mode.clone();
    new_config.settings.gui.voice.keywords = voice_keywords;
    new_config.settings.gui.voice.whisper_model = state.voice_settings_model.clone();
    new_config.settings.gui.voice.language = state.voice_settings_language.clone();
    new_config.settings.gui.voice.silence_threshold = silence_threshold;
    new_config.settings.gui.voice.silence_duration = silence_duration;
    new_config.settings.gui.voice.max_duration = max_duration;
    new_config.settings.gui.voice.global_hotkey = state.voice_settings_global_hotkey.clone();
    new_config.settings.gui.voice.popup_hotkey = state.voice_settings_popup_hotkey.clone();

    // Orchestrator settings
    new_config.settings.gui.orchestrator.cli_agent = state.orchestrator_cli_agent.clone();
    new_config.settings.gui.orchestrator.cli_command = state.orchestrator_cli_command.clone();
    new_config.settings.gui.orchestrator.system_prompt = state.orchestrator_system_prompt.clone();

    // Try to save to file FIRST - before modifying any state
    let config_path = Config::global_config_path();
    if let Err(e) = new_config.save_to_file(&config_path) {
        *state.settings_status = Some((format!("Failed to save config: {}", e), true));
        return;
    }

    // File save succeeded - now safely update in-memory state
    *state.config = new_config;

    // Update the shared atomic value so executor picks up the change immediately
    // Using SeqCst for proper visibility across threads after config file write
    state
        .max_concurrent_jobs_shared
        .store(max_concurrent, Ordering::SeqCst);

    *state.settings_status = Some(("Settings saved!".to_string(), false));
    *state.voice_config_changed = true;
}
