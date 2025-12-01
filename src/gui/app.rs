//! Main GUI application using egui
//!
//! Full-featured GUI replacing the TUI with:
//! - Job list panel (left)
//! - Detail panel with logs (right)
//! - Selection popup for IDE extension input
//! - Controls for job management

use super::diff::DiffState;
use super::executor::ExecutorEvent;
use super::http_server::SelectionRequest;
use super::jobs;
use super::voice::{VoiceConfig, VoiceInputMode, VoiceManager};
use crate::config::Config;
use crate::job::JobManager;
use crate::{Job, JobId, LogEvent};
use eframe::egui::{self, Color32, Key, Stroke};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use tracing::info;

// ═══════════════════════════════════════════════════════════════════════════
// Selection Context - Information about the current selection from IDE
// ═══════════════════════════════════════════════════════════════════════════

/// Information about the current selection context (received from IDE extensions)
#[derive(Debug, Clone, Default)]
pub struct SelectionContext {
    /// Name of the focused application (e.g., "Visual Studio Code", "IntelliJ IDEA")
    pub app_name: Option<String>,
    /// Path to the current file (if detectable)
    pub file_path: Option<String>,
    /// The selected text
    pub selected_text: Option<String>,
    /// Start line number (if available)
    pub line_number: Option<usize>,
    /// End line number (if available)
    pub line_end: Option<usize>,
    /// Multiple file matches (when file_path couldn't be determined uniquely)
    pub possible_files: Vec<String>,
}

impl SelectionContext {
    /// Check if we have any useful selection data
    pub fn has_selection(&self) -> bool {
        self.selected_text.as_ref().map_or(false, |s| !s.is_empty())
    }
}

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

// REMOVED: Hardcoded MODES and AGENTS constants
// Modes and agents are now dynamically loaded from config.toml
// via self.config.mode and self.config.agent in update_suggestions()

/// Autocomplete suggestion
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub text: String,
    pub description: String,
    pub category: &'static str,
}

