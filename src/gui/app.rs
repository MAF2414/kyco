//! Main GUI application using egui
//!
//! Full-featured GUI replacing the TUI with:
//! - Job list panel (left)
//! - Detail panel with logs (right)
//! - Selection popup for IDE extension input
//! - Controls for job management

use super::diff::DiffState;
use super::executor::ExecutorEvent;
use super::groups::{render_comparison_popup, ComparisonAction, ComparisonState};
use super::http_server::SelectionRequest;
use super::jobs;
use super::selection::autocomplete::parse_input_multi;
use super::selection::{AutocompleteState, SelectionContext};
use super::voice::{VoiceConfig, VoiceInputMode, VoiceManager};
use crate::config::Config;
use crate::job::{GroupManager, JobManager};
use crate::{AgentGroupId, Job, JobId, LogEvent};
use eframe::egui::{self, Color32, Key, Stroke};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use tracing::info;

// ═══════════════════════════════════════════════════════════════════════════
// THEME: "Terminal Phosphor" - Retro CRT monitor aesthetic
// ═══════════════════════════════════════════════════════════════════════════

/// Background: Deep charcoal with subtle blue tint (like a powered-off CRT)
pub(super) const BG_PRIMARY: Color32 = Color32::from_rgb(18, 20, 24);
/// Secondary background for panels
pub(super) const BG_SECONDARY: Color32 = Color32::from_rgb(24, 28, 34);
/// Accent highlight background
pub(super) const BG_HIGHLIGHT: Color32 = Color32::from_rgb(32, 40, 52);
/// Selected item background
pub(super) const BG_SELECTED: Color32 = Color32::from_rgb(40, 50, 65);

/// Primary text: Warm amber phosphor glow
pub(super) const TEXT_PRIMARY: Color32 = Color32::from_rgb(255, 176, 0);
/// Secondary text: Dimmed amber
pub(super) const TEXT_DIM: Color32 = Color32::from_rgb(180, 130, 50);
/// Muted text
pub(super) const TEXT_MUTED: Color32 = Color32::from_rgb(100, 85, 60);

/// Status colors
pub(super) const STATUS_PENDING: Color32 = Color32::from_rgb(150, 150, 150);
pub(super) const STATUS_QUEUED: Color32 = Color32::from_rgb(100, 180, 255);
pub(super) const STATUS_RUNNING: Color32 = Color32::from_rgb(255, 200, 50);
pub(super) const STATUS_DONE: Color32 = Color32::from_rgb(80, 255, 120);
pub(super) const STATUS_FAILED: Color32 = Color32::from_rgb(255, 80, 80);
pub(super) const STATUS_REJECTED: Color32 = Color32::from_rgb(180, 100, 100);
pub(super) const STATUS_MERGED: Color32 = Color32::from_rgb(150, 100, 255);

/// Accent colors
pub(super) const ACCENT_CYAN: Color32 = Color32::from_rgb(0, 255, 200);
pub(super) const ACCENT_GREEN: Color32 = Color32::from_rgb(80, 255, 120);
pub(super) const ACCENT_RED: Color32 = Color32::from_rgb(255, 80, 80);
pub(super) const ACCENT_PURPLE: Color32 = Color32::from_rgb(200, 120, 255);
pub(super) const ACCENT_YELLOW: Color32 = Color32::from_rgb(255, 200, 50);

// REMOVED: Hardcoded MODES and AGENTS constants
// Modes and agents are now dynamically loaded from config.toml
// via self.config.mode and self.config.agent in update_suggestions()

/// View mode for the app
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Main job list view
    JobList,
    /// Selection popup (triggered by IDE extension)
    SelectionPopup,
    /// Diff view popup
    DiffView,
    /// Comparison popup for multi-agent results
    ComparisonPopup,
    /// Settings/Extensions view
    Settings,
    /// Modes configuration view
    Modes,
    /// Agents configuration view
    Agents,
    /// Chains configuration view
    Chains,
}

