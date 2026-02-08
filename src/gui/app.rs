//! Main GUI application using egui
//!
//! Full-featured GUI replacing the TUI with:
//! - Job list panel (left)
//! - Detail panel with logs (right)
//! - Selection popup for IDE extension input
//! - Controls for job management

use super::app_popup::{ApplyTarget, ApplyThreadOutcome};
use super::detail_panel::ActivityLogFilters;
use super::diff::DiffState;
use super::executor::ExecutorEvent;
use super::groups::ComparisonState;
use super::http_server::{BatchFile, BatchRequest, SelectionRequest};
use super::jobs;
use super::permission::PermissionPopupState;
use super::selection::{AutocompleteState, SelectionContext};
use super::update::UpdateChecker;
use super::voice::VoiceManager;
use crate::agent::bridge::{BridgeClient, BridgeProcess, PermissionMode};
use crate::config::Config;
use crate::job::{GroupManager, JobManager};
use crate::{Job, JobId, LogEvent};
use global_hotkey::{GlobalHotKeyManager, hotkey::HotKey};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, RwLock};

// Types
pub use super::app_types::{Mode, ViewMode};

/// Main application state
pub struct KycoApp {
    /// Working directory
    pub(crate) work_dir: PathBuf,
    /// Configuration
    #[allow(dead_code)]
    pub(crate) config: Arc<RwLock<Config>>,
    /// Whether config file exists (show init button if not)
    pub(crate) config_exists: bool,
    /// Job manager (shared with async tasks)
    pub(crate) job_manager: Arc<Mutex<JobManager>>,
    /// Group manager for multi-agent parallel execution
    pub(crate) group_manager: Arc<Mutex<GroupManager>>,
    /// Cached jobs for display (updated only when changed)
    pub(crate) cached_jobs: Vec<Job>,
    /// Last known job manager generation (for change detection)
    pub(crate) last_job_generation: u64,
    /// Selected job ID
    pub(crate) selected_job_id: Option<u64>,
    /// Job list filter
    pub(crate) job_list_filter: jobs::JobListFilter,
    /// Log events
    pub(crate) logs: Vec<LogEvent>,
    /// Receiver for HTTP selection events from IDE extensions
    pub(crate) http_rx: Receiver<SelectionRequest>,
    /// Receiver for batch processing requests from IDE extensions
    pub(crate) batch_rx: Receiver<BatchRequest>,
    /// Receiver for executor events
    pub(crate) executor_rx: Receiver<ExecutorEvent>,
    /// Shared max concurrent jobs (runtime-adjustable)
    pub(crate) max_concurrent_jobs: Arc<AtomicUsize>,
    /// Current selection context (from IDE extension)
    pub(crate) selection: SelectionContext,
    /// Batch files for batch processing (from IDE extension)
    pub(crate) batch_files: Vec<BatchFile>,
    /// File search state (for Files view)
    pub(crate) file_search: crate::gui::files::FileSearchState,
    /// Current view mode
    pub(crate) view_mode: ViewMode,
    /// Selection popup input
    pub(crate) popup_input: String,
    /// Autocomplete state (suggestions, selection, etc.)
    pub(crate) autocomplete: AutocompleteState,
    /// Status message for popup
    pub(crate) popup_status: Option<(String, bool)>,
    /// Diff view state
    pub(crate) diff_state: DiffState,
    /// View mode to return to after closing diff
    pub(crate) diff_return_view: ViewMode,
    /// Inline diff content for detail panel (loaded when job selected)
    pub(crate) inline_diff_content: Option<String>,
    /// Previously selected job ID (to detect selection changes)
    pub(crate) prev_selected_job_id: Option<u64>,
    /// Pending merge/apply confirmation (shown in ApplyConfirmPopup)
    pub(crate) apply_confirm_target: Option<ApplyTarget>,
    /// View mode to return to after canceling apply confirmation
    pub(crate) apply_confirm_return_view: ViewMode,
    /// Error message shown in apply confirmation popup
    pub(crate) apply_confirm_error: Option<String>,
    /// Receiver for async apply/merge results
    pub(crate) apply_confirm_rx:
        Option<std::sync::mpsc::Receiver<Result<ApplyThreadOutcome, String>>>,
    /// Markdown rendering cache (for agent responses)
    pub(crate) commonmark_cache: egui_commonmark::CommonMarkCache,
    /// Comparison popup state for multi-agent results
    pub(crate) comparison_state: ComparisonState,
    /// Permission popup state for tool approval requests
    pub(crate) permission_state: PermissionPopupState,
    /// Last time we polled the bridge for pending tool approvals
    pub(crate) last_permission_poll: std::time::Instant,
    /// Bridge server process (keeps Node.js server alive)
    #[allow(dead_code)]
    pub(crate) bridge_process: Option<BridgeProcess>,
    /// Bridge client for sending tool approval responses
    pub(crate) bridge_client: BridgeClient,
    /// Current Claude permission mode overrides per job (UI state)
    pub(crate) permission_mode_overrides: HashMap<JobId, PermissionMode>,
    /// Auto-run enabled
    pub(crate) auto_run: bool,
    /// Auto-allow tool calls (skip permission popup)
    pub(crate) auto_allow: bool,
    /// Log scroll to bottom
    pub(crate) log_scroll_to_bottom: bool,
    /// Activity log kind filters (UI state)
    pub(crate) activity_log_filters: ActivityLogFilters,
    /// Session continuation prompt (for follow-up messages in session mode)
    pub(crate) continuation_prompt: String,
    /// Extension install status message
    pub(crate) extension_status: Option<(String, bool)>,
    /// Selected skill for editing (None = list view)
    pub(crate) selected_mode: Option<String>,
    /// Skill editor: name field for new skills
    pub(crate) mode_edit_name: String,
    /// Skill editor: status message
    pub(crate) mode_edit_status: Option<(String, bool)>,
    /// Selected agent for editing (None = list view)
    pub(crate) selected_agent: Option<String>,
    /// Agent editor: name field
    pub(crate) agent_edit_name: String,
    /// Agent editor: aliases field
    pub(crate) agent_edit_aliases: String,
    /// Agent editor: sdk type (claude/codex)
    pub(crate) agent_edit_cli_type: String,
    /// Agent editor: model override
    pub(crate) agent_edit_model: String,
    /// Agent editor: permission/approval mode (SDK-specific)
    pub(crate) agent_edit_permission_mode: String,
    /// Agent editor: Codex sandbox mode
    pub(crate) agent_edit_sandbox: String,
    /// Agent editor: Codex approvals policy (--ask-for-approval)
    pub(crate) agent_edit_ask_for_approval: String,
    /// Agent editor: session mode (oneshot/session)
    pub(crate) agent_edit_mode: String,
    /// Agent editor: system_prompt_mode
    pub(crate) agent_edit_system_prompt_mode: String,
    /// Agent editor: disallowed_tools
    pub(crate) agent_edit_disallowed_tools: String,
    /// Agent editor: allowed_tools
    pub(crate) agent_edit_allowed_tools: String,
    /// Agent editor: status message
    pub(crate) agent_edit_status: Option<(String, bool)>,
    /// Agent editor: input token price per 1M tokens
    pub(crate) agent_edit_price_input: String,
    /// Agent editor: cached input token price per 1M tokens
    pub(crate) agent_edit_price_cached_input: String,
    /// Agent editor: output token price per 1M tokens
    pub(crate) agent_edit_price_output: String,
    /// Agent editor: allow dangerous bypass (--dangerously-skip-permissions / --yolo)
    pub(crate) agent_edit_allow_dangerous_bypass: bool,
    /// Skill editor: raw SKILL.md content being edited
    pub(crate) skill_edit_content: String,
    /// Skill editor: folder structure info (scripts/, references/, assets/)
    pub(crate) skill_folder_info: crate::gui::skills::SkillFolderInfo,
    /// Skills view: current tab (Local/Registry)
    pub(crate) skills_tab: crate::gui::skills::SkillsTab,
    /// Skills view: registry search query
    pub(crate) registry_search_query: String,
    /// Skills view: registry search results
    pub(crate) registry_search_results: Vec<crate::config::RegistrySkill>,
    /// Skills view: loaded registry (lazily loaded)
    pub(crate) skill_registry: Option<crate::config::SkillRegistry>,
    /// Skills view: registry install status
    pub(crate) registry_install_status: Option<(String, bool)>,
    /// Skills view: registry install location (Global/Workspace)
    pub(crate) registry_install_location: crate::gui::skills::SkillInstallLocation,
    /// Settings editor: max concurrent jobs
    pub(crate) settings_max_concurrent: String,
    // NOTE: auto_run and auto_allow are used directly by SettingsState
    // (no separate settings_auto_run / settings_auto_allow fields needed).
    /// Settings editor: use worktree
    pub(crate) settings_use_worktree: bool,
    /// Settings editor: output schema template
    pub(crate) settings_output_schema: String,
    /// Settings editor: structured output JSON schema (optional)
    pub(crate) settings_structured_output_schema: String,
    /// Settings editor: status message
    pub(crate) settings_status: Option<(String, bool)>,
    /// Voice input manager
    pub(crate) voice_manager: VoiceManager,
    /// Voice settings editor: mode
    pub(crate) voice_settings_mode: String,
    /// Voice settings editor: keywords
    pub(crate) voice_settings_keywords: String,
    /// Voice settings editor: whisper model
    pub(crate) voice_settings_model: String,
    /// Voice settings editor: language
    pub(crate) voice_settings_language: String,
    /// Voice settings editor: silence threshold
    pub(crate) voice_settings_silence_threshold: String,
    /// Voice settings editor: silence duration
    pub(crate) voice_settings_silence_duration: String,
    /// Voice settings editor: max duration
    pub(crate) voice_settings_max_duration: String,
    /// Voice settings editor: global hotkey (dictate from any app)
    pub(crate) voice_settings_global_hotkey: String,
    /// Voice settings editor: popup hotkey (start/stop in selection popup)
    pub(crate) voice_settings_popup_hotkey: String,
    /// VAD settings: enabled
    pub(crate) vad_enabled: bool,
    /// VAD settings: speech threshold
    pub(crate) vad_speech_threshold: String,
    /// VAD settings: silence duration (ms)
    pub(crate) vad_silence_duration_ms: String,
    /// Voice installation status message
    pub(crate) voice_install_status: Option<(String, bool)>,
    /// Voice installation in progress
    pub(crate) voice_install_in_progress: bool,
    /// Async voice installation handle (for non-blocking installation)
    pub(crate) voice_install_handle: Option<super::voice::install::InstallHandle>,
    /// Voice test status
    pub(crate) voice_test_status: super::settings::VoiceTestStatus,
    /// Voice test result (transcribed text)
    pub(crate) voice_test_result: Option<String>,
    /// Flag to indicate voice config was changed and VoiceManager needs to be updated
    pub(crate) voice_config_changed: bool,
    /// Flag to execute popup task after voice transcription completes
    pub(crate) voice_pending_execute: bool,
    /// Update checker for new version notifications
    pub(crate) update_checker: UpdateChecker,
    /// Update install status
    pub(crate) update_install_status: super::status_bar::InstallStatus,
    /// Receiver for install results
    pub(crate) update_install_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    /// Selected chain for editing (None = list view)
    pub(crate) selected_chain: Option<String>,
    /// Chain editor: name field
    pub(crate) chain_edit_name: String,
    /// Chain editor: description field
    pub(crate) chain_edit_description: String,
    /// Chain editor: state definitions
    pub(crate) chain_edit_states: Vec<super::chains::state::StateDefinitionEdit>,
    /// Chain editor: steps
    pub(crate) chain_edit_steps: Vec<super::chains::state::ChainStepEdit>,
    /// Chain editor: stop on failure
    pub(crate) chain_edit_stop_on_failure: bool,
    /// Chain editor: pass full response to next step
    pub(crate) chain_edit_pass_full_response: bool,
    /// Chain editor: maximum loop iterations
    pub(crate) chain_edit_max_loops: u32,
    /// Chain editor: use worktree (None = global, Some(true) = always, Some(false) = never)
    pub(crate) chain_edit_use_worktree: Option<bool>,
    /// Chain editor: status message
    pub(crate) chain_edit_status: Option<(String, bool)>,
    /// Chain editor: pending confirmation dialog
    pub(crate) chain_pending_confirmation: super::chains::PendingConfirmation,
    /// Orchestrator settings: CLI agent (claude/codex)
    pub(crate) orchestrator_cli_agent: String,
    /// Orchestrator settings: CLI command
    pub(crate) orchestrator_cli_command: String,
    /// Orchestrator settings: system prompt
    pub(crate) orchestrator_system_prompt: String,

