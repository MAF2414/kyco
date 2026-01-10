//! Constructor for KycoApp
//!
//! Extracted to reduce app.rs size. Contains the complex initialization logic.

use super::app::KycoApp;
use super::detail_panel::ActivityLogFilters;
use super::diff::DiffState;
use super::executor::ExecutorEvent;
use super::groups::ComparisonState;
use super::http_server::{BatchRequest, SelectionRequest};
use super::jobs;
use super::permission::PermissionPopupState;
use super::selection::{AutocompleteState, SelectionContext};
use super::update::UpdateChecker;
use super::voice::{VoiceConfig, VoiceInputMode, VoiceManager};
use crate::LogEvent;
use crate::agent::bridge::BridgeClient;
use crate::config::{default_orchestrator_system_prompt, Config};
use crate::job::{GroupManager, JobManager};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, RwLock};

impl KycoApp {
    /// Create a new GUI application
    pub fn new(
        work_dir: PathBuf,
        config: Arc<RwLock<Config>>,
        config_exists: bool,
        job_manager: Arc<Mutex<JobManager>>,
        group_manager: Arc<Mutex<GroupManager>>,
        http_rx: Receiver<SelectionRequest>,
        batch_rx: Receiver<BatchRequest>,
        executor_rx: Receiver<ExecutorEvent>,
        max_concurrent_jobs: Arc<AtomicUsize>,
    ) -> Self {
        // Extract all values within read lock scope to avoid cloning entire Config.
        // This reduces memory allocation from O(config_size) to O(extracted_fields).
        let (
            settings_max_concurrent,
            settings_auto_run,
            settings_use_worktree,
            voice_config,
            voice_settings_mode,
            voice_settings_keywords,
            voice_settings_model,
            voice_settings_language,
            voice_settings_silence_threshold,
            voice_settings_silence_duration,
            voice_settings_max_duration,
            voice_settings_global_hotkey,
            voice_settings_popup_hotkey,
            settings_output_schema,
            settings_structured_output_schema,
            orchestrator_cli_agent,
            orchestrator_cli_command,
            orchestrator_system_prompt,
        ) = config
            .read()
            .map(|cfg| {
                let voice_settings = &cfg.settings.gui.voice;

                // Build voice action registry from modes and chains
                let action_registry = super::voice::VoiceActionRegistry::from_config(
                    &cfg.mode,
                    &cfg.chain,
                    &cfg.agent,
                );

                // Clone voice settings for UI editor fields first, then reuse for VoiceConfig
                let voice_settings_mode = voice_settings.mode.clone();
                let voice_settings_model = voice_settings.whisper_model.clone();
                let voice_settings_language = voice_settings.language.clone();
                let voice_settings_global_hotkey = voice_settings.global_hotkey.clone();
                let voice_settings_popup_hotkey = voice_settings.popup_hotkey.clone();

                let voice_config = VoiceConfig {
                    mode: match voice_settings_mode.as_str() {
                        "manual" => VoiceInputMode::Manual,
                        "hotkey_hold" => VoiceInputMode::HotkeyHold,
                        "continuous" => VoiceInputMode::Continuous,
                        _ => VoiceInputMode::Disabled,
                    },
                    keywords: voice_settings.keywords.clone(),
                    action_registry,
                    // Clone from already-cloned strings to avoid double-borrowing issues
                    whisper_model: voice_settings_model.clone(),
                    language: voice_settings_language.clone(),
                    silence_threshold: voice_settings.silence_threshold,
                    silence_duration: voice_settings.silence_duration,
                    max_duration: voice_settings.max_duration,
                    vad_config: super::voice::VadConfig::default(),
                    use_vad: true,
                };

                (
                    cfg.settings.max_concurrent_jobs.to_string(),
                    cfg.settings.auto_run,
                    cfg.settings.use_worktree,
                    voice_config,
                    voice_settings_mode,
                    voice_settings.keywords.join(", "),
                    voice_settings_model,
                    voice_settings_language,
                    voice_settings.silence_threshold.to_string(),
                    voice_settings.silence_duration.to_string(),
                    voice_settings.max_duration.to_string(),
                    voice_settings_global_hotkey,
                    voice_settings_popup_hotkey,
                    cfg.settings.gui.output_schema.clone(),
                    cfg.settings.gui.structured_output_schema.clone(),
                    cfg.settings.gui.orchestrator.cli_agent.clone(),
                    cfg.settings.gui.orchestrator.cli_command.clone(),
                    // Use default if config value is empty (common after config migration)
                    if cfg.settings.gui.orchestrator.system_prompt.trim().is_empty() {
                        default_orchestrator_system_prompt()
                    } else {
                        cfg.settings.gui.orchestrator.system_prompt.clone()
                    },
                )
            })
            .unwrap_or_else(|_| {
                // Fallback to defaults if lock is poisoned
                let defaults = Config::with_defaults();
                let voice_settings = &defaults.settings.gui.voice;
                let action_registry = super::voice::VoiceActionRegistry::from_config(
                    &defaults.mode,
                    &defaults.chain,
                    &defaults.agent,
                );
                let voice_settings_mode = voice_settings.mode.clone();
                let voice_settings_model = voice_settings.whisper_model.clone();
                let voice_settings_language = voice_settings.language.clone();
                let voice_config = VoiceConfig {
                    mode: VoiceInputMode::Disabled,
                    keywords: voice_settings.keywords.clone(),
                    action_registry,
                    whisper_model: voice_settings_model.clone(),
                    language: voice_settings_language.clone(),
                    silence_threshold: voice_settings.silence_threshold,
                    silence_duration: voice_settings.silence_duration,
                    max_duration: voice_settings.max_duration,
                    vad_config: super::voice::VadConfig::default(),
                    use_vad: true,
                };
                (
                    defaults.settings.max_concurrent_jobs.to_string(),
                    defaults.settings.auto_run,
                    defaults.settings.use_worktree,
                    voice_config,
                    voice_settings_mode,
                    voice_settings.keywords.join(", "),
                    voice_settings_model,
                    voice_settings_language,
                    voice_settings.silence_threshold.to_string(),
                    voice_settings.silence_duration.to_string(),
                    voice_settings.max_duration.to_string(),
                    voice_settings.global_hotkey.clone(),
                    voice_settings.popup_hotkey.clone(),
                    defaults.settings.gui.output_schema.clone(),
                    defaults.settings.gui.structured_output_schema.clone(),
                    defaults.settings.gui.orchestrator.cli_agent.clone(),
                    defaults.settings.gui.orchestrator.cli_command.clone(),
                    defaults.settings.gui.orchestrator.system_prompt.clone(),
                )
            });

        // Initialize global hotkey manager with configured hotkey (before struct init)
        let global_hotkey_manager = Self::init_global_hotkey_manager(&voice_settings_global_hotkey);

        // Clone work_dir once for struct field; move original to voice_manager
        let work_dir_owned = work_dir.clone();

        Self {
            work_dir: work_dir_owned,
            config,
            config_exists,
            job_manager,
            group_manager,
            cached_jobs: Vec::new(),
            last_job_generation: 0,
            selected_job_id: None,
            job_list_filter: jobs::JobListFilter::default(),
            logs: vec![LogEvent::system("kyco GUI started")],
            http_rx,
            batch_rx,
            executor_rx,
            max_concurrent_jobs,
            selection: SelectionContext::default(),
            batch_files: Vec::new(),
            file_search: crate::gui::files::FileSearchState::default(),
            view_mode: super::app_types::ViewMode::JobList,
            popup_input: String::new(),
            autocomplete: AutocompleteState::default(),
            popup_status: None,
            diff_state: DiffState::new(),
            diff_return_view: super::app_types::ViewMode::JobList,
            inline_diff_content: None,
            prev_selected_job_id: None,
            apply_confirm_target: None,
            apply_confirm_return_view: super::app_types::ViewMode::JobList,
            apply_confirm_error: None,
            apply_confirm_rx: None,
            commonmark_cache: egui_commonmark::CommonMarkCache::default(),
            comparison_state: ComparisonState::default(),
            permission_state: PermissionPopupState::default(),
            last_permission_poll: std::time::Instant::now(),
            bridge_client: BridgeClient::new(),
            permission_mode_overrides: HashMap::new(),
            auto_run: settings_auto_run,
            log_scroll_to_bottom: true,
            activity_log_filters: ActivityLogFilters::default(),
            continuation_prompt: String::new(),
            extension_status: None,
            selected_mode: None,
            mode_edit_name: String::new(),
            mode_edit_status: None,
            skill_edit_content: String::new(),
            skill_folder_info: Default::default(),
            skills_tab: Default::default(),
            registry_search_query: String::new(),
            registry_search_results: Vec::new(),
            skill_registry: None,
            registry_install_status: None,
            registry_install_location: Default::default(),
            selected_agent: None,
            agent_edit_name: String::new(),
            agent_edit_aliases: String::new(),
            agent_edit_cli_type: String::new(),
            agent_edit_model: String::new(),
            agent_edit_permission_mode: String::new(),
            agent_edit_sandbox: String::new(),
            agent_edit_ask_for_approval: String::new(),
            agent_edit_mode: String::new(),
            agent_edit_system_prompt_mode: String::new(),
            agent_edit_disallowed_tools: String::new(),
            agent_edit_allowed_tools: String::new(),
            agent_edit_status: None,
            agent_edit_price_input: String::new(),
            agent_edit_price_cached_input: String::new(),
            agent_edit_price_output: String::new(),
            agent_edit_allow_dangerous_bypass: false,
            settings_max_concurrent,
            settings_auto_run,
            settings_use_worktree,
            settings_output_schema,
            settings_structured_output_schema,
            settings_status: None,
            voice_manager: {
                let mut vm = VoiceManager::new(voice_config);
                vm.set_work_dir(work_dir); // Move original, no clone needed
                vm
            },
            voice_settings_mode,
            voice_settings_keywords,
            voice_settings_model,
            voice_settings_language,
            voice_settings_silence_threshold,
            voice_settings_silence_duration,
            voice_settings_max_duration,
            voice_settings_global_hotkey,
            voice_settings_popup_hotkey,
            vad_enabled: true,
            vad_speech_threshold: "0.5".to_string(),
            vad_silence_duration_ms: "1000".to_string(),
            voice_install_status: None,
            voice_install_in_progress: false,
            voice_install_handle: None,
            voice_test_status: super::settings::VoiceTestStatus::Idle,
            voice_test_result: None,
            selected_chain: None,
            chain_edit_name: String::new(),
            chain_edit_description: String::new(),
            chain_edit_states: Vec::new(),
            chain_edit_steps: Vec::new(),
            chain_edit_stop_on_failure: true,
            chain_edit_pass_full_response: true,
            chain_edit_max_loops: 1,
            chain_edit_use_worktree: None,
            chain_edit_status: None,
            chain_pending_confirmation: super::chains::PendingConfirmation::None,
            voice_config_changed: false,
            voice_pending_execute: false,
            update_checker: UpdateChecker::new(),
            update_install_status: super::status_bar::InstallStatus::default(),
            update_install_rx: None,
            orchestrator_cli_agent,
            orchestrator_cli_command,
            orchestrator_system_prompt,
            orchestrator_requested: false,
            last_log_cleanup: std::time::Instant::now(),

            // Use pre-computed global hotkey manager
            global_hotkey_manager,
            voice_hotkey: None, // Will be set after manager is created
            global_voice_recording: false,
            global_voice_auto_paste: true,
            show_voice_overlay: false,

            // Statistics tracking
            stats_manager: crate::stats::StatsManager::new().ok(),
            stats_time_range: crate::stats::TimeRange::default(),
            stats_last_refresh: std::time::Instant::now(),

            // Dashboard V2
            stats_filter_agent: None,
            stats_filter_mode: None,
            stats_filter_workspace: None,
            dashboard_summary: crate::stats::DashboardSummary::default(),
            stats_reset_confirm: false,

            // Gamification
            gamification_events: std::collections::VecDeque::new(),
            current_toast: None,
            player_stats: None,
            streaks: None,
        }
    }
}