/// Main application state
pub struct KycoApp {
    /// Working directory
    work_dir: PathBuf,
    /// Configuration
    #[allow(dead_code)]
    config: Config,
    /// Job manager (shared with async tasks)
    job_manager: Arc<Mutex<JobManager>>,
    /// Group manager for multi-agent parallel execution
    group_manager: Arc<Mutex<GroupManager>>,
    /// Cached jobs for display (updated periodically)
    cached_jobs: Vec<Job>,
    /// Selected job ID
    selected_job_id: Option<u64>,
    /// Log events
    logs: Vec<LogEvent>,
    /// Receiver for HTTP selection events from IDE extensions
    http_rx: Receiver<SelectionRequest>,
    /// Receiver for executor events
    executor_rx: Receiver<ExecutorEvent>,
    /// Current selection context (from IDE extension)
    selection: SelectionContext,
    /// Current view mode
    view_mode: ViewMode,
    /// Selection popup input
    popup_input: String,
    /// Autocomplete state (suggestions, selection, etc.)
    autocomplete: AutocompleteState,
    /// Status message for popup
    popup_status: Option<(String, bool)>,
    /// Diff view state
    diff_state: DiffState,
    /// Comparison popup state for multi-agent results
    comparison_state: ComparisonState,
    /// Auto-run enabled
    auto_run: bool,
    /// Auto-scan enabled
    auto_scan: bool,
    /// Log scroll to bottom
    log_scroll_to_bottom: bool,
    /// Extension install status message
    extension_status: Option<(String, bool)>,
    /// Selected mode for editing (None = list view)
    selected_mode: Option<String>,
    /// Mode editor: name field
    mode_edit_name: String,
    /// Mode editor: aliases field
    mode_edit_aliases: String,
    /// Mode editor: prompt field
    mode_edit_prompt: String,
    /// Mode editor: system_prompt field
    mode_edit_system_prompt: String,
    /// Mode editor: is read-only (disallowed_tools contains Write/Edit)
    mode_edit_readonly: bool,
    /// Mode editor: status message
    mode_edit_status: Option<(String, bool)>,
    /// Selected agent for editing (None = list view)
    selected_agent: Option<String>,
    /// Agent editor: name field
    agent_edit_name: String,
    /// Agent editor: aliases field
    agent_edit_aliases: String,
    /// Agent editor: binary field
    agent_edit_binary: String,
    /// Agent editor: cli_type field
    agent_edit_cli_type: String,
    /// Agent editor: mode (print/repl)
    agent_edit_mode: String,
    /// Agent editor: print_mode_args
    agent_edit_print_args: String,
    /// Agent editor: output_format_args
    agent_edit_output_args: String,
    /// Agent editor: repl_mode_args
    agent_edit_repl_args: String,
    /// Agent editor: system_prompt_mode
    agent_edit_system_prompt_mode: String,
    /// Agent editor: disallowed_tools
    agent_edit_disallowed_tools: String,
    /// Agent editor: allowed_tools
    agent_edit_allowed_tools: String,
    /// Agent editor: status message
    agent_edit_status: Option<(String, bool)>,
    /// Mode editor: default agent
    mode_edit_agent: String,
    /// Mode editor: allowed_tools
    mode_edit_allowed_tools: String,
    /// Mode editor: disallowed_tools
    mode_edit_disallowed_tools: String,
    /// Settings editor: max concurrent jobs
    settings_max_concurrent: String,
    /// Settings editor: debounce ms
    settings_debounce_ms: String,
    /// Settings editor: auto run
    settings_auto_run: bool,
    /// Settings editor: auto scan (local state, not in config)
    settings_auto_scan: bool,
    /// Settings editor: marker prefix
    settings_marker_prefix: String,
    /// Settings editor: use worktree
    settings_use_worktree: bool,
    /// Settings editor: scan exclude patterns
    settings_scan_exclude: String,
    /// Settings editor: output schema template
    settings_output_schema: String,
    /// Settings editor: status message
    settings_status: Option<(String, bool)>,
    /// Voice input manager
    voice_manager: VoiceManager,
    /// Voice settings editor: mode
    voice_settings_mode: String,
    /// Voice settings editor: keywords
    voice_settings_keywords: String,
    /// Voice settings editor: whisper model
    voice_settings_model: String,
    /// Voice settings editor: language
    voice_settings_language: String,
    /// Voice settings editor: silence threshold
    voice_settings_silence_threshold: String,
    /// Voice settings editor: silence duration
    voice_settings_silence_duration: String,
    /// Voice settings editor: max duration
    voice_settings_max_duration: String,
    /// VAD settings: enabled
    vad_enabled: bool,
    /// VAD settings: speech threshold
    vad_speech_threshold: String,
    /// VAD settings: silence duration (ms)
    vad_silence_duration_ms: String,
    /// Hotkey is currently held (for hotkey_hold mode)
    hotkey_held: bool,
    /// Voice installation status message
    voice_install_status: Option<(String, bool)>,
    /// Voice installation in progress
    voice_install_in_progress: bool,
    /// Selected chain for editing (None = list view)
    selected_chain: Option<String>,
    /// Chain editor: name field
    chain_edit_name: String,
    /// Chain editor: description field
    chain_edit_description: String,
    /// Chain editor: steps
    chain_edit_steps: Vec<super::chains::state::ChainStepEdit>,
    /// Chain editor: stop on failure
    chain_edit_stop_on_failure: bool,
    /// Chain editor: status message
    chain_edit_status: Option<(String, bool)>,
}

