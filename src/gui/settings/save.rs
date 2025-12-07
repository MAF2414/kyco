//! Settings save functionality
//!
//! Handles validation and persistence of settings to config file.

use super::state::SettingsState;

/// Save settings to config file
///
/// Uses proper TOML serialization to avoid config file corruption.
pub fn save_settings_to_config(state: &mut SettingsState<'_>) {
    // Parse and validate values
    let max_concurrent = match state.settings_max_concurrent.trim().parse::<usize>() {
        Ok(n) if n > 0 => n,
        _ => {
            *state.settings_status =
                Some(("Invalid max concurrent jobs (must be > 0)".to_string(), true));
            return;
        }
    };

    let debounce_ms = match state.settings_debounce_ms.trim().parse::<u64>() {
        Ok(n) => n,
        _ => {
            *state.settings_status = Some(("Invalid debounce ms".to_string(), true));
            return;
        }
    };

    let scan_exclude: Vec<String> = state
        .settings_scan_exclude
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Parse voice settings
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

    // Update the in-memory config with new values
    state.config.settings.max_concurrent_jobs = max_concurrent;
    state.config.settings.debounce_ms = debounce_ms;
    state.config.settings.auto_run = *state.settings_auto_run;
    state.config.settings.marker_prefix = state.settings_marker_prefix.clone();
    state.config.settings.scan_exclude = scan_exclude;
    state.config.settings.use_worktree = *state.settings_use_worktree;
    state.config.settings.gui.output_schema = state.settings_output_schema.clone();

    // Update voice settings
    state.config.settings.gui.voice.mode = state.voice_settings_mode.clone();
    state.config.settings.gui.voice.keywords = voice_keywords;
    state.config.settings.gui.voice.whisper_model = state.voice_settings_model.clone();
    state.config.settings.gui.voice.language = state.voice_settings_language.clone();
    state.config.settings.gui.voice.silence_threshold = silence_threshold;
    state.config.settings.gui.voice.silence_duration = silence_duration;
    state.config.settings.gui.voice.max_duration = max_duration;

    // Serialize entire config using proper TOML serialization
    let config_path = state.work_dir.join(".kyco").join("config.toml");
    match toml::to_string_pretty(state.config) {
        Ok(toml_content) => {
            if let Err(e) = std::fs::write(&config_path, &toml_content) {
                *state.settings_status = Some((format!("Failed to write config: {}", e), true));
                return;
            }
            *state.settings_status = Some(("Settings saved!".to_string(), false));
            // Signal that voice config needs to be applied to the VoiceManager
            *state.voice_config_changed = true;
        }
        Err(e) => {
            *state.settings_status = Some((format!("Failed to serialize config: {}", e), true));
        }
    }
}