/// View mode for the app
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Main job list view
    JobList,
    /// Selection popup (triggered by IDE extension)
    SelectionPopup,
    /// Diff view popup
    DiffView,
    /// Settings/Extensions view
    Settings,
    /// Modes configuration view
    Modes,
    /// Agents configuration view
    Agents,
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
    /// Autocomplete suggestions
    suggestions: Vec<Suggestion>,
    /// Selected suggestion index
    selected_suggestion: usize,
    /// Show suggestions dropdown
    show_suggestions: bool,
    /// Request cursor to move to end of input
    cursor_to_end: bool,
    /// Status message for popup
    popup_status: Option<(String, bool)>,
    /// Diff view state
    diff_state: DiffState,
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
    /// Hotkey is currently held (for hotkey_hold mode)
    hotkey_held: bool,
    /// Voice installation status message
    voice_install_status: Option<(String, bool)>,
    /// Voice installation in progress
    voice_install_in_progress: bool,
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
        let voice_config = VoiceConfig {
            mode: match voice_settings.mode.as_str() {
                "manual" => VoiceInputMode::Manual,
                "hotkey_hold" => VoiceInputMode::HotkeyHold,
                "continuous" => VoiceInputMode::Continuous,
                _ => VoiceInputMode::Disabled,
            },
            keywords: voice_settings.keywords.clone(),
            whisper_model: voice_settings.whisper_model.clone(),
            language: voice_settings.language.clone(),
            silence_threshold: voice_settings.silence_threshold,
            silence_duration: voice_settings.silence_duration,
            max_duration: voice_settings.max_duration,
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
            cached_jobs: Vec::new(),
            selected_job_id: None,
            logs: vec![LogEvent::system("kyco GUI started")],
            http_rx,
            executor_rx,
            selection: SelectionContext::default(),
            view_mode: ViewMode::JobList,
            popup_input: String::new(),
            suggestions: Vec::new(),
            selected_suggestion: 0,
            show_suggestions: false,
            cursor_to_end: false,
            popup_status: None,
            diff_state: DiffState::new(),
            auto_run: false,
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
            hotkey_held: false,
            voice_install_status: None,
            voice_install_in_progress: false,
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

    /// Handle incoming selection from IDE extension
    fn on_selection_received(&mut self, req: SelectionRequest, ctx: &egui::Context) {
        info!(
            "[kyco:gui] Received selection: file={:?}, lines={:?}-{:?}",
            req.file_path, req.line_start, req.line_end
        );

        self.selection = SelectionContext {
            app_name: Some("IDE".to_string()),
            file_path: req.file_path,
            selected_text: req.selected_text,
            line_number: req.line_start,
            line_end: req.line_end,
            possible_files: Vec::new(),
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
        self.suggestions.clear();
        self.selected_suggestion = 0;

        let input_lower = self.popup_input.to_lowercase();
        let input_trimmed = input_lower.trim();

        // Get default agent from config
        let default_agent = &self.config.settings.gui.default_agent;

        if input_trimmed.is_empty() {
            // Show agents first, then modes when empty
            for (agent_name, agent_config) in &self.config.agent {
                let desc = format!("{} ({})", agent_config.binary, agent_config.aliases.join(", "));
                self.suggestions.push(Suggestion {
                    text: format!("{}:", agent_name),
                    description: desc,
                    category: "agent",
                });
            }
            for (mode_name, mode_config) in &self.config.mode {
                let aliases = if mode_config.aliases.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", mode_config.aliases.join(", "))
                };
                let agent_hint = mode_config.agent.as_deref().unwrap_or(default_agent);
                self.suggestions.push(Suggestion {
                    text: mode_name.to_string(),
                    description: format!("default: {}{}", agent_hint, aliases),
                    category: "mode",
                });
            }
            self.show_suggestions = true;
            return;
        }

        // Check if we have "agent:" prefix - show modes after colon
        if let Some(colon_pos) = input_trimmed.find(':') {
            let agent_part = &input_trimmed[..colon_pos];
            let mode_part = &input_trimmed[colon_pos + 1..];

            // After colon, show matching modes
            for (mode_name, mode_config) in &self.config.mode {
                let mode_lower = mode_name.to_lowercase();
                let matches_mode = mode_lower.starts_with(mode_part) || mode_part.is_empty();
                let matches_alias = mode_config.aliases.iter().any(|a| a.to_lowercase().starts_with(mode_part));

                if matches_mode || matches_alias {
                    let aliases = if mode_config.aliases.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", mode_config.aliases.join(", "))
                    };
                    self.suggestions.push(Suggestion {
                        text: format!("{}:{}", agent_part, mode_name),
                        description: aliases,
                        category: "mode",
                    });
                }
            }
        } else {
            // No colon yet - show matching agents and modes
            // First show matching agents (by name or alias)
            for (agent_name, agent_config) in &self.config.agent {
                let name_lower = agent_name.to_lowercase();
                let matches_name = name_lower.starts_with(input_trimmed);
                let matches_alias = agent_config.aliases.iter().any(|a| a.to_lowercase().starts_with(input_trimmed));

                if matches_name || matches_alias {
                    let desc = format!("{} ({})", agent_config.binary, agent_config.aliases.join(", "));
                    self.suggestions.push(Suggestion {
                        text: format!("{}:", agent_name),
                        description: desc,
                        category: "agent",
                    });
                }
            }

            // Then show matching modes (uses default agent)
            for (mode_name, mode_config) in &self.config.mode {
                let mode_lower = mode_name.to_lowercase();
                let matches_mode = mode_lower.starts_with(input_trimmed);
                let matches_alias = mode_config.aliases.iter().any(|a| a.to_lowercase().starts_with(input_trimmed));

                if matches_mode || matches_alias {
                    let aliases = if mode_config.aliases.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", mode_config.aliases.join(", "))
                    };
                    let agent_hint = mode_config.agent.as_deref().unwrap_or(default_agent);
                    self.suggestions.push(Suggestion {
                        text: mode_name.to_string(),
                        description: format!("default: {}{}", agent_hint, aliases),
                        category: "mode",
                    });
                }
            }
        }

        self.show_suggestions = !self.suggestions.is_empty();
    }

    /// Apply selected suggestion
    fn apply_suggestion(&mut self) {
        if let Some(suggestion) = self.suggestions.get(self.selected_suggestion) {
            if suggestion.text.ends_with(':') {
                self.popup_input = suggestion.text.clone();
            } else {
                self.popup_input = format!("{} ", suggestion.text);
            }
            self.show_suggestions = false;
            self.cursor_to_end = true; // Request cursor move to end
        }
    }

    /// Parse the popup input into agent, mode, and prompt
    fn parse_input(&self) -> (String, String, String) {
        let input = self.popup_input.trim();

        let (command, prompt) = match input.find(' ') {
            Some(pos) => (&input[..pos], input[pos + 1..].trim()),
            None => (input, ""),
        };

        let (agent, mode) = match command.find(':') {
            Some(pos) => (&command[..pos], &command[pos + 1..]),
            None => ("claude", command),
        };

        (agent.to_string(), mode.to_string(), prompt.to_string())
    }

    /// Execute the task from selection popup
    fn execute_popup_task(&mut self) {
        let (agent, mode, prompt) = self.parse_input();

        if mode.is_empty() {
            self.popup_status = Some(("Please enter a mode (e.g., 'refactor', 'fix')".to_string(), true));
            return;
        }

        // Create job in JobManager
        if let Some(job_id) = self.create_job_from_selection(&agent, &mode, &prompt) {
            let selection_info = self
                .selection
                .selected_text
                .as_ref()
                .map(|s| format!("{} chars", s.len()))
                .unwrap_or_else(|| "no selection".to_string());

            self.popup_status = Some((
                format!("Job #{} created: {}:{} ({})", job_id, agent, mode, selection_info),
                false,
            ));

            // Refresh job list and select new job
            self.refresh_jobs();
            self.selected_job_id = Some(job_id);

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
        use super::selection_popup::{render_selection_popup, SelectionPopupAction, SelectionPopupState};

        let mut state = SelectionPopupState {
            selection: &self.selection,
            popup_input: &mut self.popup_input,
            popup_status: &self.popup_status,
            suggestions: &self.suggestions,
            selected_suggestion: self.selected_suggestion,
            show_suggestions: self.show_suggestions,
            cursor_to_end: &mut self.cursor_to_end,
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
                    self.selected_suggestion = idx;
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
}

impl eframe::App for KycoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Refresh jobs periodically (every frame for now, could optimize)
        self.refresh_jobs();

        // Check for HTTP selection events from IDE extensions
        while let Ok(req) = self.http_rx.try_recv() {
            self.on_selection_received(req, ctx);
        }

        // Check for executor events (job status updates, logs)
        while let Ok(event) = self.executor_rx.try_recv() {
            match event {
                ExecutorEvent::JobStarted(job_id) => {
                    self.logs.push(LogEvent::system(format!("Job #{} started", job_id)));
                }
                ExecutorEvent::JobCompleted(job_id) => {
                    self.logs.push(LogEvent::system(format!("Job #{} completed", job_id)));
                }
                ExecutorEvent::JobFailed(job_id, error) => {
                    self.logs.push(LogEvent::error(format!("Job #{} failed: {}", job_id, error)));
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
                    // Parse the transcription for mode keyword
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
                    if i.key_pressed(Key::Tab) && self.show_suggestions && !self.suggestions.is_empty() {
                        self.apply_suggestion();
                        self.update_suggestions();
                    }
                    if i.key_pressed(Key::ArrowDown) && self.show_suggestions {
                        self.selected_suggestion = (self.selected_suggestion + 1) % self.suggestions.len().max(1);
                    }
                    if i.key_pressed(Key::ArrowUp) && self.show_suggestions {
                        self.selected_suggestion = self.selected_suggestion.saturating_sub(1);
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
                    .resizable(true)
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
            ViewMode::Settings => {
                self.render_settings(ctx);
            }
            ViewMode::Modes => {
                self.render_modes(ctx);
            }
            ViewMode::Agents => {
                self.render_agents(ctx);
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
