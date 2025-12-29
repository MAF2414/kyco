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
use crate::config::Config;
use crate::job::{GroupManager, JobManager};
use crate::workspace::WorkspaceRegistry;
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
        let config_snapshot = config
            .read()
            .map(|cfg| cfg.clone())
            .unwrap_or_else(|_| Config::with_defaults());

        // Extract settings before moving config
        let settings_max_concurrent = config_snapshot.settings.max_concurrent_jobs.to_string();
        let settings_auto_run = config_snapshot.settings.auto_run;
        let settings_use_worktree = config_snapshot.settings.use_worktree;

        // Extract voice settings
        let voice_settings = &config_snapshot.settings.gui.voice;
        // Build voice action registry from modes and chains
        let action_registry = super::voice::VoiceActionRegistry::from_config(
            &config_snapshot.mode,
            &config_snapshot.chain,
            &config_snapshot.agent,
        );
        let voice_config = VoiceConfig {
            mode: match voice_settings.mode.as_str() {
                "manual" => VoiceInputMode::Manual,
                "hotkey_hold" => VoiceInputMode::HotkeyHold,
                "continuous" => VoiceInputMode::Continuous,
                _ => VoiceInputMode::Disabled,
            },
            keywords: voice_settings.keywords.clone(),
            action_registry,
            whisper_model: voice_settings.whisper_model.clone(),
            language: voice_settings.language.clone(),
            silence_threshold: voice_settings.silence_threshold,
            silence_duration: voice_settings.silence_duration,
            max_duration: voice_settings.max_duration,
            vad_config: super::voice::VadConfig::default(),
            use_vad: true,
        };
        let voice_settings_mode = voice_settings.mode.clone();
        let voice_settings_keywords = voice_settings.keywords.join(", ");
        let voice_settings_model = voice_settings.whisper_model.clone();
        let voice_settings_language = voice_settings.language.clone();
        let voice_settings_silence_threshold = voice_settings.silence_threshold.to_string();
        let voice_settings_silence_duration = voice_settings.silence_duration.to_string();
        let voice_settings_max_duration = voice_settings.max_duration.to_string();
        let voice_settings_global_hotkey = voice_settings.global_hotkey.clone();
        let voice_settings_popup_hotkey = voice_settings.popup_hotkey.clone();

        // Extract output schema
        let settings_output_schema = config_snapshot.settings.gui.output_schema.clone();
        let settings_structured_output_schema = config_snapshot
            .settings
            .gui
            .structured_output_schema
            .clone();

        // Extract orchestrator settings
        let orchestrator_cli_agent = config_snapshot
            .settings
            .gui
            .orchestrator
            .cli_agent
            .clone();
        let orchestrator_cli_command = config_snapshot
            .settings
            .gui
            .orchestrator
            .cli_command
            .clone();
        let orchestrator_system_prompt = config_snapshot
            .settings
            .gui
            .orchestrator
            .system_prompt
            .clone();

        // Load or create workspace registry and register the initial workspace
        let mut workspace_registry = WorkspaceRegistry::load_or_create();
        let initial_workspace_id = workspace_registry.get_or_create(work_dir.clone());
        workspace_registry.set_active(initial_workspace_id);

        // Initialize global hotkey manager with configured hotkey (before struct init)
        let global_hotkey_manager = Self::init_global_hotkey_manager(&voice_settings_global_hotkey);

        Self {
            work_dir: work_dir.clone(),
            config,
            config_exists,
            job_manager,
            group_manager,
            workspace_registry: Arc::new(Mutex::new(workspace_registry)),
            active_workspace_id: Some(initial_workspace_id),
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
            bridge_client: BridgeClient::new(),
            permission_mode_overrides: HashMap::new(),
            auto_run: settings_auto_run,
            log_scroll_to_bottom: true,
            activity_log_filters: ActivityLogFilters::default(),
            continuation_prompt: String::new(),
            extension_status: None,
            selected_mode: None,
            mode_edit_name: String::new(),
            mode_edit_aliases: String::new(),
            mode_edit_prompt: String::new(),
            mode_edit_system_prompt: String::new(),
            mode_edit_readonly: false,
            mode_edit_status: None,
            mode_edit_agent: String::new(),
            mode_edit_allowed_tools: String::new(),
            mode_edit_disallowed_tools: String::new(),
            mode_edit_session_mode: "oneshot".to_string(),
            mode_edit_max_turns: "0".to_string(),
            mode_edit_model: String::new(),
            mode_edit_claude_permission: "auto".to_string(),
            mode_edit_codex_sandbox: "auto".to_string(),
            mode_edit_output_states: String::new(),
            mode_edit_state_prompt: String::new(),
            mode_edit_use_worktree: None,
            selected_agent: None,
            agent_edit_name: String::new(),
            agent_edit_aliases: String::new(),
            agent_edit_cli_type: String::new(),
            agent_edit_model: String::new(),
            agent_edit_mode: String::new(),
            agent_edit_system_prompt_mode: String::new(),
            agent_edit_disallowed_tools: String::new(),
            agent_edit_allowed_tools: String::new(),
            agent_edit_status: None,
            agent_edit_price_input: String::new(),
            agent_edit_price_cached_input: String::new(),
            agent_edit_price_output: String::new(),
            settings_max_concurrent,
            settings_auto_run,
            settings_use_worktree,
            settings_output_schema,
            settings_structured_output_schema,
            settings_status: None,
            voice_manager: {
                let mut vm = VoiceManager::new(voice_config);
                vm.set_work_dir(work_dir.clone());
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
            chain_edit_use_worktree: None,
            chain_edit_status: None,
            chain_pending_confirmation: super::chains::PendingConfirmation::None,
            voice_config_changed: false,
            voice_pending_execute: false,
            update_checker: UpdateChecker::new(),
            update_install_status: super::status_bar::InstallStatus::default(),
            update_install_rx: None,
            import_workspace_selected: 0,
            import_modes: true,
            import_agents: true,
            import_chains: false,
            import_settings: false,
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
            stats_summary: crate::stats::StatsSummary::default(),
            stats_time_range: crate::stats::TimeRange::default(),
            stats_graph: crate::stats::StatsGraph::default(),
            stats_last_refresh: std::time::Instant::now(),

            // Dashboard V2
            stats_filter_agent: None,
            stats_filter_mode: None,
            dashboard_summary: crate::stats::DashboardSummary::default(),
            stats_reset_confirm: false,
        }
    }
}
