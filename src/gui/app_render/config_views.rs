//! Configuration view rendering methods for KycoApp
//!
//! Contains methods for rendering settings, modes, agents, and chains views.

use crate::gui::app::KycoApp;
use crate::LogEvent;
use eframe::egui;

impl KycoApp {
    /// Render settings/extensions view
    pub(crate) fn render_settings(&mut self, ctx: &egui::Context) {
        use crate::gui::settings;

        let Ok(mut config) = self.config.write() else {
            // Lock poisoned - show error and return
            self.logs.push(LogEvent::error(
                "Config lock poisoned, cannot render settings",
            ));
            return;
        };
        settings::render_settings(
            ctx,
            &mut settings::SettingsState {
                // General settings
                settings_max_concurrent: &mut self.settings_max_concurrent,
                settings_auto_run: &mut self.settings_auto_run,
                settings_use_worktree: &mut self.settings_use_worktree,
                settings_output_schema: &mut self.settings_output_schema,
                settings_structured_output_schema: &mut self.settings_structured_output_schema,
                settings_status: &mut self.settings_status,
                // Voice settings
                voice_settings_mode: &mut self.voice_settings_mode,
                voice_settings_keywords: &mut self.voice_settings_keywords,
                voice_settings_model: &mut self.voice_settings_model,
                voice_settings_language: &mut self.voice_settings_language,
                voice_settings_silence_threshold: &mut self.voice_settings_silence_threshold,
                voice_settings_silence_duration: &mut self.voice_settings_silence_duration,
                voice_settings_max_duration: &mut self.voice_settings_max_duration,
                voice_settings_global_hotkey: &mut self.voice_settings_global_hotkey,
                voice_settings_popup_hotkey: &mut self.voice_settings_popup_hotkey,
                voice_install_status: &mut self.voice_install_status,
                voice_install_in_progress: &mut self.voice_install_in_progress,
                voice_install_handle: &mut self.voice_install_handle,
                // Voice test state
                voice_test_status: &mut self.voice_test_status,
                voice_test_result: &mut self.voice_test_result,
                // VAD settings
                vad_enabled: &mut self.vad_enabled,
                vad_speech_threshold: &mut self.vad_speech_threshold,
                vad_silence_duration_ms: &mut self.vad_silence_duration_ms,
                // Voice action registry (from voice manager config)
                voice_action_registry: &self.voice_manager.config.action_registry,
                // Extension status
                extension_status: &mut self.extension_status,
                // Navigation and config
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
                // Voice config change tracking
                voice_config_changed: &mut self.voice_config_changed,
                // Shared max concurrent jobs (for runtime updates to executor)
                max_concurrent_jobs_shared: &self.max_concurrent_jobs,
                orchestrator_cli_agent: &mut self.orchestrator_cli_agent,
                orchestrator_cli_command: &mut self.orchestrator_cli_command,
                orchestrator_system_prompt: &mut self.orchestrator_system_prompt,
            },
        );
    }

    /// Render modes configuration view
    pub(crate) fn render_modes(&mut self, ctx: &egui::Context) {
        use crate::gui::modes;

        let Ok(mut config) = self.config.write() else {
            self.logs
                .push(LogEvent::error("Config lock poisoned, cannot render modes"));
            return;
        };
        modes::render_modes(
            ctx,
            &mut modes::ModeEditorState {
                selected_mode: &mut self.selected_mode,
                mode_edit_name: &mut self.mode_edit_name,
                mode_edit_aliases: &mut self.mode_edit_aliases,
                mode_edit_prompt: &mut self.mode_edit_prompt,
                mode_edit_system_prompt: &mut self.mode_edit_system_prompt,
                mode_edit_readonly: &mut self.mode_edit_readonly,
                mode_edit_status: &mut self.mode_edit_status,
                mode_edit_agent: &mut self.mode_edit_agent,
                mode_edit_allowed_tools: &mut self.mode_edit_allowed_tools,
                mode_edit_disallowed_tools: &mut self.mode_edit_disallowed_tools,
                mode_edit_session_mode: &mut self.mode_edit_session_mode,
                mode_edit_max_turns: &mut self.mode_edit_max_turns,
                mode_edit_model: &mut self.mode_edit_model,
                mode_edit_claude_permission: &mut self.mode_edit_claude_permission,
                mode_edit_codex_sandbox: &mut self.mode_edit_codex_sandbox,
                mode_edit_output_states: &mut self.mode_edit_output_states,
                mode_edit_state_prompt: &mut self.mode_edit_state_prompt,
                mode_edit_use_worktree: &mut self.mode_edit_use_worktree,
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Render agents configuration view
    pub(crate) fn render_agents(&mut self, ctx: &egui::Context) {
        use crate::gui::agents;

        let Ok(mut config) = self.config.write() else {
            self.logs.push(LogEvent::error(
                "Config lock poisoned, cannot render agents",
            ));
            return;
        };
        agents::render_agents(
            ctx,
            &mut agents::AgentEditorState {
                selected_agent: &mut self.selected_agent,
                agent_edit_name: &mut self.agent_edit_name,
                agent_edit_aliases: &mut self.agent_edit_aliases,
                agent_edit_cli_type: &mut self.agent_edit_cli_type,
                agent_edit_model: &mut self.agent_edit_model,
                agent_edit_mode: &mut self.agent_edit_mode,
                agent_edit_system_prompt_mode: &mut self.agent_edit_system_prompt_mode,
                agent_edit_disallowed_tools: &mut self.agent_edit_disallowed_tools,
                agent_edit_allowed_tools: &mut self.agent_edit_allowed_tools,
                agent_edit_status: &mut self.agent_edit_status,
                agent_edit_price_input: &mut self.agent_edit_price_input,
                agent_edit_price_cached_input: &mut self.agent_edit_price_cached_input,
                agent_edit_price_output: &mut self.agent_edit_price_output,
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Render chains configuration view
    pub(crate) fn render_chains(&mut self, ctx: &egui::Context) {
        use crate::gui::chains;

        let Ok(mut config) = self.config.write() else {
            self.logs.push(LogEvent::error(
                "Config lock poisoned, cannot render chains",
            ));
            return;
        };
        chains::render_chains(
            ctx,
            &mut chains::ChainEditorState {
                selected_chain: &mut self.selected_chain,
                chain_edit_name: &mut self.chain_edit_name,
                chain_edit_description: &mut self.chain_edit_description,
                chain_edit_states: &mut self.chain_edit_states,
                chain_edit_steps: &mut self.chain_edit_steps,
                chain_edit_stop_on_failure: &mut self.chain_edit_stop_on_failure,
                chain_edit_pass_full_response: &mut self.chain_edit_pass_full_response,
                chain_edit_use_worktree: &mut self.chain_edit_use_worktree,
                chain_edit_status: &mut self.chain_edit_status,
                pending_confirmation: &mut self.chain_pending_confirmation,
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
            },
        );
    }
}