    /// UI action: launch an external orchestrator session (Terminal)
    pub(crate) orchestrator_requested: bool,

    /// Last time we ran log truncation (to avoid running every frame)
    pub(crate) last_log_cleanup: std::time::Instant,

    /// Global hotkey manager for voice input
    pub(crate) global_hotkey_manager: Option<GlobalHotKeyManager>,
    /// Registered voice hotkey ID (for future multi-hotkey support)
    #[allow(dead_code)]
    pub(crate) voice_hotkey: Option<HotKey>,
    /// Whether global voice recording is active (triggered by hotkey, not popup)
    pub(crate) global_voice_recording: bool,
    /// Auto-paste after transcription (for global voice input)
    pub(crate) global_voice_auto_paste: bool,
    /// Show voice overlay window (small indicator when recording)
    pub(crate) show_voice_overlay: bool,

    /// Statistics manager (None if initialization failed)
    pub(crate) stats_manager: Option<crate::stats::StatsManager>,
    /// Selected time range for stats view
    pub(crate) stats_time_range: crate::stats::TimeRange,
    /// Last time stats were refreshed
    pub(crate) stats_last_refresh: std::time::Instant,

    // Dashboard V2 state
    /// Dashboard filter: agent (None = all)
    pub(crate) stats_filter_agent: Option<String>,
    /// Dashboard filter: mode/chain (None = all)
    pub(crate) stats_filter_mode: Option<String>,
    /// Dashboard filter: workspace (None = all)
    pub(crate) stats_filter_workspace: Option<String>,
    /// Dashboard V2 cached summary
    pub(crate) dashboard_summary: crate::stats::DashboardSummary,
    /// Show stats reset confirmation dialog
    pub(crate) stats_reset_confirm: bool,

    // Gamification state
    /// Queue of gamification events to display (achievements, level-ups, etc.)
    pub(crate) gamification_events: std::collections::VecDeque<crate::stats::GamificationEvent>,
    /// Current toast being displayed (event, start time)
    pub(crate) current_toast: Option<(crate::stats::GamificationEvent, std::time::Instant)>,
    /// Cached player stats for display
    pub(crate) player_stats: Option<crate::stats::PlayerStats>,
    /// Cached streaks for display
    pub(crate) streaks: Option<crate::stats::Streaks>,

}