impl KycoApp {
    /// Create a new GUI application
    pub fn new(
        work_dir: PathBuf,
        config: Config,
        job_manager: Arc<Mutex<JobManager>>,
        http_rx: Receiver<SelectionRequest>,
        executor_rx: Receiver<ExecutorEvent>,
    ) -> Self {
        // Extract settings before moving config
        let settings_max_concurrent = config.settings.max_concurrent_jobs.to_string();
        let settings_debounce_ms = config.settings.debounce_ms.to_string();
        let settings_auto_run = config.settings.auto_run;
        let settings_marker_prefix = config.settings.marker_prefix.clone();
        let settings_use_worktree = config.settings.use_worktree;
        let settings_scan_exclude = config.settings.scan_exclude.join(", ");

        // Extract voice settings
        let voice_settings = &config.settings.gui.voice;
        // Build voice action registry from modes and chains
        let action_registry = super::voice::VoiceActionRegistry::from_config(
            &config.mode,
            &config.chain,
            &config.agent,
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

        // Extract output schema
        let settings_output_schema = config.settings.gui.output_schema.clone();

        Self {
            work_dir: work_dir.clone(),
            config,
            job_manager,
            group_manager: Arc::new(Mutex::new(GroupManager::new())),
            cached_jobs: Vec::new(),
            selected_job_id: None,
            logs: vec![LogEvent::system("kyco GUI started")],
            http_rx,
            executor_rx,
            selection: SelectionContext::default(),
            view_mode: ViewMode::JobList,
            popup_input: String::new(),
            autocomplete: AutocompleteState::default(),
            popup_status: None,
            diff_state: DiffState::new(),
            comparison_state: ComparisonState::default(),
            auto_run: settings_auto_run,
            auto_scan: true,
            log_scroll_to_bottom: true,
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
            selected_agent: None,
            agent_edit_name: String::new(),
            agent_edit_aliases: String::new(),
            agent_edit_binary: String::new(),
            agent_edit_cli_type: String::new(),
            agent_edit_mode: String::new(),
            agent_edit_print_args: String::new(),
            agent_edit_output_args: String::new(),
            agent_edit_repl_args: String::new(),
            agent_edit_system_prompt_mode: String::new(),
            agent_edit_disallowed_tools: String::new(),
            agent_edit_allowed_tools: String::new(),
            agent_edit_status: None,
            settings_max_concurrent,
            settings_debounce_ms,
            settings_auto_run,
            settings_auto_scan: true,
            settings_marker_prefix,
            settings_use_worktree,
            settings_scan_exclude,
            settings_output_schema,
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
            vad_enabled: true,
            vad_speech_threshold: "0.5".to_string(),
            vad_silence_duration_ms: "1000".to_string(),
            hotkey_held: false,
            voice_install_status: None,
            voice_install_in_progress: false,
            selected_chain: None,
            chain_edit_name: String::new(),
            chain_edit_description: String::new(),
            chain_edit_steps: Vec::new(),
            chain_edit_stop_on_failure: true,
            chain_edit_status: None,
        }
    }

    /// Render settings/extensions view
    fn render_settings(&mut self, ctx: &egui::Context) {
        super::settings::render_settings(
            ctx,
            &mut super::settings::SettingsState {
                // General settings
                settings_max_concurrent: &mut self.settings_max_concurrent,
                settings_debounce_ms: &mut self.settings_debounce_ms,
                settings_auto_run: &mut self.settings_auto_run,
                settings_marker_prefix: &mut self.settings_marker_prefix,
                settings_use_worktree: &mut self.settings_use_worktree,
                settings_scan_exclude: &mut self.settings_scan_exclude,
                settings_output_schema: &mut self.settings_output_schema,
                settings_status: &mut self.settings_status,
                // Voice settings
                voice_settings_mode: &mut self.voice_settings_mode,
                voice_settings_keywords: &mut self.voice_settings_keywords,
                voice_settings_model: &mut self.voice_settings_model,
                voice_settings_language: &mut self.voice_settings_language,
                voice_settings_silence_threshold: &mut self.voice_settings_silence_threshold,
                voice_settings_silence_duration: &mut self.voice_settings_silence_duration,
                voice_settings_max_duration: &mut self.voice_settings_max_duration,
                voice_install_status: &mut self.voice_install_status,
                voice_install_in_progress: &mut self.voice_install_in_progress,
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
                config: &mut self.config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Render modes configuration view
    fn render_modes(&mut self, ctx: &egui::Context) {
        super::modes::render_modes(
            ctx,
            &mut super::modes::ModeEditorState {
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
                view_mode: &mut self.view_mode,
                config: &mut self.config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Render agents configuration view
    fn render_agents(&mut self, ctx: &egui::Context) {
        super::agents::render_agents(
            ctx,
            &mut super::agents::AgentEditorState {
                selected_agent: &mut self.selected_agent,
                agent_edit_name: &mut self.agent_edit_name,
                agent_edit_aliases: &mut self.agent_edit_aliases,
                agent_edit_binary: &mut self.agent_edit_binary,
                agent_edit_cli_type: &mut self.agent_edit_cli_type,
                agent_edit_mode: &mut self.agent_edit_mode,
                agent_edit_print_args: &mut self.agent_edit_print_args,
                agent_edit_output_args: &mut self.agent_edit_output_args,
                agent_edit_repl_args: &mut self.agent_edit_repl_args,
                agent_edit_system_prompt_mode: &mut self.agent_edit_system_prompt_mode,
                agent_edit_disallowed_tools: &mut self.agent_edit_disallowed_tools,
                agent_edit_allowed_tools: &mut self.agent_edit_allowed_tools,
                agent_edit_status: &mut self.agent_edit_status,
                view_mode: &mut self.view_mode,
                config: &mut self.config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Render chains configuration view
    fn render_chains(&mut self, ctx: &egui::Context) {
        super::chains::render_chains(
            ctx,
            &mut super::chains::ChainEditorState {
                selected_chain: &mut self.selected_chain,
                chain_edit_name: &mut self.chain_edit_name,
                chain_edit_description: &mut self.chain_edit_description,
                chain_edit_steps: &mut self.chain_edit_steps,
                chain_edit_stop_on_failure: &mut self.chain_edit_stop_on_failure,
                chain_edit_status: &mut self.chain_edit_status,
                view_mode: &mut self.view_mode,
                config: &mut self.config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Refresh cached jobs from JobManager
    fn refresh_jobs(&mut self) {
        self.cached_jobs = jobs::refresh_jobs(&self.job_manager);
    }

    /// Create a job from the selection popup
    fn create_job_from_selection(&mut self, agent: &str, mode: &str, prompt: &str) -> Option<JobId> {
        jobs::create_job_from_selection(
            &self.job_manager,
            &self.selection,
            agent,
            mode,
            prompt,
            &mut self.logs,
        )
    }

    /// Queue a job for execution
    fn queue_job(&mut self, job_id: JobId) {
        jobs::queue_job(&self.job_manager, job_id, &mut self.logs);
        self.refresh_jobs();
    }

    /// Apply job changes (merge worktree to main)
    fn apply_job(&mut self, job_id: JobId) {
        jobs::apply_job(&self.job_manager, job_id, &mut self.logs);
        self.refresh_jobs();
    }

    /// Reject job changes
    fn reject_job(&mut self, job_id: JobId) {
        jobs::reject_job(&self.job_manager, job_id, &mut self.logs);
        self.refresh_jobs();
    }

    /// Check if a job's completion means a group is ready for comparison
    fn check_group_completion(&mut self, job_id: JobId) {
        // Get the group ID for this job
        let group_id = {
            let manager = match self.job_manager.lock() {
                Ok(m) => m,
                Err(_) => return,
            };
            match manager.get(job_id) {
                Some(job) => job.group_id,
                None => return,
            }
        };

        let group_id = match group_id {
            Some(id) => id,
            None => return, // Job is not part of a group
        };

        // Collect job references for status update
        let jobs: Vec<&Job> = self.cached_jobs.iter().collect();

        // Update group status
        if let Ok(mut gm) = self.group_manager.lock() {
            gm.update_group_status(group_id, &jobs);

            // Check if group is now in Comparing status
            if let Some(group) = gm.get(group_id) {
                if group.status == crate::GroupStatus::Comparing {
                    // Log that the group is ready
                    self.logs.push(LogEvent::system(format!(
                        "Group #{} ready for comparison ({} agents)",
                        group_id,
                        group.job_ids.len()
                    )));
                }
            }
        }
    }

    /// Open the comparison popup for a group
    fn open_comparison_popup(&mut self, group_id: AgentGroupId) {
        // Get the group
        let group = {
            let gm = match self.group_manager.lock() {
                Ok(m) => m,
                Err(_) => return,
            };
            match gm.get(group_id) {
                Some(g) => g.clone(),
                None => return,
            }
        };

        // Collect jobs for this group
        let jobs: Vec<Job> = self
            .cached_jobs
            .iter()
            .filter(|j| j.group_id == Some(group_id))
            .cloned()
            .collect();

        // Open the popup
        self.comparison_state.open(group, jobs);
        self.view_mode = ViewMode::ComparisonPopup;
    }

    /// Handle incoming selection from IDE extension
    fn on_selection_received(&mut self, req: SelectionRequest, ctx: &egui::Context) {
        info!(
            "[kyco:gui] Received selection: file={:?}, lines={:?}-{:?}, deps={:?}, tests={:?}",
            req.file_path, req.line_start, req.line_end, req.dependency_count, req.related_tests.as_ref().map(|t| t.len())
        );

        self.selection = SelectionContext {
            app_name: Some("IDE".to_string()),
            file_path: req.file_path,
            selected_text: req.selected_text,
            line_number: req.line_start,
            line_end: req.line_end,
            possible_files: Vec::new(),
            dependencies: req.dependencies,
            dependency_count: req.dependency_count,
            additional_dependency_count: req.additional_dependency_count,
            related_tests: req.related_tests,
        };

        // Show selection popup
        self.view_mode = ViewMode::SelectionPopup;
        self.popup_input.clear();
        self.popup_status = None;
        self.update_suggestions();

        // Bring window to front
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    /// Update autocomplete suggestions based on input
    fn update_suggestions(&mut self) {
        self.autocomplete.update_suggestions(&self.popup_input, &self.config);
    }

    /// Apply selected suggestion
    fn apply_suggestion(&mut self) {
        if let Some(new_input) = self.autocomplete.apply_suggestion(&self.popup_input) {
            self.popup_input = new_input;
        }
    }

    /// Parse the popup input into agent, mode, and prompt
    fn parse_input(&self) -> (String, String, String) {
        super::selection::autocomplete::parse_input(&self.popup_input)
    }

    /// Execute the task from selection popup
    fn execute_popup_task(&mut self) {
        // Use the multi-agent parser to support "claude+codex+gemini:mode" syntax
        let (agents, mode, prompt) = parse_input_multi(&self.popup_input);

        if mode.is_empty() {
            self.popup_status = Some(("Please enter a mode (e.g., 'refactor', 'fix')".to_string(), true));
            return;
        }

        // Resolve agent aliases
        let resolved_agents: Vec<String> = agents
            .iter()
            .map(|a| {
                self.config
                    .agent
                    .iter()
                    .find(|(name, cfg)| {
                        name.eq_ignore_ascii_case(a) || cfg.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(a))
                    })
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| a.clone())
            })
            .collect();

        // Create job(s) - uses multi-agent creation for parallel execution
        if let Some(result) = jobs::create_jobs_from_selection_multi(
            &self.job_manager,
            &self.group_manager,
            &self.selection,
            &resolved_agents,
            &mode,
            &prompt,
            &mut self.logs,
        ) {
            let selection_info = self
                .selection
                .selected_text
                .as_ref()
                .map(|s| format!("{} chars", s.len()))
                .unwrap_or_else(|| "no selection".to_string());

            if result.job_ids.len() == 1 {
                // Single agent
                let job_id = result.job_ids[0];
                self.popup_status = Some((
                    format!("Job #{} created: {}:{} ({})", job_id, resolved_agents[0], mode, selection_info),
                    false,
                ));
                self.selected_job_id = Some(job_id);
            } else {
                // Multi-agent - show group info
                let agent_list = resolved_agents.join("+");
                self.popup_status = Some((
                    format!(
                        "Group #{} created: {} jobs ({}) for {}:{} ({})",
                        result.group_id.unwrap_or(0),
                        result.job_ids.len(),
                        agent_list,
                        agent_list,
                        mode,
                        selection_info
                    ),
                    false,
                ));
                // Select first job
                self.selected_job_id = result.job_ids.first().copied();
            }

            // Refresh job list
            self.refresh_jobs();

            // Return to job list view after a moment
            self.view_mode = ViewMode::JobList;
        } else {
            self.popup_status = Some(("Failed to create job".to_string(), true));
        }
    }

    /// Write a job request file
    #[allow(dead_code)]
    fn write_job_request(&self, agent: &str, mode: &str, prompt: &str) -> std::io::Result<()> {
        jobs::write_job_request(&self.work_dir, &self.selection, agent, mode, prompt)
    }

    /// Render the job list panel
    fn render_job_list(&mut self, ui: &mut egui::Ui) {
        jobs::render_job_list(ui, &self.cached_jobs, &mut self.selected_job_id);
    }

    /// Render the detail panel
    fn render_detail_panel(&mut self, ui: &mut egui::Ui) {
        use super::detail_panel::{render_detail_panel, DetailPanelAction, DetailPanelState};

        let state = DetailPanelState {
            selected_job_id: self.selected_job_id,
            cached_jobs: &self.cached_jobs,
            logs: &self.logs,
            config: &self.config,
            log_scroll_to_bottom: self.log_scroll_to_bottom,
        };

        if let Some(action) = render_detail_panel(ui, &state) {
            match action {
                DetailPanelAction::Queue(job_id) => self.queue_job(job_id),
                DetailPanelAction::Apply(job_id) => self.apply_job(job_id),
                DetailPanelAction::Reject(job_id) => self.reject_job(job_id),
                DetailPanelAction::ViewDiff(_job_id) => {
                    self.view_mode = ViewMode::DiffView;
                    self.diff_state.set_content("Diff would appear here...".to_string());
                }
            }
        }
    }

    /// Render the selection popup
    fn render_selection_popup(&mut self, ctx: &egui::Context) {
        use super::selection::{render_selection_popup, SelectionPopupAction, SelectionPopupState};

        let mut state = SelectionPopupState {
            selection: &self.selection,
            popup_input: &mut self.popup_input,
            popup_status: &self.popup_status,
            suggestions: &self.autocomplete.suggestions,
            selected_suggestion: self.autocomplete.selected_suggestion,
            show_suggestions: self.autocomplete.show_suggestions,
            cursor_to_end: &mut self.autocomplete.cursor_to_end,
            voice_state: self.voice_manager.state,
            voice_mode: self.voice_manager.config.mode,
            voice_last_error: self.voice_manager.last_error.as_deref(),
        };

        if let Some(action) = render_selection_popup(ctx, &mut state) {
            match action {
                SelectionPopupAction::InputChanged => {
                    self.update_suggestions();
                }
                SelectionPopupAction::SuggestionClicked(idx) => {
                    self.autocomplete.selected_suggestion = idx;
                    self.apply_suggestion();
                    self.update_suggestions();
                }
                SelectionPopupAction::ToggleRecording => {
                    self.voice_manager.toggle_recording();
                }
            }
        }
    }

    /// Render the diff view popup
    fn render_diff_popup(&mut self, ctx: &egui::Context) {
        if super::diff::render_diff_popup(ctx, &self.diff_state) {
            self.view_mode = ViewMode::JobList;
            self.diff_state.clear();
        }
    }

    /// Render the comparison popup for multi-agent results
    fn render_comparison_popup(&mut self, ctx: &egui::Context) {
        if let Some(action) = render_comparison_popup(ctx, &mut self.comparison_state) {
            match action {
                ComparisonAction::SelectJob(job_id) => {
                    // Update the selection in the group manager
                    if let Some(group_id) = self.comparison_state.group_id() {
                        if let Ok(mut gm) = self.group_manager.lock() {
                            gm.select_result(group_id, job_id);
                        }
                    }
                    self.logs.push(LogEvent::system(format!("Selected job #{}", job_id)));
                }
                ComparisonAction::ViewDiff(job_id) => {
                    // Load diff for the selected job
                    if let Some(job) = self.cached_jobs.iter().find(|j| j.id == job_id) {
                        if let Some(worktree) = &job.git_worktree_path {
                            let git_manager = crate::git::GitManager::new(&self.work_dir).ok();
                            if let Some(gm) = git_manager {
                                match gm.diff(worktree) {
                                    Ok(diff) => {
                                        self.diff_state.set_content(diff);
                                        // Note: We stay in comparison mode but could show diff as sub-modal
                                    }
                                    Err(e) => {
                                        self.logs.push(LogEvent::error(format!("Failed to load diff: {}", e)));
                                    }
                                }
                            }
                        }
                    }
                }
                ComparisonAction::MergeAndClose => {
                    // Merge the selected job and cleanup
                    if let Some(group_id) = self.comparison_state.group_id() {
                        let result = super::groups::merge_and_cleanup(
                            group_id,
                            &mut *self.group_manager.lock().unwrap(),
                            &mut *self.job_manager.lock().unwrap(),
                            &crate::git::GitManager::new(&self.work_dir).unwrap(),
                        );

                        if result.success {
                            self.logs.push(LogEvent::system(result.message));
                        } else {
                            self.logs.push(LogEvent::error(result.message));
                        }
                    }

                    // Close popup and return to job list
                    self.comparison_state.close();
                    self.view_mode = ViewMode::JobList;
                    self.refresh_jobs();
                }
                ComparisonAction::Cancel => {
                    // Close popup without merging
                    self.comparison_state.close();
                    self.view_mode = ViewMode::JobList;
                }
            }
        }
    }
}

impl eframe::App for KycoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Refresh jobs periodically (every frame for now, could optimize)
        self.refresh_jobs();

        // Check for HTTP selection events from IDE extensions
        while let Ok(req) = self.http_rx.try_recv() {
            self.on_selection_received(req, ctx);
        }

        // Auto-run: Queue pending jobs automatically when auto_run is enabled
        if self.auto_run {
            let pending_job_ids: Vec<u64> = self
                .cached_jobs
                .iter()
                .filter(|j| j.status == crate::JobStatus::Pending)
                .map(|j| j.id)
                .collect();

            if !pending_job_ids.is_empty() {
                if let Ok(mut manager) = self.job_manager.lock() {
                    for job_id in pending_job_ids {
                        manager.set_status(job_id, crate::JobStatus::Queued);
                        self.logs.push(crate::LogEvent::system(format!(
                            "Auto-queued job #{}",
                            job_id
                        )));
                    }
                }
            }
        }

        // Check for executor events (job status updates, logs)
        while let Ok(event) = self.executor_rx.try_recv() {
            match event {
                ExecutorEvent::JobStarted(job_id) => {
                    self.logs.push(LogEvent::system(format!("Job #{} started", job_id)));
                }
                ExecutorEvent::JobCompleted(job_id) => {
                    self.logs.push(LogEvent::system(format!("Job #{} completed", job_id)));
                    // Check if this job is part of a group and update group status
                    self.check_group_completion(job_id);
                }
                ExecutorEvent::JobFailed(job_id, error) => {
                    self.logs.push(LogEvent::error(format!("Job #{} failed: {}", job_id, error)));
                    // Check if this job is part of a group and update group status
                    self.check_group_completion(job_id);
                }
                ExecutorEvent::ChainStepCompleted { job_id: _, step_index, total_steps, mode, state } => {
                    let state_str = state.as_deref().unwrap_or("none");
                    self.logs.push(LogEvent::system(format!(
                        "Chain step {}/{} completed: {} (state: {})",
                        step_index + 1, total_steps, mode, state_str
                    )));
                }
                ExecutorEvent::ChainCompleted { job_id: _, chain_name, steps_executed, success } => {
                    if success {
                        self.logs.push(LogEvent::system(format!(
                            "Chain '{}' completed: {} steps executed",
                            chain_name, steps_executed
                        )));
                    } else {
                        self.logs.push(LogEvent::error(format!(
                            "Chain '{}' failed after {} steps",
                            chain_name, steps_executed
                        )));
                    }
                }
                ExecutorEvent::Log(log_event) => {
                    self.logs.push(log_event);
                }
            }
        }

        // Process voice events
        for event in self.voice_manager.poll_events() {
            match event {
                super::voice::VoiceEvent::TranscriptionComplete { text } => {
                    // Try to match against voice action registry first
                    if let Some(wakeword_match) = self.voice_manager.config.action_registry.match_text(&text) {
                        // Wakeword matched - use mode and prompt from the match
                        self.popup_input = format!("{} {}", wakeword_match.mode, wakeword_match.get_final_prompt());
                        self.update_suggestions();

                        // Open selection popup if not already open
                        if self.view_mode != ViewMode::SelectionPopup {
                            self.view_mode = ViewMode::SelectionPopup;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        }

                        self.logs.push(LogEvent::system(format!(
                            "Voice wakeword '{}' → mode '{}'",
                            wakeword_match.wakeword, wakeword_match.mode
                        )));
                    } else {
                        // No wakeword match - fall back to legacy keyword parsing
                        let (detected_mode, prompt) = super::voice::parse_voice_input(
                            &text,
                            &self.voice_manager.config.keywords,
                        );

                        // Update input field with transcribed text
                        if let Some(mode) = detected_mode {
                            self.popup_input = format!("{} {}", mode, prompt);
                        } else {
                            // If no mode detected, append to existing input
                            if self.popup_input.is_empty() {
                                self.popup_input = text;
                            } else {
                                self.popup_input.push(' ');
                                self.popup_input.push_str(&text);
                            }
                        }
                        self.update_suggestions();
                        self.logs.push(LogEvent::system("Voice transcription complete".to_string()));
                    }
                }
                super::voice::VoiceEvent::WakewordMatched { wakeword, mode, prompt } => {
                    // Direct wakeword match from continuous listening
                    self.popup_input = format!("{} {}", mode, prompt);
                    self.update_suggestions();

                    // Open selection popup if not already open
                    if self.view_mode != ViewMode::SelectionPopup {
                        self.view_mode = ViewMode::SelectionPopup;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    self.logs.push(LogEvent::system(format!("Voice wakeword: {} → {}", wakeword, mode)));
                }
                super::voice::VoiceEvent::KeywordDetected { keyword, full_text } => {
                    // In continuous mode: keyword detected, trigger hotkey and fill input
                    self.popup_input = format!("{} {}", keyword, full_text.trim_start_matches(&keyword).trim());
                    self.update_suggestions();

                    // Open selection popup if not already open
                    if self.view_mode != ViewMode::SelectionPopup {
                        self.view_mode = ViewMode::SelectionPopup;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    self.logs.push(LogEvent::system(format!("Voice keyword detected: {}", keyword)));
                }
                super::voice::VoiceEvent::Error { message } => {
                    self.logs.push(LogEvent::error(format!("Voice error: {}", message)));
                }
                super::voice::VoiceEvent::RecordingStarted => {
                    self.logs.push(LogEvent::system("Voice recording started".to_string()));
                }
                super::voice::VoiceEvent::RecordingStopped { duration_secs } => {
                    self.logs.push(LogEvent::system(format!("Voice recording stopped ({:.1}s)", duration_secs)));
                }
                super::voice::VoiceEvent::VadSpeechStarted => {
                    self.logs.push(LogEvent::system("VAD: Speech detected".to_string()));
                }
                super::voice::VoiceEvent::VadSpeechEnded => {
                    self.logs.push(LogEvent::system("VAD: Speech ended".to_string()));
                }
                _ => {}
            }
        }

        // Handle keyboard shortcuts
        ctx.input(|i| {
            match self.view_mode {
                ViewMode::SelectionPopup => {
                    if i.key_pressed(Key::Escape) {
                        self.view_mode = ViewMode::JobList;
                    }
                    if i.key_pressed(Key::Tab) && self.autocomplete.show_suggestions && !self.autocomplete.suggestions.is_empty() {
                        self.apply_suggestion();
                        self.update_suggestions();
                    }
                    if i.key_pressed(Key::ArrowDown) && self.autocomplete.show_suggestions {
                        self.autocomplete.select_next();
                    }
                    if i.key_pressed(Key::ArrowUp) && self.autocomplete.show_suggestions {
                        self.autocomplete.select_previous();
                    }
                    if i.key_pressed(Key::Enter) {
                        self.execute_popup_task();
                    }
                }
                ViewMode::DiffView => {
                    if i.key_pressed(Key::Escape) || i.key_pressed(Key::Q) {
                        self.view_mode = ViewMode::JobList;
                        self.diff_state.clear();
                    }
                }
                ViewMode::ComparisonPopup => {
                    if i.key_pressed(Key::Escape) || i.key_pressed(Key::Q) {
                        self.comparison_state.close();
                        self.view_mode = ViewMode::JobList;
                    }
                }
                ViewMode::JobList => {
                    // Navigate jobs with j/k or arrows
                    if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
                        // Select next job
                        if let Some(current_id) = self.selected_job_id {
                            if let Some(idx) = self.cached_jobs.iter().position(|j| j.id == current_id) {
                                if idx + 1 < self.cached_jobs.len() {
                                    self.selected_job_id = Some(self.cached_jobs[idx + 1].id);
                                }
                            }
                        } else if !self.cached_jobs.is_empty() {
                            self.selected_job_id = Some(self.cached_jobs[0].id);
                        }
                    }
                    if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
                        // Select previous job
                        if let Some(current_id) = self.selected_job_id {
                            if let Some(idx) = self.cached_jobs.iter().position(|j| j.id == current_id) {
                                if idx > 0 {
                                    self.selected_job_id = Some(self.cached_jobs[idx - 1].id);
                                }
                            }
                        }
                    }
                }
                ViewMode::Settings => {
                    if i.key_pressed(Key::Escape) {
                        self.view_mode = ViewMode::JobList;
                    }
                }
                ViewMode::Modes => {
                    if i.key_pressed(Key::Escape) {
                        if self.selected_mode.is_some() {
                            self.selected_mode = None;
                            self.mode_edit_status = None;
                        } else {
                            self.view_mode = ViewMode::JobList;
                        }
                    }
                }
                ViewMode::Agents => {
                    if i.key_pressed(Key::Escape) {
                        if self.selected_agent.is_some() {
                            self.selected_agent = None;
                            self.agent_edit_status = None;
                        } else {
                            self.view_mode = ViewMode::JobList;
                        }
                    }
                }
                ViewMode::Chains => {
                    if i.key_pressed(Key::Escape) {
                        if self.selected_chain.is_some() {
                            self.selected_chain = None;
                            self.chain_edit_status = None;
                        } else {
                            self.view_mode = ViewMode::JobList;
                        }
                    }
                }
            }

            // Global shortcuts for auto_run and auto_scan toggles (Shift+A and Shift+S)
            if i.modifiers.shift && i.key_pressed(Key::A) {
                self.auto_run = !self.auto_run;
            }
            if i.modifiers.shift && i.key_pressed(Key::S) {
                self.auto_scan = !self.auto_scan;
            }

            // Voice hotkey handling (Shift+V to toggle recording in selection popup)
            if self.view_mode == ViewMode::SelectionPopup {
                // Check for voice recording hotkey (Shift+V or Cmd/Ctrl+Shift+V)
                let voice_hotkey_pressed = i.modifiers.shift && i.key_pressed(Key::V);

                if self.voice_manager.config.mode == VoiceInputMode::HotkeyHold {
                    // Hotkey-hold mode: start recording when pressed, stop when released
                    let hotkey_down = i.modifiers.shift && i.keys_down.contains(&Key::V);

                    if hotkey_down && !self.hotkey_held {
                        // Hotkey just pressed - start recording
                        self.hotkey_held = true;
                        self.voice_manager.start_recording();
                    } else if !hotkey_down && self.hotkey_held {
                        // Hotkey just released - stop recording and transcribe
                        self.hotkey_held = false;
                        self.voice_manager.stop_recording();
                    }
                } else if voice_hotkey_pressed {
                    // Manual mode: toggle recording on press
                    if self.voice_manager.config.mode == VoiceInputMode::Manual {
                        self.voice_manager.toggle_recording();
                    }
                }
            }

            // Global shortcut to toggle continuous listening (Shift+L)
            if i.modifiers.shift && i.key_pressed(Key::L) {
                if self.voice_manager.config.mode == VoiceInputMode::Continuous {
                    self.voice_manager.toggle_listening();
                }
            }
        });

        // Apply theme
        let mut style = (*ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.panel_fill = BG_PRIMARY;
        style.visuals.window_fill = BG_PRIMARY;
        style.visuals.extreme_bg_color = BG_SECONDARY;
        style.visuals.widgets.noninteractive.bg_fill = BG_SECONDARY;
        style.visuals.widgets.inactive.bg_fill = BG_SECONDARY;
        style.visuals.widgets.hovered.bg_fill = BG_HIGHLIGHT;
        style.visuals.widgets.active.bg_fill = BG_HIGHLIGHT;
        style.visuals.selection.bg_fill = BG_HIGHLIGHT;
        style.visuals.selection.stroke = Stroke::new(1.0, TEXT_PRIMARY);
        ctx.set_style(style);

        // Render based on view mode
        match self.view_mode {
            ViewMode::JobList => {
                egui::SidePanel::left("job_list")
                    .default_width(280.0)
                    .min_width(280.0)
                    .max_width(280.0)
                    .resizable(false)
                    .frame(egui::Frame::none().fill(BG_PRIMARY).inner_margin(8.0))
                    .show(ctx, |ui| {
                        self.render_job_list(ui);
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(BG_PRIMARY).inner_margin(8.0))
                    .show(ctx, |ui| {
                        self.render_detail_panel(ui);
                    });
            }
            ViewMode::SelectionPopup => {
                // Show main UI dimmed behind popup
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_selection_popup(ctx);
            }
            ViewMode::DiffView => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_diff_popup(ctx);
            }
            ViewMode::ComparisonPopup => {
                // Show main UI dimmed behind popup
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_comparison_popup(ctx);
            }
            ViewMode::Settings => {
                self.render_settings(ctx);
            }
            ViewMode::Modes => {
                self.render_modes(ctx);
            }
            ViewMode::Agents => {
                self.render_agents(ctx);
            }
            ViewMode::Chains => {
                self.render_chains(ctx);
            }
        }

        // Bottom status bar
        super::status_bar::render_status_bar(
            ctx,
            &mut super::status_bar::StatusBarState {
                auto_run: &mut self.auto_run,
                auto_scan: &mut self.auto_scan,
                view_mode: &mut self.view_mode,
                selected_mode: &mut self.selected_mode,
                mode_edit_status: &mut self.mode_edit_status,
                selected_agent: &mut self.selected_agent,
                agent_edit_status: &mut self.agent_edit_status,
                selected_chain: &mut self.selected_chain,
                chain_edit_status: &mut self.chain_edit_status,
            },
        );

        // Request continuous updates
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

// Keep old types for compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Refactor,
    Fix,
    Tests,
    Docs,
    Review,
    Optimize,
    Implement,
    Custom,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Refactor => "refactor",
            Mode::Fix => "fix",
            Mode::Tests => "tests",
            Mode::Docs => "docs",
            Mode::Review => "review",
            Mode::Optimize => "optimize",
            Mode::Implement => "implement",
            Mode::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agent {
    Claude,
    Codex,
    Gemini,
}

impl Agent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
            Agent::Gemini => "gemini",
        }
    }
}
