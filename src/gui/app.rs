//! Main GUI application using egui
//!
//! Full-featured GUI replacing the TUI with:
//! - Job list panel (left)
//! - Detail panel with logs (right)
//! - Selection popup for IDE extension input
//! - Controls for job management

use super::detail_panel::ActivityLogFilters;
use super::diff::DiffState;
use super::executor::ExecutorEvent;
use super::groups::{ComparisonAction, ComparisonState, render_comparison_popup};
use super::http_server::{BatchFile, BatchRequest, SelectionRequest};
use super::jobs;
use super::permission::{
    PermissionAction, PermissionPopupState, PermissionRequest, render_permission_popup,
};
use super::selection::autocomplete::parse_input_multi;
use super::selection::{AutocompleteState, SelectionContext};
use super::update::{UpdateChecker, UpdateStatus};
use super::voice::{VoiceConfig, VoiceInputMode, VoiceManager, VoiceState, copy_and_paste};
use global_hotkey::{GlobalHotKeyManager, GlobalHotKeyEvent, hotkey::{HotKey, Modifiers, Code}};
use crate::agent::TerminalSession;
use crate::agent::bridge::{BridgeClient, PermissionMode, ToolApprovalResponse, ToolDecision};
use crate::config::Config;
use crate::job::{GroupManager, JobManager};
use crate::workspace::{WorkspaceId, WorkspaceRegistry};
use crate::{AgentGroupId, Job, JobId, LogEvent, SdkType};
use eframe::egui::{self, Color32, Key, Stroke};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

// ═══════════════════════════════════════════════════════════════════════════
// MEMORY LIMITS - Prevent unbounded growth
// ═══════════════════════════════════════════════════════════════════════════

/// Maximum number of log entries to keep in memory (FIFO eviction)
const MAX_GLOBAL_LOGS: usize = 500;

/// Minimal default config for GUI initialization
const DEFAULT_CONFIG_MINIMAL: &str = r#"# KYCo Configuration
# Run `kyco init` for full configuration with all options

[settings]
max_concurrent_jobs = 4
auto_run = false
use_worktree = false

[agent.claude]
aliases = ["c", "cl"]
binary = "claude"

[mode.refactor]
aliases = ["r", "ref"]
prompt = "Refactor this code to improve readability and maintainability."

[mode.fix]
aliases = ["f"]
prompt = "Fix the bug or issue in this code."

[mode.test]
aliases = ["t"]
prompt = "Write comprehensive unit tests for this code."
"#;

const ORCHESTRATOR_SYSTEM_PROMPT: &str = r#"
You are an interactive KYCo Orchestrator running in the user's workspace.

Your job is to help the user run KYCo jobs (modes/chains) safely and iteratively.

Rules
- Do NOT directly edit repository files yourself. Use KYCo jobs so the user can review diffs in the KYCo GUI.
- Use the `Bash` tool to run `kyco ...` commands.
- Before starting a large batch of jobs, confirm the plan with the user.
- Before changing `.kyco/config.toml` (mode CRUD), ask for explicit confirmation.

Discovery
- List available agents: `kyco agent list`
- List available modes: `kyco mode list`
- List available chains: `kyco chain list`

Job lifecycle (GUI must be running)
- Start a job (creates + queues by default):
  `kyco job start --file <path> --mode <mode_or_chain> --prompt "<what to do>" [--agent <id>] [--agents a,b] [--force-worktree]`
- Abort/stop a job:
  `kyco job abort <job_id>`
- Delete a job from the GUI list:
  `kyco job delete <job_id> [--cleanup-worktree]`
- Wait until done/failed/rejected/merged:
  `kyco job wait <job_id>`
- Fetch output:
  - Full output: `kyco job output <job_id>`
  - Summary only: `kyco job output <job_id> --summary`
  - State only: `kyco job output <job_id> --state`
- Inspect job JSON: `kyco job get <job_id> --json`
- Continue a session job with a follow-up prompt (creates a new job):
  `kyco job continue <job_id> --prompt "<follow-up>" [--pending]`

Mode CRUD (only with explicit user confirmation)
- Create/update mode: `kyco mode set <name> [--prompt ...] [--system-prompt ...] [--aliases ...] [--readonly] ...`
- Delete mode: `kyco mode delete <name>`

Batch job creation (efficient pattern for many files)
- Use ripgrep to find files, write to a temp file, then loop:
  ```bash
  # Example: Add tests to all .rs files in src/
  rg --files -g '*.rs' src/ > /tmp/files.txt
  while read file; do
    kyco job start --file "$file" --mode test --prompt "Add unit tests" --pending
  done < /tmp/files.txt
  ```
- Use --pending flag to create jobs without auto-queueing (review first in GUI)
- For multi-agent comparison: `--agents claude,codex` creates parallel jobs

Orchestration pattern
- Start a job, wait for completion, read its output/state, then decide follow-ups.
- If you start multiple jobs, keep track of IDs and report progress to the user.
- For batch operations: create all jobs first (--pending), let user review in GUI, then queue.
"#;

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
pub(super) const STATUS_BLOCKED: Color32 = Color32::from_rgb(255, 165, 0); // Orange - waiting for file lock
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

/// View mode for the app
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Main job list view
    JobList,
    /// Selection popup (triggered by IDE extension)
    SelectionPopup,
    /// Batch selection popup (triggered by IDE batch request)
    BatchPopup,
    /// Diff view popup
    DiffView,
    /// Apply/merge confirmation popup
    ApplyConfirmPopup,
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

#[derive(Debug, Clone)]
enum ApplyTarget {
    Single {
        job_id: JobId,
    },
    Group {
        group_id: AgentGroupId,
        selected_job_id: JobId,
    },
}

#[derive(Debug, Clone)]
struct ApplyThreadOutcome {
    target: ApplyTarget,
    /// For group merges: all job IDs in the group (empty for single jobs).
    group_job_ids: Vec<JobId>,
    message: String,
}

#[derive(Debug, Clone)]
enum ApplyThreadInput {
    Single(SingleApplyInput),
    Group(GroupApplyInput),
}

#[derive(Debug, Clone)]
struct SingleApplyInput {
    job_id: JobId,
    workspace_root: PathBuf,
    worktree_path: Option<PathBuf>,
    base_branch: Option<String>,
    commit_message: crate::git::CommitMessage,
}

#[derive(Debug, Clone)]
struct GroupApplyInput {
    group_id: AgentGroupId,
    selected_job_id: JobId,
    selected_agent_id: String,
    workspace_root: PathBuf,
    selected_worktree_path: PathBuf,
    base_branch: String,
    commit_message: crate::git::CommitMessage,
    cleanup_worktrees: Vec<(JobId, PathBuf)>,
    group_job_ids: Vec<JobId>,
}

/// Main application state
pub struct KycoApp {
    /// Working directory
    work_dir: PathBuf,
    /// Configuration
    #[allow(dead_code)]
    config: Arc<RwLock<Config>>,
    /// Whether config file exists (show init button if not)
    config_exists: bool,
    /// Job manager (shared with async tasks)
    job_manager: Arc<Mutex<JobManager>>,
    /// Group manager for multi-agent parallel execution
    group_manager: Arc<Mutex<GroupManager>>,
    /// Workspace registry for multi-repository support
    workspace_registry: Arc<Mutex<WorkspaceRegistry>>,
    /// Currently active workspace ID (for filtering jobs in UI)
    active_workspace_id: Option<WorkspaceId>,
    /// Cached jobs for display (updated only when changed)
    cached_jobs: Vec<Job>,
    /// Last known job manager generation (for change detection)
    last_job_generation: u64,
    /// Selected job ID
    selected_job_id: Option<u64>,
    /// Job list filter
    job_list_filter: jobs::JobListFilter,
    /// Log events
    logs: Vec<LogEvent>,
    /// Receiver for HTTP selection events from IDE extensions
    http_rx: Receiver<SelectionRequest>,
    /// Receiver for batch processing requests from IDE extensions
    batch_rx: Receiver<BatchRequest>,
    /// Receiver for executor events
    executor_rx: Receiver<ExecutorEvent>,
    /// Shared max concurrent jobs (runtime-adjustable)
    max_concurrent_jobs: Arc<AtomicUsize>,
    /// Current selection context (from IDE extension)
    selection: SelectionContext,
    /// Batch files for batch processing (from IDE extension)
    batch_files: Vec<BatchFile>,
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
    /// View mode to return to after closing diff
    diff_return_view: ViewMode,
    /// Inline diff content for detail panel (loaded when job selected)
    inline_diff_content: Option<String>,
    /// Previously selected job ID (to detect selection changes)
    prev_selected_job_id: Option<u64>,
    /// Pending merge/apply confirmation (shown in ApplyConfirmPopup)
    apply_confirm_target: Option<ApplyTarget>,
    /// View mode to return to after canceling apply confirmation
    apply_confirm_return_view: ViewMode,
    /// Error message shown in apply confirmation popup
    apply_confirm_error: Option<String>,
    /// Receiver for async apply/merge results
    apply_confirm_rx: Option<std::sync::mpsc::Receiver<Result<ApplyThreadOutcome, String>>>,
    /// Markdown rendering cache (for agent responses)
    commonmark_cache: egui_commonmark::CommonMarkCache,
    /// Comparison popup state for multi-agent results
    comparison_state: ComparisonState,
    /// Permission popup state for tool approval requests
    permission_state: PermissionPopupState,
    /// Bridge client for sending tool approval responses
    bridge_client: BridgeClient,
    /// Current Claude permission mode overrides per job (UI state)
    permission_mode_overrides: HashMap<JobId, PermissionMode>,
    /// Auto-run enabled
    auto_run: bool,
    /// Log scroll to bottom
    log_scroll_to_bottom: bool,
    /// Activity log kind filters (UI state)
    activity_log_filters: ActivityLogFilters,
    /// Session continuation prompt (for follow-up messages in session mode)
    continuation_prompt: String,
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
    /// Agent editor: sdk type (claude/codex)
    agent_edit_cli_type: String,
    /// Agent editor: session mode (oneshot/session)
    agent_edit_mode: String,
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
    /// Mode editor: session mode (oneshot/session)
    mode_edit_session_mode: String,
    /// Mode editor: max turns (0 = unlimited)
    mode_edit_max_turns: String,
    /// Mode editor: model override
    mode_edit_model: String,
    /// Mode editor: Claude permission mode
    mode_edit_claude_permission: String,
    /// Mode editor: Codex sandbox mode
    mode_edit_codex_sandbox: String,
    /// Mode editor: output states (comma-separated)
    mode_edit_output_states: String,
    /// Mode editor: state prompt for chain workflows
    mode_edit_state_prompt: String,
    /// Settings editor: max concurrent jobs
    settings_max_concurrent: String,
    /// Settings editor: auto run
    settings_auto_run: bool,
    /// Settings editor: use worktree
    settings_use_worktree: bool,
    /// Settings editor: output schema template
    settings_output_schema: String,
    /// Settings editor: structured output JSON schema (optional)
    settings_structured_output_schema: String,
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
    /// Voice settings editor: global hotkey (dictate from any app)
    voice_settings_global_hotkey: String,
    /// Voice settings editor: popup hotkey (start/stop in selection popup)
    voice_settings_popup_hotkey: String,
    /// VAD settings: enabled
    vad_enabled: bool,
    /// VAD settings: speech threshold
    vad_speech_threshold: String,
    /// VAD settings: silence duration (ms)
    vad_silence_duration_ms: String,
    /// Voice installation status message
    voice_install_status: Option<(String, bool)>,
    /// Voice installation in progress
    voice_install_in_progress: bool,
    /// Voice test status
    voice_test_status: super::settings::VoiceTestStatus,
    /// Voice test result (transcribed text)
    voice_test_result: Option<String>,
    /// Flag to indicate voice config was changed and VoiceManager needs to be updated
    voice_config_changed: bool,
    /// Flag to execute popup task after voice transcription completes
    voice_pending_execute: bool,
    /// Update checker for new version notifications
    update_checker: UpdateChecker,
    /// Update install status
    update_install_status: super::status_bar::InstallStatus,
    /// Receiver for install results
    update_install_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    /// Selected chain for editing (None = list view)
    selected_chain: Option<String>,
    /// Chain editor: name field
    chain_edit_name: String,
    /// Chain editor: description field
    chain_edit_description: String,
    /// Chain editor: state definitions
    chain_edit_states: Vec<super::chains::state::StateDefinitionEdit>,
    /// Chain editor: steps
    chain_edit_steps: Vec<super::chains::state::ChainStepEdit>,
    /// Chain editor: stop on failure
    chain_edit_stop_on_failure: bool,
    /// Chain editor: pass full response to next step
    chain_edit_pass_full_response: bool,
    /// Chain editor: status message
    chain_edit_status: Option<(String, bool)>,
    /// Chain editor: pending confirmation dialog
    chain_pending_confirmation: super::chains::PendingConfirmation,
    /// Config import: selected workspace index
    import_workspace_selected: usize,
    /// Config import: import modes
    import_modes: bool,
    /// Config import: import agents
    import_agents: bool,
    /// Config import: import chains
    import_chains: bool,
    /// Config import: import settings
    import_settings: bool,

    /// UI action: launch an external orchestrator session (Terminal)
    orchestrator_requested: bool,

    /// Last time we ran log truncation (to avoid running every frame)
    last_log_cleanup: std::time::Instant,

    // ═══════════════════════════════════════════════════════════════════════════
    // GLOBAL VOICE HOTKEY - Voice input from any app (Cmd+Shift+V)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Global hotkey manager for voice input
    global_hotkey_manager: Option<GlobalHotKeyManager>,
    /// Registered voice hotkey ID (for future multi-hotkey support)
    #[allow(dead_code)]
    voice_hotkey: Option<HotKey>,
    /// Whether global voice recording is active (triggered by hotkey, not popup)
    global_voice_recording: bool,
    /// Auto-paste after transcription (for global voice input)
    global_voice_auto_paste: bool,
    /// Show voice overlay window (small indicator when recording)
    show_voice_overlay: bool,
}

/// Check if a hotkey string matches the current egui input state
/// Returns true if the hotkey is pressed
fn check_egui_hotkey(input: &egui::InputState, hotkey_str: &str) -> bool {
    let hotkey_lower = hotkey_str.to_lowercase();
    let parts: Vec<&str> = hotkey_lower.split('+').collect();
    if parts.is_empty() {
        return false;
    }

    let key_part = match parts.last() {
        Some(k) => *k,
        None => return false,
    };

    // Check modifiers
    let mut need_cmd = false;
    let mut need_ctrl = false;
    let mut need_alt = false;
    let mut need_shift = false;

    for part in &parts[..parts.len() - 1] {
        match *part {
            "cmd" | "command" | "super" | "win" => need_cmd = true,
            "ctrl" | "control" => need_ctrl = true,
            "alt" | "option" => need_alt = true,
            "shift" => need_shift = true,
            _ => {}
        }
    }

    // Check if modifiers match (use command for cmd on all platforms in egui)
    let mods_match = input.modifiers.command == need_cmd
        && input.modifiers.ctrl == need_ctrl
        && input.modifiers.alt == need_alt
        && input.modifiers.shift == need_shift;

    if !mods_match {
        return false;
    }

    // Map key string to egui Key
    let key = match key_part {
        "a" => Key::A,
        "b" => Key::B,
        "c" => Key::C,
        "d" => Key::D,
        "e" => Key::E,
        "f" => Key::F,
        "g" => Key::G,
        "h" => Key::H,
        "i" => Key::I,
        "j" => Key::J,
        "k" => Key::K,
        "l" => Key::L,
        "m" => Key::M,
        "n" => Key::N,
        "o" => Key::O,
        "p" => Key::P,
        "q" => Key::Q,
        "r" => Key::R,
        "s" => Key::S,
        "t" => Key::T,
        "u" => Key::U,
        "v" => Key::V,
        "w" => Key::W,
        "x" => Key::X,
        "y" => Key::Y,
        "z" => Key::Z,
        "0" => Key::Num0,
        "1" => Key::Num1,
        "2" => Key::Num2,
        "3" => Key::Num3,
        "4" => Key::Num4,
        "5" => Key::Num5,
        "6" => Key::Num6,
        "7" => Key::Num7,
        "8" => Key::Num8,
        "9" => Key::Num9,
        "space" => Key::Space,
        "enter" | "return" => Key::Enter,
        "escape" | "esc" => Key::Escape,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        _ => return false,
    };

    input.key_pressed(key)
}

/// Parse a hotkey string like "cmd+shift+v" into Modifiers and Code
/// Returns None if the string is invalid
fn parse_hotkey_string(hotkey_str: &str) -> Option<(Modifiers, Code)> {
    let hotkey_lower = hotkey_str.to_lowercase();
    let parts: Vec<&str> = hotkey_lower.split('+').collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = Modifiers::empty();
    let key_part = parts.last()?;

    for part in &parts[..parts.len() - 1] {
        match *part {
            "cmd" | "command" | "super" | "win" => modifiers |= Modifiers::SUPER,
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            _ => {}
        }
    }

    let code = match *key_part {
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "space" => Code::Space,
        "enter" | "return" => Code::Enter,
        "escape" | "esc" => Code::Escape,
        "tab" => Code::Tab,
        "backspace" => Code::Backspace,
        "delete" => Code::Delete,
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,
        _ => return None,
    };

    Some((modifiers, code))
}

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
            view_mode: ViewMode::JobList,
            popup_input: String::new(),
            autocomplete: AutocompleteState::default(),
            popup_status: None,
            diff_state: DiffState::new(),
            diff_return_view: ViewMode::JobList,
            inline_diff_content: None,
            prev_selected_job_id: None,
            apply_confirm_target: None,
            apply_confirm_return_view: ViewMode::JobList,
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
            selected_agent: None,
            agent_edit_name: String::new(),
            agent_edit_aliases: String::new(),
            agent_edit_cli_type: String::new(),
            agent_edit_mode: String::new(),
            agent_edit_system_prompt_mode: String::new(),
            agent_edit_disallowed_tools: String::new(),
            agent_edit_allowed_tools: String::new(),
            agent_edit_status: None,
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
            voice_test_status: super::settings::VoiceTestStatus::Idle,
            voice_test_result: None,
            selected_chain: None,
            chain_edit_name: String::new(),
            chain_edit_description: String::new(),
            chain_edit_states: Vec::new(),
            chain_edit_steps: Vec::new(),
            chain_edit_stop_on_failure: true,
            chain_edit_pass_full_response: true,
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
            orchestrator_requested: false,
            last_log_cleanup: std::time::Instant::now(),

            // Use pre-computed global hotkey manager
            global_hotkey_manager,
            voice_hotkey: None, // Will be set after manager is created
            global_voice_recording: false,
            global_voice_auto_paste: true,
            show_voice_overlay: false,
        }
    }

    /// Initialize the global hotkey manager and register voice hotkey
    fn init_global_hotkey_manager(hotkey_str: &str) -> Option<GlobalHotKeyManager> {
        match GlobalHotKeyManager::new() {
            Ok(manager) => {
                // Parse the configured hotkey string
                let (modifiers, code) = match parse_hotkey_string(hotkey_str) {
                    Some((m, c)) => (m, c),
                    None => {
                        tracing::warn!(
                            "Invalid hotkey string '{}', using default Cmd+Shift+V",
                            hotkey_str
                        );
                        // Fallback to default
                        #[cfg(target_os = "macos")]
                        let default_mods = Modifiers::SUPER | Modifiers::SHIFT;
                        #[cfg(not(target_os = "macos"))]
                        let default_mods = Modifiers::CONTROL | Modifiers::SHIFT;
                        (default_mods, Code::KeyV)
                    }
                };

                let hotkey = HotKey::new(Some(modifiers), code);

                if let Err(e) = manager.register(hotkey) {
                    tracing::warn!("Failed to register global voice hotkey: {}", e);
                    return Some(manager);
                }

                tracing::info!("Global voice hotkey registered: {}", hotkey_str);
                Some(manager)
            }
            Err(e) => {
                tracing::warn!("Failed to create global hotkey manager: {}", e);
                None
            }
        }
    }

    /// Handle global voice hotkey press (Cmd+Shift+V / Ctrl+Shift+V)
    ///
    /// This toggles voice recording from any application:
    /// - First press: Start recording, show overlay
    /// - Second press: Stop recording, transcribe, auto-paste to focused app
    fn handle_global_voice_hotkey(&mut self) {
        // Auto-install voice dependencies if not available
        if !self.voice_manager.is_available() && !self.voice_install_in_progress {
            self.voice_install_in_progress = true;
            self.voice_install_status =
                Some(("Installing voice dependencies...".to_string(), false));

            let model_name = self.voice_manager.config.whisper_model.clone();
            let result = super::voice::install::install_voice_dependencies(
                &self.work_dir,
                &model_name,
            );

            self.voice_install_status = Some((result.message.clone(), result.is_error));
            self.voice_install_in_progress = result.in_progress;

            if result.is_error {
                self.logs.push(LogEvent::error(format!(
                    "Voice dependencies installation failed: {}",
                    result.message
                )));
                return;
            }

            // Invalidate availability cache
            self.voice_manager.reset();
        }

        if self.voice_install_in_progress {
            return; // Still installing
        }

        match self.voice_manager.state {
            VoiceState::Idle | VoiceState::Error => {
                // Start recording
                self.voice_manager.start_recording();
                self.global_voice_recording = true;
                self.show_voice_overlay = true;
                self.logs.push(LogEvent::system(
                    "🎤 Global voice recording started (Cmd+Shift+V to stop)".to_string(),
                ));
            }
            VoiceState::Recording => {
                // Stop recording and transcribe
                self.voice_manager.stop_recording();
                // Note: global_voice_recording stays true until transcription completes
                self.logs.push(LogEvent::system(
                    "⏳ Stopping recording, transcribing...".to_string(),
                ));
            }
            VoiceState::Transcribing => {
                // Already transcribing, ignore
                self.logs.push(LogEvent::system(
                    "⏳ Already transcribing, please wait...".to_string(),
                ));
            }
            _ => {}
        }
    }

    /// Handle completed transcription for global voice input
    fn handle_global_voice_transcription(&mut self, text: &str) {
        self.global_voice_recording = false;
        self.show_voice_overlay = false;

        if self.global_voice_auto_paste {
            // Copy to clipboard and auto-paste
            match copy_and_paste(text, true) {
                Ok(()) => {
                    self.logs.push(LogEvent::system(format!(
                        "✓ Voice transcribed and pasted: \"{}\"",
                        if text.chars().count() > 50 {
                            let end = text
                                .char_indices()
                                .nth(50)
                                .map(|(i, _)| i)
                                .unwrap_or(text.len());
                            format!("{}...", &text[..end])
                        } else {
                            text.to_string()
                        }
                    )));
                }
                Err(e) => {
                    // Paste failed but text is in clipboard
                    self.logs.push(LogEvent::system(format!(
                        "Voice transcribed (use Cmd+V to paste): {}",
                        e
                    )));
                }
            }
        } else {
            // Just copy to clipboard, no auto-paste
            if let Err(e) = copy_and_paste(text, false) {
                self.logs.push(LogEvent::error(format!(
                    "Failed to copy to clipboard: {}",
                    e
                )));
            } else {
                self.logs.push(LogEvent::system(format!(
                    "✓ Voice transcribed and copied: \"{}\"",
                    if text.chars().count() > 50 {
                        let end = text
                            .char_indices()
                            .nth(50)
                            .map(|(i, _)| i)
                            .unwrap_or(text.len());
                        format!("{}...", &text[..end])
                    } else {
                        text.to_string()
                    }
                )));
            }
        }
    }

    /// Render a small voice recording overlay in the corner of the screen
    fn render_voice_overlay(&self, ctx: &egui::Context) {
        let state_text = match self.voice_manager.state {
            VoiceState::Recording => "🎤 Recording...",
            VoiceState::Transcribing => "⏳ Transcribing...",
            _ => "🎤 Voice Input",
        };

        egui::Window::new("voice_overlay")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-20.0, 20.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(if self.voice_manager.state == VoiceState::Recording {
                        Color32::from_rgb(200, 60, 60) // Red when recording
                    } else {
                        Color32::from_rgb(60, 60, 80) // Dark when transcribing
                    })
                    .corner_radius(12)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(state_text)
                            .color(Color32::WHITE)
                            .size(14.0)
                            .strong(),
                    );
                });
                if self.voice_manager.state == VoiceState::Recording {
                    ui.label(
                        egui::RichText::new("Press Cmd+Shift+V to stop")
                            .color(Color32::from_gray(200))
                            .size(11.0),
                    );
                }
            });
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Memory Management Helpers
    // ═══════════════════════════════════════════════════════════════════════

    /// Truncate global logs to MAX_GLOBAL_LOGS (FIFO eviction).
    /// Called periodically to prevent unbounded memory growth.
    fn truncate_logs(&mut self) {
        if self.logs.len() > MAX_GLOBAL_LOGS {
            let excess = self.logs.len() - MAX_GLOBAL_LOGS;
            self.logs.drain(0..excess);
        }
    }

    /// Render settings/extensions view
    fn render_settings(&mut self, ctx: &egui::Context) {
        let Ok(mut config) = self.config.write() else {
            // Lock poisoned - show error and return
            self.logs.push(LogEvent::error("Config lock poisoned, cannot render settings"));
            return;
        };
        super::settings::render_settings(
            ctx,
            &mut super::settings::SettingsState {
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
                // Workspace config import
                workspace_registry: Some(&self.workspace_registry),
                import_workspace_selected: &mut self.import_workspace_selected,
                import_modes: &mut self.import_modes,
                import_agents: &mut self.import_agents,
                import_chains: &mut self.import_chains,
                import_settings: &mut self.import_settings,
            },
        );
    }

    /// Render modes configuration view
    fn render_modes(&mut self, ctx: &egui::Context) {
        let Ok(mut config) = self.config.write() else {
            self.logs.push(LogEvent::error("Config lock poisoned, cannot render modes"));
            return;
        };
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
                mode_edit_session_mode: &mut self.mode_edit_session_mode,
                mode_edit_max_turns: &mut self.mode_edit_max_turns,
                mode_edit_model: &mut self.mode_edit_model,
                mode_edit_claude_permission: &mut self.mode_edit_claude_permission,
                mode_edit_codex_sandbox: &mut self.mode_edit_codex_sandbox,
                mode_edit_output_states: &mut self.mode_edit_output_states,
                mode_edit_state_prompt: &mut self.mode_edit_state_prompt,
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Render agents configuration view
    fn render_agents(&mut self, ctx: &egui::Context) {
        let Ok(mut config) = self.config.write() else {
            self.logs.push(LogEvent::error("Config lock poisoned, cannot render agents"));
            return;
        };
        super::agents::render_agents(
            ctx,
            &mut super::agents::AgentEditorState {
                selected_agent: &mut self.selected_agent,
                agent_edit_name: &mut self.agent_edit_name,
                agent_edit_aliases: &mut self.agent_edit_aliases,
                agent_edit_cli_type: &mut self.agent_edit_cli_type,
                agent_edit_mode: &mut self.agent_edit_mode,
                agent_edit_system_prompt_mode: &mut self.agent_edit_system_prompt_mode,
                agent_edit_disallowed_tools: &mut self.agent_edit_disallowed_tools,
                agent_edit_allowed_tools: &mut self.agent_edit_allowed_tools,
                agent_edit_status: &mut self.agent_edit_status,
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Render chains configuration view
    fn render_chains(&mut self, ctx: &egui::Context) {
        let Ok(mut config) = self.config.write() else {
            self.logs.push(LogEvent::error("Config lock poisoned, cannot render chains"));
            return;
        };
        super::chains::render_chains(
            ctx,
            &mut super::chains::ChainEditorState {
                selected_chain: &mut self.selected_chain,
                chain_edit_name: &mut self.chain_edit_name,
                chain_edit_description: &mut self.chain_edit_description,
                chain_edit_states: &mut self.chain_edit_states,
                chain_edit_steps: &mut self.chain_edit_steps,
                chain_edit_stop_on_failure: &mut self.chain_edit_stop_on_failure,
                chain_edit_pass_full_response: &mut self.chain_edit_pass_full_response,
                chain_edit_status: &mut self.chain_edit_status,
                pending_confirmation: &mut self.chain_pending_confirmation,
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
            },
        );
    }

    fn launch_orchestrator(&mut self) -> anyhow::Result<()> {
        #[cfg(not(target_os = "macos"))]
        {
            anyhow::bail!("Orchestrator launch is only supported on macOS right now.");
        }

        #[cfg(target_os = "macos")]
        {
            let kyco_dir = self.work_dir.join(".kyco");
            std::fs::create_dir_all(&kyco_dir)?;

            // Get orchestrator settings from config
            let (custom_cli, custom_prompt, default_agent) = self
                .config
                .read()
                .map(|cfg| {
                    let gui = &cfg.settings.gui;
                    (
                        gui.orchestrator.cli_command.trim().to_string(),
                        gui.orchestrator.system_prompt.trim().to_string(),
                        gui.default_agent.trim().to_lowercase(),
                    )
                })
                .unwrap_or_default();

            // Use custom prompt or fallback to built-in default
            let prompt = if custom_prompt.is_empty() {
                ORCHESTRATOR_SYSTEM_PROMPT.to_string()
            } else {
                custom_prompt
            };

            let prompt_file = kyco_dir.join("orchestrator_system_prompt.txt");
            std::fs::write(&prompt_file, &prompt)?;

            // Use custom CLI command or generate default based on agent
            let command = if !custom_cli.is_empty() {
                // Replace {prompt_file} placeholder with actual path
                custom_cli.replace("{prompt_file}", ".kyco/orchestrator_system_prompt.txt")
            } else {
                let agent = if default_agent.is_empty() {
                    "claude"
                } else {
                    default_agent.as_str()
                };
                match agent {
                    "codex" => {
                        "codex \"$(cat .kyco/orchestrator_system_prompt.txt)\"".to_string()
                    }
                    _ => {
                        "claude --append-system-prompt \"$(cat .kyco/orchestrator_system_prompt.txt)\""
                            .to_string()
                    }
                }
            };

            let session_id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let args = vec!["-lc".to_string(), command.clone()];
            TerminalSession::spawn(session_id, "bash", &args, "", &self.work_dir)?;

            self.logs.push(LogEvent::system(format!(
                "Orchestrator started in Terminal.app ({})",
                if custom_cli.is_empty() {
                    if default_agent.is_empty() { "claude" } else { &default_agent }
                } else {
                    "custom"
                }
            )));
            Ok(())
        }
    }

    /// Refresh cached jobs from JobManager (only if changed)
    fn refresh_jobs(&mut self) {
        // Only refresh if jobs have changed (generation counter check)
        if let Some(new_generation) =
            jobs::check_jobs_changed(&self.job_manager, self.last_job_generation)
        {
            let (new_jobs, generation) = jobs::refresh_jobs(&self.job_manager);
            self.cached_jobs = new_jobs;
            self.last_job_generation = generation;
            tracing::trace!(
                "Jobs refreshed, generation {} -> {}",
                self.last_job_generation,
                new_generation
            );
        }
    }

    fn open_apply_confirm(&mut self, target: ApplyTarget) {
        self.apply_confirm_target = Some(target);
        self.apply_confirm_return_view = self.view_mode;
        self.apply_confirm_error = None;
        self.apply_confirm_rx = None;
        self.view_mode = ViewMode::ApplyConfirmPopup;
    }

    fn workspace_root_for_job(&self, job: &Job) -> PathBuf {
        job.workspace_path
            .clone()
            .unwrap_or_else(|| self.work_dir.clone())
    }

    fn open_job_diff(&mut self, job_id: JobId, return_view: ViewMode) {
        let Some(job) = self.cached_jobs.iter().find(|j| j.id == job_id).cloned() else {
            self.logs
                .push(LogEvent::error(format!("Job #{} not found", job_id)));
            return;
        };

        let workspace_root = self.workspace_root_for_job(&job);
        let gm = match crate::git::GitManager::new(&workspace_root) {
            Ok(gm) => gm,
            Err(e) => {
                self.logs.push(LogEvent::error(format!(
                    "Failed to initialize git manager for {}: {}",
                    workspace_root.display(),
                    e
                )));
                return;
            }
        };

        let diff_result =
            if let Some(worktree) = job.git_worktree_path.as_ref().filter(|p| p.exists()) {
                gm.diff(worktree, job.base_branch.as_deref())
                    .map(|mut diff| {
                        if let Ok(untracked) = gm.untracked_files(worktree) {
                            if !untracked.is_empty() {
                                if !diff.is_empty() {
                                    diff.push_str("\n\n");
                                }
                                diff.push_str("--- Untracked files ---\n");
                                for file in untracked {
                                    diff.push_str(&file.display().to_string());
                                    diff.push('\n');
                                }
                            }
                        }
                        diff
                    })
            } else {
                gm.diff(&workspace_root, None).map(|mut diff| {
                    let mut header = "--- Workspace changes (no worktree) ---\n\n".to_string();
                    if let Ok(untracked) = gm.untracked_files(&workspace_root) {
                        if !untracked.is_empty() {
                            if !diff.is_empty() {
                                diff.push_str("\n\n");
                            }
                            diff.push_str("--- Untracked files ---\n");
                            for file in untracked {
                                diff.push_str(&file.display().to_string());
                                diff.push('\n');
                            }
                        }
                    }

                    if diff.is_empty() {
                        header.push_str("No changes in workspace.");
                        header
                    } else {
                        format!("{}{}", header, diff)
                    }
                })
            };

        match diff_result {
            Ok(content) => {
                self.diff_state.set_content(content);
                self.diff_return_view = return_view;
                self.view_mode = ViewMode::DiffView;
            }
            Err(e) => {
                self.logs
                    .push(LogEvent::error(format!("Failed to load diff: {}", e)));
            }
        }
    }

    /// Load inline diff for the currently selected job (for detail panel display)
    fn load_inline_diff_for_selected(&mut self) {
        let Some(job_id) = self.selected_job_id else {
            self.inline_diff_content = None;
            return;
        };

        let Some(job) = self.cached_jobs.iter().find(|j| j.id == job_id).cloned() else {
            self.inline_diff_content = None;
            return;
        };

        // Only load diff for completed jobs with changes
        if job.status != crate::JobStatus::Done {
            self.inline_diff_content = None;
            return;
        }

        let workspace_root = self.workspace_root_for_job(&job);
        let gm = match crate::git::GitManager::new(&workspace_root) {
            Ok(gm) => gm,
            Err(_) => {
                self.inline_diff_content = None;
                return;
            }
        };

        let diff_result: Option<String> =
            if let Some(worktree) = job.git_worktree_path.as_ref().filter(|p| p.exists()) {
                gm.diff(worktree, job.base_branch.as_deref())
                    .ok()
                    .map(|mut diff| {
                        if let Ok(untracked) = gm.untracked_files(worktree) {
                            if !untracked.is_empty() {
                                if !diff.is_empty() {
                                    diff.push_str("\n\n");
                                }
                                diff.push_str("--- Untracked files ---\n");
                                for file in untracked {
                                    diff.push_str(&file.display().to_string());
                                    diff.push('\n');
                                }
                            }
                        }
                        diff
                    })
            } else {
                // No worktree - show workspace diff
                gm.diff(&workspace_root, None).ok()
            };

        self.inline_diff_content = diff_result.filter(|d| !d.is_empty());
    }

    fn build_apply_thread_input(&self, target: &ApplyTarget) -> Result<ApplyThreadInput, String> {
        match target {
            ApplyTarget::Single { job_id } => {
                let job = self
                    .job_manager
                    .lock()
                    .map_err(|_| "Failed to lock job manager".to_string())?
                    .get(*job_id)
                    .cloned()
                    .ok_or_else(|| format!("Job #{} not found", job_id))?;

                let workspace_root = self.workspace_root_for_job(&job);
                Ok(ApplyThreadInput::Single(SingleApplyInput {
                    job_id: *job_id,
                    workspace_root,
                    worktree_path: job.git_worktree_path.clone(),
                    base_branch: job.base_branch.clone(),
                    commit_message: crate::git::CommitMessage::from_job(&job),
                }))
            }
            ApplyTarget::Group {
                group_id,
                selected_job_id,
            } => {
                let group = self
                    .group_manager
                    .lock()
                    .map_err(|_| "Failed to lock group manager".to_string())?
                    .get(*group_id)
                    .cloned()
                    .ok_or_else(|| format!("Group #{} not found", group_id))?;

                if !matches!(
                    group.status,
                    crate::GroupStatus::Comparing | crate::GroupStatus::Selected
                ) {
                    return Err(format!(
                        "Group #{} is not ready to merge yet (status: {})",
                        group_id, group.status
                    ));
                }

                if !group.job_ids.contains(selected_job_id) {
                    return Err(format!(
                        "Selected job #{} is not part of group #{}",
                        selected_job_id, group_id
                    ));
                }

                let manager = self
                    .job_manager
                    .lock()
                    .map_err(|_| "Failed to lock job manager".to_string())?;

                let selected_job = manager
                    .get(*selected_job_id)
                    .cloned()
                    .ok_or_else(|| format!("Selected job #{} not found", selected_job_id))?;

                let selected_worktree_path = selected_job
                    .git_worktree_path
                    .clone()
                    .ok_or_else(|| "Selected job has no worktree".to_string())?;

                let base_branch = selected_job
                    .base_branch
                    .clone()
                    .ok_or_else(|| "Selected job has no base branch recorded".to_string())?;

                let cleanup_worktrees: Vec<(JobId, PathBuf)> = group
                    .job_ids
                    .iter()
                    .filter_map(|&job_id| {
                        manager
                            .get(job_id)
                            .and_then(|j| j.git_worktree_path.clone().map(|p| (job_id, p)))
                    })
                    .collect();

                let workspace_root = self.workspace_root_for_job(&selected_job);
                Ok(ApplyThreadInput::Group(GroupApplyInput {
                    group_id: *group_id,
                    selected_job_id: *selected_job_id,
                    selected_agent_id: selected_job.agent_id.clone(),
                    workspace_root,
                    selected_worktree_path,
                    base_branch,
                    commit_message: crate::git::CommitMessage::from_job(&selected_job),
                    cleanup_worktrees,
                    group_job_ids: group.job_ids.clone(),
                }))
            }
        }
    }

    fn start_apply_confirm_merge(&mut self) {
        if self.apply_confirm_rx.is_some() {
            return;
        }

        let Some(target) = self.apply_confirm_target.clone() else {
            self.apply_confirm_error = Some("No merge target selected".to_string());
            return;
        };

        let input = match self.build_apply_thread_input(&target) {
            Ok(input) => input,
            Err(e) => {
                self.apply_confirm_error = Some(e);
                return;
            }
        };

        self.apply_confirm_error = None;
        let (tx, rx) = std::sync::mpsc::channel();
        self.apply_confirm_rx = Some(rx);

        std::thread::spawn(move || {
            let result = run_apply_thread(input);
            let _ = tx.send(result);
        });
    }

    /// Queue a job for execution
    fn queue_job(&mut self, job_id: JobId) {
        jobs::queue_job(&self.job_manager, job_id, &mut self.logs);
        self.refresh_jobs();
    }

    /// Apply job changes (merge worktree to main)
    fn apply_job(&mut self, job_id: JobId) {
        let job = match self.job_manager.lock() {
            Ok(manager) => manager.get(job_id).cloned(),
            Err(_) => None,
        };

        let Some(job) = job else {
            self.logs
                .push(LogEvent::error(format!("Job #{} not found", job_id)));
            return;
        };

        let target = if let Some(group_id) = job.group_id {
            ApplyTarget::Group {
                group_id,
                selected_job_id: job_id,
            }
        } else {
            ApplyTarget::Single { job_id }
        };

        self.open_apply_confirm(target);
    }

    /// Reject job changes
    fn reject_job(&mut self, job_id: JobId) {
        let job = match self.job_manager.lock() {
            Ok(manager) => manager.get(job_id).cloned(),
            Err(_) => None,
        };

        let Some(job) = job else {
            self.logs
                .push(LogEvent::error(format!("Job #{} not found", job_id)));
            return;
        };

        if let Some(worktree) = job.git_worktree_path.clone() {
            let workspace_root = self.workspace_root_for_job(&job);
            if let Ok(git) = crate::git::GitManager::new(&workspace_root) {
                if let Err(e) = git.remove_worktree_by_path(&worktree) {
                    self.logs.push(LogEvent::error(format!(
                        "Failed to remove worktree for rejected job: {}",
                        e
                    )));
                }
            } else {
                self.logs.push(LogEvent::error(format!(
                    "Failed to initialize git manager for {}",
                    workspace_root.display()
                )));
            }
        } else {
            self.logs.push(LogEvent::system(
                "Rejected job without worktree (no changes were reverted)".to_string(),
            ));
        }

        if let Ok(mut manager) = self.job_manager.lock() {
            if let Some(j) = manager.get_mut(job_id) {
                j.set_status(crate::JobStatus::Rejected);
                j.git_worktree_path = None;
                j.branch_name = None;
            }
        }
        self.logs
            .push(LogEvent::system(format!("Rejected job #{}", job_id)));
        self.refresh_jobs();
    }

    /// Kill/stop a running job
    fn kill_job(&mut self, job_id: JobId) {
        let (agent_id, job_mode, session_id) = {
            let manager = match self.job_manager.lock() {
                Ok(m) => m,
                Err(_) => {
                    self.logs
                        .push(LogEvent::error("Failed to lock job manager"));
                    return;
                }
            };

            match manager.get(job_id) {
                Some(job) => (
                    job.agent_id.clone(),
                    job.mode.clone(),
                    job.bridge_session_id.clone(),
                ),
                None => {
                    self.logs
                        .push(LogEvent::error(format!("Job #{} not found", job_id)));
                    return;
                }
            }
        };

        if let Some(session_id) = session_id.as_deref() {
            let sdk_type = self
                .config
                .read()
                .ok()
                .and_then(|cfg| cfg.get_agent_for_job(&agent_id, &job_mode))
                .map(|a| a.sdk_type)
                .unwrap_or_else(|| {
                    if agent_id == "codex" {
                        SdkType::Codex
                    } else {
                        SdkType::Claude
                    }
                });

            let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if sdk_type == SdkType::Codex {
                    self.bridge_client.interrupt_codex(session_id)
                } else {
                    self.bridge_client.interrupt_claude(session_id)
                }
            }));

            match interrupted {
                Ok(Ok(true)) => self.logs.push(LogEvent::system(format!(
                    "Sent interrupt for job #{}",
                    job_id
                ))),
                Ok(Ok(false)) => self.logs.push(LogEvent::error(format!(
                    "Interrupt was rejected (job #{})",
                    job_id
                ))),
                Ok(Err(e)) => self.logs.push(LogEvent::error(format!(
                    "Failed to interrupt job #{}: {}",
                    job_id, e
                ))),
                Err(_) => self.logs.push(LogEvent::error(format!(
                    "Bridge interrupt panicked (job #{})",
                    job_id
                ))),
            };
        }

        jobs::kill_job(&self.job_manager, job_id, &mut self.logs);
        self.refresh_jobs();
    }

    fn set_job_permission_mode(&mut self, job_id: JobId, mode: PermissionMode) {
        let (agent_id, job_mode, session_id) = {
            let manager = match self.job_manager.lock() {
                Ok(m) => m,
                Err(_) => {
                    self.logs
                        .push(LogEvent::error("Failed to lock job manager"));
                    return;
                }
            };

            match manager.get(job_id) {
                Some(job) => (
                    job.agent_id.clone(),
                    job.mode.clone(),
                    job.bridge_session_id.clone(),
                ),
                None => {
                    self.logs
                        .push(LogEvent::error(format!("Job #{} not found", job_id)));
                    return;
                }
            }
        };

        let is_codex = {
            let Ok(config) = self.config.read() else {
                self.logs.push(LogEvent::error("Config lock poisoned"));
                return;
            };
            config
                .get_agent_for_job(&agent_id, &job_mode)
                .map(|a| a.sdk_type == SdkType::Codex)
                .unwrap_or(agent_id == "codex")
        };

        if is_codex {
            self.logs.push(LogEvent::error(format!(
                "Permission mode switching is only supported for Claude sessions (job #{})",
                job_id
            )));
            return;
        }

        let Some(session_id) = session_id else {
            self.logs.push(LogEvent::error(format!(
                "Job #{} has no active Claude session yet",
                job_id
            )));
            return;
        };

        match self
            .bridge_client
            .set_claude_permission_mode(&session_id, mode)
        {
            Ok(true) => {
                self.permission_mode_overrides.insert(job_id, mode);
                self.logs.push(LogEvent::system(format!(
                    "Set permission mode to {} for job #{}",
                    match mode {
                        PermissionMode::Default => "default",
                        PermissionMode::AcceptEdits => "acceptEdits",
                        PermissionMode::BypassPermissions => "bypassPermissions",
                        PermissionMode::Plan => "plan",
                    },
                    job_id
                )));
            }
            Ok(false) => {
                self.logs.push(LogEvent::error(format!(
                    "Failed to set permission mode for job #{} (bridge rejected request)",
                    job_id
                )));
            }
            Err(e) => {
                self.logs.push(LogEvent::error(format!(
                    "Failed to set permission mode for job #{}: {}",
                    job_id, e
                )));
            }
        }
    }

    /// Mark a REPL job as complete
    fn mark_job_complete(&mut self, job_id: JobId) {
        jobs::mark_job_complete(&self.job_manager, job_id, &mut self.logs);
        self.refresh_jobs();
    }

    fn continue_job_session(&mut self, job_id: JobId, prompt: String) {
        let (continuation_id, continuation_mode) = {
            let mut manager = match self.job_manager.lock() {
                Ok(m) => m,
                Err(_) => {
                    self.logs
                        .push(LogEvent::error("Failed to lock job manager"));
                    return;
                }
            };

            let Some(original) = manager.get(job_id).cloned() else {
                self.logs
                    .push(LogEvent::error(format!("Job #{} not found", job_id)));
                return;
            };

            let Some(session_id) = original.bridge_session_id.clone() else {
                self.logs.push(LogEvent::error(format!(
                    "Job #{} has no session to continue",
                    job_id
                )));
                return;
            };

            let tag = crate::CommentTag {
                file_path: original.source_file.clone(),
                line_number: original.source_line,
                raw_line: format!("// @{}:{} {}", &original.agent_id, &original.mode, &prompt),
                agent: original.agent_id.clone(),
                agents: vec![original.agent_id.clone()],
                mode: original.mode.clone(),
                target: crate::Target::Block,
                status_marker: None,
                description: Some(prompt),
                job_id: None,
            };

            let continuation_id =
                match manager.create_job_with_range(&tag, &original.agent_id, None) {
                    Ok(id) => id,
                    Err(e) => {
                        self.logs.push(LogEvent::error(format!(
                            "Failed to create continuation job: {}",
                            e
                        )));
                        return;
                    }
                };

            if let Some(job) = manager.get_mut(continuation_id) {
                job.raw_tag_line = None;
                job.bridge_session_id = Some(session_id);

                // Reuse the same worktree and job context
                job.git_worktree_path = original.git_worktree_path.clone();
                job.branch_name = original.branch_name.clone();
                job.base_branch = original.base_branch.clone();
                job.scope = original.scope.clone();
                job.target = original.target;
                job.ide_context = original.ide_context;
                job.force_worktree = original.force_worktree;
                job.workspace_id = original.workspace_id;
                job.workspace_path = original.workspace_path.clone();
            }

            (continuation_id, original.mode)
        };

        self.logs.push(LogEvent::system(format!(
            "Created continuation job #{} (mode: {})",
            continuation_id, continuation_mode
        )));

        self.queue_job(continuation_id);
        self.selected_job_id = Some(continuation_id);
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
    #[allow(dead_code)]
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
            "[kyco:gui] Received selection: file={:?}, lines={:?}-{:?}, deps={:?}, tests={:?}, project_root={:?}, git_root={:?}, workspace={:?}",
            req.file_path,
            req.line_start,
            req.line_end,
            req.dependency_count,
            req.related_tests.as_ref().map(|t| t.len()),
            req.project_root,
            req.git_root,
            req.workspace
        );

        // Auto-register workspace from IDE request
        // Priority: project_root (includes git detection) > workspace > active workspace
        let effective_path = req
            .project_root
            .as_ref()
            .or(req.git_root.as_ref())
            .or(req.workspace.as_ref());

        let (workspace_id, workspace_path) = if let Some(ref ws_path) = effective_path {
            let ws_path_buf = PathBuf::from(ws_path);
            if let Ok(mut registry) = self.workspace_registry.lock() {
                let ws_id = registry.get_or_create(ws_path_buf.clone());
                // Switch to this workspace and update active
                registry.set_active(ws_id);
                self.active_workspace_id = Some(ws_id);
                // Save registry to persist the new workspace
                if let Err(e) = registry.save_default() {
                    self.logs.push(LogEvent::error(format!(
                        "Failed to save workspace registry: {}",
                        e
                    )));
                }
                (Some(ws_id), Some(ws_path_buf))
            } else {
                (None, Some(ws_path_buf))
            }
        } else {
            // Use currently active workspace if no workspace specified
            (
                self.active_workspace_id,
                self.active_workspace_id.and_then(|id| {
                    self.workspace_registry
                        .lock()
                        .ok()
                        .and_then(|r| r.get(id).map(|w| w.path.clone()))
                }),
            )
        };

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
            diagnostics: req.diagnostics,
            workspace_id,
            workspace_path,
        };

        // Show selection popup
        self.view_mode = ViewMode::SelectionPopup;
        self.popup_input.clear();
        self.popup_status = None;
        self.update_suggestions();

        // Bring window to front
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    /// Handle incoming batch request from IDE extension
    fn on_batch_received(&mut self, req: BatchRequest, ctx: &egui::Context) {
        info!("[kyco:gui] Received batch: {} files", req.files.len(),);

        if req.files.is_empty() {
            self.logs
                .push(LogEvent::error("Batch request has no files".to_string()));
            return;
        }

        // Store batch files and open popup for mode/agent/prompt selection
        self.batch_files = req.files;
        self.view_mode = ViewMode::BatchPopup;
        self.popup_input.clear();
        self.popup_status = None;
        self.update_suggestions();

        self.logs.push(LogEvent::system(format!(
            "Batch: {} files selected, waiting for mode/prompt",
            self.batch_files.len()
        )));

        // Bring window to front
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    /// Execute batch task from batch popup
    /// Creates jobs for all batch files with the selected mode/agents/prompt
    fn execute_batch_task(&mut self, force_worktree: bool) {
        // Parse input same as single selection
        let (agents, mode, prompt) = parse_input_multi(&self.popup_input);

        if mode.is_empty() {
            self.popup_status = Some((
                "Please enter a mode (e.g., 'refactor', 'fix')".to_string(),
                true,
            ));
            return;
        }

        if self.batch_files.is_empty() {
            self.popup_status = Some(("No files in batch".to_string(), true));
            return;
        }

        // Resolve agent aliases
        let resolved_agents: Vec<String> = {
            let Ok(config) = self.config.read() else {
                self.logs.push(LogEvent::error("Config lock poisoned"));
                return;
            };
            agents
                .iter()
                .map(|a| {
                    config
                        .agent
                        .iter()
                        .find(|(name, cfg)| {
                            name.eq_ignore_ascii_case(a)
                                || cfg
                                    .aliases
                                    .iter()
                                    .any(|alias| alias.eq_ignore_ascii_case(a))
                        })
                        .map(|(name, _)| name.clone())
                        .unwrap_or_else(|| a.clone())
                })
                .collect()
        };

        self.logs.push(LogEvent::system(format!(
            "Starting batch: {} files with agents {:?}, mode '{}'",
            self.batch_files.len(),
            resolved_agents,
            mode
        )));

        let mut total_jobs = 0;
        let mut total_groups = 0;

        // Create jobs for each file
        for file in &self.batch_files {
            // Extract workspace from batch file
            // Priority: project_root (includes git detection) > git_root > workspace
            let effective_path = file
                .project_root
                .as_ref()
                .or(file.git_root.as_ref())
                .map(|s| s.as_str())
                .unwrap_or(&file.workspace);
            let ws_path_buf = PathBuf::from(effective_path);
            let (workspace_id, workspace_path) =
                if let Ok(mut registry) = self.workspace_registry.lock() {
                    let ws_id = registry.get_or_create(ws_path_buf.clone());
                    (Some(ws_id), Some(ws_path_buf))
                } else {
                    (None, Some(ws_path_buf))
                };

            // Create SelectionContext for this file
            let selection = SelectionContext {
                app_name: Some("IDE Batch".to_string()),
                file_path: Some(file.path.clone()),
                selected_text: None,
                line_number: file.line_start,
                line_end: file.line_end,
                possible_files: Vec::new(),
                dependencies: None,
                dependency_count: None,
                additional_dependency_count: None,
                related_tests: None,
                diagnostics: None, // Batch files don't have diagnostics
                workspace_id,
                workspace_path,
            };

            // Create job(s) for this file
            if let Some(result) = jobs::create_jobs_from_selection_multi(
                &self.job_manager,
                &self.group_manager,
                &selection,
                &resolved_agents,
                &mode,
                &prompt,
                &mut self.logs,
                force_worktree,
            ) {
                total_jobs += result.job_ids.len();
                if result.group_id.is_some() {
                    total_groups += 1;
                }
            }
        }

        self.popup_status = Some((
            format!(
                "Batch complete: {} jobs created{}",
                total_jobs,
                if total_groups > 0 {
                    format!(" in {} groups", total_groups)
                } else {
                    String::new()
                }
            ),
            false,
        ));

        // Clear batch files and return to job list
        self.batch_files.clear();
        self.refresh_jobs();
        self.view_mode = ViewMode::JobList;
    }

    /// Update autocomplete suggestions based on input
    fn update_suggestions(&mut self) {
        let Ok(config) = self.config.read() else {
            return; // Skip autocomplete if lock poisoned
        };
        self.autocomplete
            .update_suggestions(&self.popup_input, &config);
    }

    /// Apply selected suggestion
    fn apply_suggestion(&mut self) {
        if let Some(new_input) = self.autocomplete.apply_suggestion(&self.popup_input) {
            self.popup_input = new_input;
        }
    }

    /// Execute the task from selection popup
    /// If force_worktree is true, the job will run in a git worktree regardless of global settings
    fn execute_popup_task(&mut self, force_worktree: bool) {
        // Use the multi-agent parser to support "claude+codex:mode" syntax
        let (agents, mode, prompt) = parse_input_multi(&self.popup_input);

        if mode.is_empty() {
            self.popup_status = Some((
                "Please enter a mode (e.g., 'refactor', 'fix')".to_string(),
                true,
            ));
            return;
        }

        // Resolve agent aliases
        let resolved_agents: Vec<String> = {
            let Ok(config) = self.config.read() else {
                self.logs.push(LogEvent::error("Config lock poisoned"));
                return;
            };
            agents
                .iter()
                .map(|a| {
                    config
                        .agent
                        .iter()
                        .find(|(name, cfg)| {
                            name.eq_ignore_ascii_case(a)
                                || cfg
                                    .aliases
                                    .iter()
                                    .any(|alias| alias.eq_ignore_ascii_case(a))
                        })
                        .map(|(name, _)| name.clone())
                        .unwrap_or_else(|| a.clone())
                })
                .collect()
        };

        // Remove duplicates and map legacy agents.
        let mut seen = std::collections::HashSet::new();
        let resolved_agents: Vec<String> = resolved_agents
            .into_iter()
            .map(|a| match a.as_str() {
                "g" | "gm" | "gemini" | "custom" => "claude".to_string(),
                _ => a,
            })
            .filter(|a| seen.insert(a.clone()))
            .collect();

        let resolved_agents = if resolved_agents.is_empty() {
            vec!["claude".to_string()]
        } else {
            resolved_agents
        };

        // Create job(s) - uses multi-agent creation for parallel execution
        if let Some(result) = jobs::create_jobs_from_selection_multi(
            &self.job_manager,
            &self.group_manager,
            &self.selection,
            &resolved_agents,
            &mode,
            &prompt,
            &mut self.logs,
            force_worktree,
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
                    format!(
                        "Job #{} created: {}:{} ({})",
                        job_id, resolved_agents[0], mode, selection_info
                    ),
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
        let action = jobs::render_job_list(
            ui,
            &self.cached_jobs,
            &mut self.selected_job_id,
            &mut self.job_list_filter,
        );

        // Handle actions
        match action {
            jobs::JobListAction::DeleteJob(job_id) => {
                self.delete_job(job_id);
            }
            jobs::JobListAction::DeleteAllFinished => {
                self.delete_all_finished_jobs();
            }
            jobs::JobListAction::None => {}
        }
    }

    /// Delete a job from the job manager
    fn delete_job(&mut self, job_id: JobId) {
        if let Ok(mut manager) = self.job_manager.lock() {
            if let Some(job) = manager.remove_job(job_id) {
                self.logs.push(LogEvent::system(format!(
                    "Deleted job #{} ({})",
                    job_id, job.mode
                )));

                // Clear selection if deleted job was selected
                if self.selected_job_id == Some(job_id) {
                    self.selected_job_id = None;
                }
            }
        }

        // Also remove from group manager
        if let Ok(mut gm) = self.group_manager.lock() {
            gm.remove_job(job_id);
        }

        // Cleanup per-job UI state
        self.permission_mode_overrides.remove(&job_id);

        // Refresh to update UI
        self.refresh_jobs();
    }

    /// Delete all finished jobs (Done, Failed, Rejected, Merged)
    fn delete_all_finished_jobs(&mut self) {
        // Collect IDs of finished jobs
        let finished_ids: Vec<JobId> = self
            .cached_jobs
            .iter()
            .filter(|j| j.is_finished())
            .map(|j| j.id)
            .collect();

        if finished_ids.is_empty() {
            return;
        }

        let count = finished_ids.len();

        // Remove from job manager
        if let Ok(mut manager) = self.job_manager.lock() {
            for job_id in &finished_ids {
                manager.remove_job(*job_id);
            }
        }

        // Remove from group manager
        if let Ok(mut gm) = self.group_manager.lock() {
            for job_id in &finished_ids {
                gm.remove_job(*job_id);
            }
        }

        // Cleanup per-job UI state
        for job_id in &finished_ids {
            self.permission_mode_overrides.remove(job_id);
        }

        // Clear selection if deleted job was selected
        if let Some(selected) = self.selected_job_id {
            if finished_ids.contains(&selected) {
                self.selected_job_id = None;
            }
        }

        self.logs
            .push(LogEvent::system(format!("Deleted {} finished jobs", count)));

        // Refresh to update UI
        self.refresh_jobs();
    }

    /// Render the detail panel
    fn render_detail_panel(&mut self, ui: &mut egui::Ui) {
        use super::detail_panel::{DetailPanelAction, DetailPanelState, render_detail_panel};

        let action = {
            let Ok(config) = self.config.read() else {
                ui.label("Config unavailable");
                return;
            };
            let mut state = DetailPanelState {
                selected_job_id: self.selected_job_id,
                cached_jobs: &self.cached_jobs,
                logs: &self.logs,
                config: &config,
                log_scroll_to_bottom: self.log_scroll_to_bottom,
                activity_log_filters: &mut self.activity_log_filters,
                continuation_prompt: &mut self.continuation_prompt,
                commonmark_cache: &mut self.commonmark_cache,
                permission_mode_overrides: &self.permission_mode_overrides,
                diff_content: self.inline_diff_content.as_deref(),
            };

            render_detail_panel(ui, &mut state)
        };

        if let Some(action) = action {
            match action {
                DetailPanelAction::Queue(job_id) => self.queue_job(job_id),
                DetailPanelAction::Apply(job_id) => self.apply_job(job_id),
                DetailPanelAction::Reject(job_id) => self.reject_job(job_id),
                DetailPanelAction::CompareGroup(group_id) => self.open_comparison_popup(group_id),
                DetailPanelAction::Continue(job_id, prompt) => {
                    self.continue_job_session(job_id, prompt);
                }
                DetailPanelAction::ViewDiff(job_id) => {
                    self.open_job_diff(job_id, ViewMode::JobList)
                }
                DetailPanelAction::Kill(job_id) => self.kill_job(job_id),
                DetailPanelAction::MarkComplete(job_id) => self.mark_job_complete(job_id),
                DetailPanelAction::SetPermissionMode(job_id, mode) => {
                    self.set_job_permission_mode(job_id, mode);
                }
            }
        }
    }

    /// Render the selection popup
    fn render_selection_popup(&mut self, ctx: &egui::Context) {
        use super::selection::{SelectionPopupAction, SelectionPopupState, render_selection_popup};

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
                    // Auto-install voice dependencies if not available
                    if !self.voice_manager.is_available() && !self.voice_install_in_progress {
                        self.voice_install_in_progress = true;
                        self.voice_install_status =
                            Some(("Installing voice dependencies...".to_string(), false));

                        let model_name = &self.voice_manager.config.whisper_model;
                        let result = crate::gui::voice::install::install_voice_dependencies(
                            &self.work_dir,
                            model_name,
                        );

                        self.voice_install_status = Some((result.message.clone(), result.is_error));
                        self.voice_install_in_progress = result.in_progress;

                        // Invalidate availability cache so next check sees the new installation
                        if !result.is_error {
                            self.voice_manager.reset();
                        }
                    } else if !self.voice_install_in_progress {
                        self.voice_manager.toggle_recording();
                    }
                }
            }
        }
    }

    /// Render the batch popup (similar to selection popup but for multiple files)
    fn render_batch_popup(&mut self, ctx: &egui::Context) {
        use super::selection::{BatchPopupState, SelectionPopupAction, render_batch_popup};

        let mut state = BatchPopupState {
            batch_files: &self.batch_files,
            popup_input: &mut self.popup_input,
            popup_status: &self.popup_status,
            suggestions: &self.autocomplete.suggestions,
            selected_suggestion: self.autocomplete.selected_suggestion,
            show_suggestions: self.autocomplete.show_suggestions,
            cursor_to_end: &mut self.autocomplete.cursor_to_end,
        };

        if let Some(action) = render_batch_popup(ctx, &mut state) {
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
                    // No voice in batch popup
                }
            }
        }
    }

    /// Render the diff view popup
    fn render_diff_popup(&mut self, ctx: &egui::Context) {
        if super::diff::render_diff_popup(ctx, &self.diff_state) {
            self.view_mode = self.diff_return_view;
            self.diff_state.clear();
        }
    }

    fn render_apply_confirm_popup(&mut self, ctx: &egui::Context) {
        use eframe::egui::{RichText, Vec2};

        let Some(target) = self.apply_confirm_target.clone() else {
            self.view_mode = self.apply_confirm_return_view;
            return;
        };

        let in_progress = self.apply_confirm_rx.is_some();
        let validation_error = self.build_apply_thread_input(&target).err();

        let title = match &target {
            ApplyTarget::Single { job_id } => format!("Merge Job #{}", job_id),
            ApplyTarget::Group { group_id, .. } => format!("Merge Group #{}", group_id),
        };
        let mut description_lines: Vec<String> = Vec::new();
        let mut selected_job_id_for_diff: Option<JobId> = None;
        let mut warning: Option<String> = None;

        match &target {
            ApplyTarget::Single { job_id } => {
                let job = self.cached_jobs.iter().find(|j| j.id == *job_id);
                if let Some(job) = job {
                    selected_job_id_for_diff = Some(job.id);
                    let workspace_root = self.workspace_root_for_job(job);
                    description_lines.push(format!("Repo: {}", workspace_root.display()));
                    description_lines.push(format!("Agent: {}", job.agent_id));
                    description_lines.push(format!("Mode: {}", job.mode));
                    description_lines.push(format!("Target: {}", job.target));

                    let subject = crate::git::CommitMessage::from_job(job).subject;
                    description_lines.push(format!("Commit: {}", subject));

                    if let Some(worktree) = &job.git_worktree_path {
                        description_lines.push(format!("Worktree: {}", worktree.display()));
                        let base = job.base_branch.as_deref().unwrap_or("<unknown>");
                        description_lines.push(format!("Merge into: {}", base));
                    } else {
                        warning = Some(
                            "No worktree: this will commit ALL current changes in the repo."
                                .to_string(),
                        );
                    }
                } else {
                    warning = Some("Job not found".to_string());
                }
            }
            ApplyTarget::Group {
                group_id,
                selected_job_id,
            } => {
                selected_job_id_for_diff = Some(*selected_job_id);
                let group = self
                    .group_manager
                    .lock()
                    .ok()
                    .and_then(|gm| gm.get(*group_id).cloned());
                if let Some(group) = group {
                    description_lines.push(format!("Group status: {}", group.status));
                    description_lines.push(format!("Mode: {}", group.mode));
                    description_lines.push(format!("Target: {}", group.target));
                }

                let job = self.cached_jobs.iter().find(|j| j.id == *selected_job_id);
                if let Some(job) = job {
                    let workspace_root = self.workspace_root_for_job(job);
                    description_lines.push(format!("Repo: {}", workspace_root.display()));
                    description_lines
                        .push(format!("Selected result: #{} ({})", job.id, job.agent_id));

                    let subject = crate::git::CommitMessage::from_job(job).subject;
                    description_lines.push(format!("Commit: {}", subject));

                    if let Some(worktree) = &job.git_worktree_path {
                        description_lines.push(format!("Worktree: {}", worktree.display()));
                    }
                    if let Some(base) = job.base_branch.as_deref() {
                        description_lines.push(format!("Merge into: {}", base));
                    }

                    warning = Some(
                        "Other group jobs will be marked Rejected and all group worktrees will be deleted."
                            .to_string(),
                    );
                } else {
                    warning = Some("Selected job not found".to_string());
                }
            }
        }

        egui::Window::new("Merge Confirmation")
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(620.0, 360.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(
                egui::Frame::default()
                    .fill(BG_PRIMARY)
                    .stroke(Stroke::new(2.0, ACCENT_CYAN))
                    .inner_margin(16.0)
                    .corner_radius(8.0),
            )
            .show(ctx, |ui| {
                ui.label(RichText::new(title).size(18.0).strong().color(TEXT_PRIMARY));
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                for line in &description_lines {
                    ui.label(RichText::new(line).color(TEXT_DIM));
                }

                if let Some(w) = &warning {
                    ui.add_space(8.0);
                    ui.label(RichText::new(w).color(ACCENT_RED));
                }

                if let Some(err) = &validation_error {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Cannot merge yet: {}", err))
                            .color(ACCENT_RED),
                    );
                }

                if let Some(err) = &self.apply_confirm_error {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("Error: {}", err)).color(ACCENT_RED));
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(12.0);

                if in_progress {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Merging...").color(TEXT_DIM));
                    });
                } else {
                    ui.label(
                        RichText::new("Tip: If a merge conflict occurs, the merge is aborted to keep your repo clean.")
                            .small()
                            .color(TEXT_MUTED),
                    );
                }

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_merge = !in_progress && validation_error.is_none();
                        let merge_btn = egui::Button::new(
                            RichText::new("✓ Merge")
                                .color(if can_merge { BG_PRIMARY } else { TEXT_MUTED }),
                        )
                        .fill(if can_merge { ACCENT_GREEN } else { BG_SECONDARY });

                        if ui.add_enabled(can_merge, merge_btn).clicked() {
                            self.start_apply_confirm_merge();
                        }

                        ui.add_space(8.0);

                        if ui
                            .add_enabled(
                                !in_progress,
                                egui::Button::new(RichText::new("View Diff").color(TEXT_DIM)),
                            )
                            .clicked()
                        {
                            if let Some(job_id) = selected_job_id_for_diff {
                                self.open_job_diff(job_id, ViewMode::ApplyConfirmPopup);
                            }
                        }

                        ui.add_space(8.0);

                        if ui
                            .add_enabled(
                                !in_progress,
                                egui::Button::new(RichText::new("Cancel").color(TEXT_DIM)),
                            )
                            .clicked()
                        {
                            self.apply_confirm_target = None;
                            self.apply_confirm_error = None;
                            self.view_mode = self.apply_confirm_return_view;
                        }
                    });
                });
            });
    }

    /// Render the permission popup modal (on top of everything)
    fn render_permission_popup_modal(&mut self, ctx: &egui::Context) {
        if let Some(action) = render_permission_popup(ctx, &mut self.permission_state) {
            match action {
                PermissionAction::Approve(request_id) => {
                    // Send approval to Bridge via HTTP POST /claude/tool-approval
                    let response = ToolApprovalResponse {
                        request_id: request_id.clone(),
                        decision: ToolDecision::Allow,
                        reason: None,
                        modified_input: None,
                    };
                    match self.bridge_client.send_tool_approval(&response) {
                        Ok(true) => {
                            self.logs.push(LogEvent::system(format!(
                                "✓ Approved tool request: {}",
                                &request_id[..12.min(request_id.len())]
                            )));
                        }
                        Ok(false) => {
                            self.logs.push(LogEvent::error(format!(
                                "Tool approval rejected by bridge: {}",
                                &request_id[..12.min(request_id.len())]
                            )));
                        }
                        Err(e) => {
                            self.logs.push(LogEvent::error(format!(
                                "Failed to send tool approval: {}",
                                e
                            )));
                        }
                    }
                    self.permission_state.next_request();
                }
                PermissionAction::ApproveAll(request_ids) => {
                    let mut approved = 0usize;
                    for request_id in &request_ids {
                        let response = ToolApprovalResponse {
                            request_id: request_id.clone(),
                            decision: ToolDecision::Allow,
                            reason: None,
                            modified_input: None,
                        };
                        match self.bridge_client.send_tool_approval(&response) {
                            Ok(true) => {
                                approved += 1;
                            }
                            Ok(false) => {
                                self.logs.push(LogEvent::error(format!(
                                    "Tool approval rejected by bridge: {}",
                                    &request_id[..12.min(request_id.len())]
                                )));
                            }
                            Err(e) => {
                                self.logs.push(LogEvent::error(format!(
                                    "Failed to send tool approval: {}",
                                    e
                                )));
                            }
                        }
                    }

                    self.logs.push(LogEvent::system(format!(
                        "✓ Approved {} tool request(s)",
                        approved
                    )));

                    // Clear popup state
                    self.permission_state.current_request = None;
                    self.permission_state.pending_requests.clear();
                    self.permission_state.visible = false;
                    self.permission_state.should_focus = false;
                }
                PermissionAction::Deny(request_id, reason) => {
                    // Send denial to Bridge via HTTP POST /claude/tool-approval
                    let response = ToolApprovalResponse {
                        request_id: request_id.clone(),
                        decision: ToolDecision::Deny,
                        reason: Some(reason.clone()),
                        modified_input: None,
                    };
                    match self.bridge_client.send_tool_approval(&response) {
                        Ok(_) => {
                            self.logs.push(LogEvent::system(format!(
                                "✗ Denied tool request: {} ({})",
                                &request_id[..12.min(request_id.len())],
                                reason
                            )));
                        }
                        Err(e) => {
                            self.logs.push(LogEvent::error(format!(
                                "Failed to send tool denial: {}",
                                e
                            )));
                        }
                    }
                    self.permission_state.next_request();
                }
                PermissionAction::Dismiss(request_id) => {
                    // Treat dismiss as deny
                    let response = ToolApprovalResponse {
                        request_id: request_id.clone(),
                        decision: ToolDecision::Deny,
                        reason: Some("User dismissed".to_string()),
                        modified_input: None,
                    };
                    let _ = self.bridge_client.send_tool_approval(&response);
                    self.logs.push(LogEvent::system(format!(
                        "Dismissed tool request: {}",
                        &request_id[..12.min(request_id.len())]
                    )));
                    self.permission_state.next_request();
                }
            }
        }

        // Bring app to foreground if needed
        if self.permission_state.should_focus {
            self.permission_state.should_focus = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
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
                    self.logs
                        .push(LogEvent::system(format!("Selected job #{}", job_id)));
                }
                ComparisonAction::ViewDiff(job_id) => {
                    self.open_job_diff(job_id, ViewMode::ComparisonPopup);
                }
                ComparisonAction::MergeAndClose => {
                    if let Some(group_id) = self.comparison_state.group_id() {
                        let Some(selected_job_id) = self.comparison_state.selected_job_id else {
                            self.logs
                                .push(LogEvent::error("No job selected for merge".to_string()));
                            return;
                        };

                        self.open_apply_confirm(ApplyTarget::Group {
                            group_id,
                            selected_job_id,
                        });
                    }
                }
                ComparisonAction::Cancel => {
                    // Close popup without merging
                    self.comparison_state.close();
                    self.view_mode = ViewMode::JobList;
                }
            }
        }
    }

    /// Apply voice config from settings to the VoiceManager
    fn apply_voice_config(&mut self) {
        let Ok(config) = self.config.read() else {
            return; // Skip if lock poisoned
        };
        let voice_settings = &config.settings.gui.voice;
        let action_registry = super::voice::VoiceActionRegistry::from_config(
            &config.mode,
            &config.chain,
            &config.agent,
        );

        let new_config = VoiceConfig {
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
            vad_config: self.voice_manager.config.vad_config.clone(),
            use_vad: self.voice_manager.config.use_vad,
        };

        self.voice_manager.update_config(new_config);
        self.logs.push(crate::LogEvent::system(
            "Voice settings applied".to_string(),
        ));
    }
}

fn run_apply_thread(input: ApplyThreadInput) -> Result<ApplyThreadOutcome, String> {
    match input {
        ApplyThreadInput::Single(input) => {
            let git =
                crate::git::GitManager::new(&input.workspace_root).map_err(|e| e.to_string())?;

            if let Some(worktree_path) = input.worktree_path {
                let base_branch = input
                    .base_branch
                    .ok_or_else(|| "Job has no base branch recorded".to_string())?;

                git.apply_changes(&worktree_path, &base_branch, Some(&input.commit_message))
                    .map_err(|e| e.to_string())?;

                let mut message = format!("Merged job #{}", input.job_id);
                if let Err(e) = git.remove_worktree_by_path(&worktree_path) {
                    message.push_str(&format!(" (cleanup warning: {})", e));
                }

                Ok(ApplyThreadOutcome {
                    target: ApplyTarget::Single {
                        job_id: input.job_id,
                    },
                    group_job_ids: Vec::new(),
                    message,
                })
            } else {
                match git.commit_root_changes(&input.commit_message) {
                    Ok(true) => Ok(ApplyThreadOutcome {
                        target: ApplyTarget::Single {
                            job_id: input.job_id,
                        },
                        group_job_ids: Vec::new(),
                        message: format!("Committed and applied job #{}", input.job_id),
                    }),
                    Ok(false) => Ok(ApplyThreadOutcome {
                        target: ApplyTarget::Single {
                            job_id: input.job_id,
                        },
                        group_job_ids: Vec::new(),
                        message: format!("Applied job #{} (no changes to commit)", input.job_id),
                    }),
                    Err(e) => Err(e.to_string()),
                }
            }
        }
        ApplyThreadInput::Group(input) => {
            let git =
                crate::git::GitManager::new(&input.workspace_root).map_err(|e| e.to_string())?;

            git.apply_changes(
                &input.selected_worktree_path,
                &input.base_branch,
                Some(&input.commit_message),
            )
            .map_err(|e| e.to_string())?;

            let mut cleanup_warnings = Vec::new();
            for (job_id, worktree_path) in &input.cleanup_worktrees {
                if let Err(e) = git.remove_worktree_by_path(worktree_path) {
                    cleanup_warnings.push(format!("Job #{}: {}", job_id, e));
                }
            }

            let message = if cleanup_warnings.is_empty() {
                format!(
                    "Merged changes from {} and cleaned up {} worktrees",
                    input.selected_agent_id,
                    input.cleanup_worktrees.len()
                )
            } else {
                format!(
                    "Merged changes from {} (cleanup warnings: {})",
                    input.selected_agent_id,
                    cleanup_warnings.join(", ")
                )
            };

            Ok(ApplyThreadOutcome {
                target: ApplyTarget::Group {
                    group_id: input.group_id,
                    selected_job_id: input.selected_job_id,
                },
                group_job_ids: input.group_job_ids,
                message,
            })
        }
    }
}

impl eframe::App for KycoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Refresh jobs periodically (every frame for now, could optimize)
        self.refresh_jobs();

        // Periodically truncate logs (every 60 seconds)
        if self.last_log_cleanup.elapsed().as_secs() >= 60 {
            self.truncate_logs();
            self.last_log_cleanup = std::time::Instant::now();
        }

        // Load inline diff when job selection changes
        if self.selected_job_id != self.prev_selected_job_id {
            self.prev_selected_job_id = self.selected_job_id;
            self.load_inline_diff_for_selected();
        }

        // Check for HTTP selection events from IDE extensions
        while let Ok(req) = self.http_rx.try_recv() {
            self.on_selection_received(req, ctx);
        }

        // Check for batch processing requests from IDE extensions
        while let Ok(req) = self.batch_rx.try_recv() {
            self.on_batch_received(req, ctx);
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
                    self.logs
                        .push(LogEvent::system(format!("Job #{} started", job_id)));
                }
                ExecutorEvent::JobCompleted(job_id) => {
                    self.logs
                        .push(LogEvent::system(format!("Job #{} completed", job_id)));
                    // Check if this job is part of a group and update group status
                    self.check_group_completion(job_id);
                    // Reload diff if this is the currently selected job
                    if self.selected_job_id == Some(job_id) {
                        self.load_inline_diff_for_selected();
                    }
                }
                ExecutorEvent::JobFailed(job_id, error) => {
                    self.logs.push(LogEvent::error(format!(
                        "Job #{} failed: {}",
                        job_id, error
                    )));
                    // Check if this job is part of a group and update group status
                    self.check_group_completion(job_id);
                }
                ExecutorEvent::ChainStepCompleted {
                    job_id,
                    step_index,
                    total_steps,
                    mode,
                    state,
                    step_summary,
                } => {
                    let state_str = state.as_deref().unwrap_or("none");
                    self.logs.push(LogEvent::system(format!(
                        "Chain step {}/{} completed: {} (state: {})",
                        step_index + 1,
                        total_steps,
                        mode,
                        state_str
                    )));
                    // Update chain progress in the job for real-time display
                    if let Ok(mut manager) = self.job_manager.lock() {
                        if let Some(job) = manager.get_mut(job_id) {
                            job.chain_current_step = Some(step_index + 1);
                            // Add step to history if not already present
                            if job.chain_step_history.len() <= step_index {
                                job.chain_step_history.push(step_summary);
                            }
                        }
                    }
                }
                ExecutorEvent::ChainCompleted {
                    job_id: _,
                    chain_name,
                    steps_executed,
                    success,
                } => {
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
                ExecutorEvent::PermissionNeeded {
                    job_id,
                    request_id,
                    session_id,
                    tool_name,
                    tool_input,
                } => {
                    // Convert to PermissionRequest and add to popup queue
                    let request = PermissionRequest {
                        request_id,
                        session_id,
                        tool_name: tool_name.clone(),
                        tool_input,
                        received_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                    };
                    self.permission_state.add_request(request);
                    self.logs.push(
                        LogEvent::permission(format!(
                            "Permission request: {} (waiting)",
                            tool_name
                        ))
                        .for_job(job_id),
                    );

                    // Bring window to front so user notices the permission request
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
            }
        }

        // Process global voice hotkey events (Cmd+Shift+V / Ctrl+Shift+V)
        if self.global_hotkey_manager.is_some() {
            if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                // Any hotkey event triggers voice toggle (we only have one registered)
                if event.state == global_hotkey::HotKeyState::Pressed {
                    self.handle_global_voice_hotkey();
                }
            }
        }

        // Process voice events
        for event in self.voice_manager.poll_events() {
            match event {
                super::voice::VoiceEvent::TranscriptionComplete { text, from_manual } => {
                    // Check if this was a global voice recording (from hotkey)
                    if self.global_voice_recording {
                        // Global voice input - auto-paste to focused application
                        self.handle_global_voice_transcription(&text);
                    } else if from_manual {
                        // Manual recording (button press in popup) - just append text, no wakeword detection
                        if self.popup_input.is_empty() {
                            self.popup_input = text;
                        } else {
                            self.popup_input.push(' ');
                            self.popup_input.push_str(&text);
                        }
                        self.update_suggestions();
                        self.logs
                            .push(LogEvent::system("Voice transcription complete".to_string()));

                        // Auto-execute if Enter was pressed during recording
                        if self.voice_pending_execute {
                            self.voice_pending_execute = false;
                            self.execute_popup_task(false); // Normal execution (no force worktree)
                        }
                    } else {
                        // Continuous listening - try wakeword detection
                        if let Some(wakeword_match) =
                            self.voice_manager.config.action_registry.match_text(&text)
                        {
                            // Wakeword matched - use mode and prompt from the match
                            self.popup_input = format!(
                                "{} {}",
                                wakeword_match.mode,
                                wakeword_match.get_final_prompt()
                            );
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
                            self.logs
                                .push(LogEvent::system("Voice transcription complete".to_string()));
                        }
                    }
                }
                super::voice::VoiceEvent::WakewordMatched {
                    wakeword,
                    mode,
                    prompt,
                } => {
                    // Direct wakeword match from continuous listening
                    self.popup_input = format!("{} {}", mode, prompt);
                    self.update_suggestions();

                    // Open selection popup if not already open
                    if self.view_mode != ViewMode::SelectionPopup {
                        self.view_mode = ViewMode::SelectionPopup;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    self.logs.push(LogEvent::system(format!(
                        "Voice wakeword: {} → {}",
                        wakeword, mode
                    )));
                }
                super::voice::VoiceEvent::KeywordDetected { keyword, full_text } => {
                    // In continuous mode: keyword detected, trigger hotkey and fill input
                    self.popup_input = format!(
                        "{} {}",
                        keyword,
                        full_text.trim_start_matches(&keyword).trim()
                    );
                    self.update_suggestions();

                    // Open selection popup if not already open
                    if self.view_mode != ViewMode::SelectionPopup {
                        self.view_mode = ViewMode::SelectionPopup;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    self.logs.push(LogEvent::system(format!(
                        "Voice keyword detected: {}",
                        keyword
                    )));
                }
                super::voice::VoiceEvent::Error { message } => {
                    self.logs
                        .push(LogEvent::error(format!("Voice error: {}", message)));
                    self.voice_pending_execute = false; // Cancel pending execution on error
                    // Reset global voice state on error
                    if self.global_voice_recording {
                        self.global_voice_recording = false;
                        self.show_voice_overlay = false;
                    }
                }
                super::voice::VoiceEvent::RecordingStarted => {
                    self.logs
                        .push(LogEvent::system("Voice recording started".to_string()));
                }
                super::voice::VoiceEvent::RecordingStopped { duration_secs } => {
                    self.logs.push(LogEvent::system(format!(
                        "Voice recording stopped ({:.1}s)",
                        duration_secs
                    )));
                }
                super::voice::VoiceEvent::VadSpeechStarted => {
                    self.logs
                        .push(LogEvent::system("VAD: Speech detected".to_string()));
                }
                super::voice::VoiceEvent::VadSpeechEnded => {
                    self.logs
                        .push(LogEvent::system("VAD: Speech ended".to_string()));
                }
                _ => {}
            }
        }

        // Handle keyboard shortcuts
        ctx.input(|i| {
            match self.view_mode {
                ViewMode::SelectionPopup => {
                    if i.key_pressed(Key::Escape) {
                        // Cancel recording if active, otherwise close popup
                        if self.voice_manager.state.is_recording() {
                            self.voice_manager.cancel();
                            self.voice_pending_execute = false;
                        } else {
                            self.view_mode = ViewMode::JobList;
                        }
                    }
                    if i.key_pressed(Key::Tab)
                        && self.autocomplete.show_suggestions
                        && !self.autocomplete.suggestions.is_empty()
                    {
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
                        if self.voice_manager.state.is_recording() {
                            // Stop recording and execute after transcription
                            self.voice_pending_execute = true;
                            self.voice_manager.stop_recording();
                        } else if !self.voice_manager.state.is_busy() {
                            // Normal execution (not recording/transcribing)
                            let force_worktree = i.modifiers.shift;
                            self.execute_popup_task(force_worktree);
                        }
                        // If transcribing, do nothing - wait for completion
                    }
                }
                ViewMode::BatchPopup => {
                    // Batch popup uses same keyboard shortcuts as selection popup
                    if i.key_pressed(Key::Escape) {
                        self.batch_files.clear();
                        self.view_mode = ViewMode::JobList;
                    }
                    if i.key_pressed(Key::Tab)
                        && self.autocomplete.show_suggestions
                        && !self.autocomplete.suggestions.is_empty()
                    {
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
                        let force_worktree = i.modifiers.shift;
                        self.execute_batch_task(force_worktree);
                    }
                }
                ViewMode::DiffView => {
                    if i.key_pressed(Key::Escape) || i.key_pressed(Key::Q) {
                        self.view_mode = self.diff_return_view;
                        self.diff_state.clear();
                    }
                }
                ViewMode::ApplyConfirmPopup => {
                    if i.key_pressed(Key::Escape) || i.key_pressed(Key::Q) {
                        if self.apply_confirm_rx.is_none() {
                            self.apply_confirm_target = None;
                            self.apply_confirm_error = None;
                            self.view_mode = self.apply_confirm_return_view;
                        }
                    }
                    if i.key_pressed(Key::Enter) && self.apply_confirm_rx.is_none() {
                        self.start_apply_confirm_merge();
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
                            if let Some(idx) =
                                self.cached_jobs.iter().position(|j| j.id == current_id)
                            {
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
                            if let Some(idx) =
                                self.cached_jobs.iter().position(|j| j.id == current_id)
                            {
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

            // Global shortcut for auto_run toggle (Shift+A)
            if i.modifiers.shift && i.key_pressed(Key::A) {
                self.auto_run = !self.auto_run;
            }

            // Voice hotkey handling (configurable, default: Cmd+D / Ctrl+D)
            if self.view_mode == ViewMode::SelectionPopup {
                let voice_hotkey_pressed = check_egui_hotkey(i, &self.voice_settings_popup_hotkey);

                if voice_hotkey_pressed {
                    // Auto-install voice dependencies if not available
                    if !self.voice_manager.is_available() && !self.voice_install_in_progress {
                        self.voice_install_in_progress = true;
                        self.voice_install_status =
                            Some(("Installing voice dependencies...".to_string(), false));

                        let model_name = self.voice_manager.config.whisper_model.clone();
                        let result = crate::gui::voice::install::install_voice_dependencies(
                            &self.work_dir,
                            &model_name,
                        );

                        self.voice_install_status = Some((result.message.clone(), result.is_error));
                        self.voice_install_in_progress = result.in_progress;

                        // Invalidate availability cache so next check sees the new installation
                        if !result.is_error {
                            self.voice_manager.reset();
                        }
                    } else if !self.voice_install_in_progress {
                        if self.voice_manager.state == VoiceState::Idle
                            || self.voice_manager.state == VoiceState::Error
                        {
                            // Start recording
                            self.voice_manager.start_recording();
                        } else if self.voice_manager.state == VoiceState::Recording {
                            // Stop recording (but don't execute - user can press Enter for that)
                            self.voice_manager.stop_recording();
                        }
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

        // Show init banner if config doesn't exist
        if !self.config_exists {
            egui::TopBottomPanel::top("init_banner")
                .frame(egui::Frame::NONE.fill(ACCENT_YELLOW).inner_margin(8.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("⚠ No configuration found.")
                                .color(BG_PRIMARY)
                                .strong(),
                        );
                        ui.add_space(8.0);
                        if ui
                            .button(
                                egui::RichText::new("Initialize Project")
                                    .color(BG_PRIMARY)
                                    .strong(),
                            )
                            .clicked()
                        {
                            // Create global config at ~/.kyco/config.toml
                            let config_dir = Config::global_config_dir();
                            let config_path = Config::global_config_path();
                            if let Err(e) = std::fs::create_dir_all(&config_dir) {
                                self.logs.push(LogEvent::error(format!(
                                    "Failed to create config directory: {}",
                                    e
                                )));
                            } else if let Err(e) =
                                std::fs::write(&config_path, DEFAULT_CONFIG_MINIMAL)
                            {
                                self.logs.push(LogEvent::error(format!(
                                    "Failed to write config: {}",
                                    e
                                )));
                            } else {
                                self.config_exists = true;
                                self.logs.push(LogEvent::system(format!(
                                    "Created {}",
                                    config_path.display()
                                )));
                            }
                        }
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "Working directory: {}",
                                self.work_dir.display()
                            ))
                            .color(BG_PRIMARY)
                            .small(),
                        );
                    });
                });
        }

        // Poll update checker (needed for status bar)
        let update_info = match self.update_checker.poll() {
            UpdateStatus::UpdateAvailable(info) => Some(info.clone()),
            _ => None,
        };

        // Handle install request
        if matches!(
            self.update_install_status,
            super::status_bar::InstallStatus::InstallRequested
        ) {
            if let Some(info) = &update_info {
                self.update_install_status = super::status_bar::InstallStatus::Installing;
                let (tx, rx) = std::sync::mpsc::channel();
                self.update_install_rx = Some(rx);
                let info_clone = info.clone();
                std::thread::spawn(move || {
                    let result = super::update::install_update(&info_clone);
                    let _ = tx.send(result);
                });
            }
        }

        // Poll install result if we're installing
        if let Some(rx) = &self.update_install_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(msg) => {
                        self.update_install_status = super::status_bar::InstallStatus::Success(msg)
                    }
                    Err(err) => {
                        self.update_install_status = super::status_bar::InstallStatus::Error(err)
                    }
                }
                self.update_install_rx = None;
            }
        }

        // Poll apply/merge result if an operation is running
        let apply_result = self
            .apply_confirm_rx
            .as_ref()
            .and_then(|rx| rx.try_recv().ok());

        if let Some(result) = apply_result {
            self.apply_confirm_rx = None;

            match result {
                Ok(outcome) => {
                    match outcome.target {
                        ApplyTarget::Single { job_id } => {
                            if let Ok(mut jm) = self.job_manager.lock() {
                                if let Some(job) = jm.get_mut(job_id) {
                                    job.set_status(crate::JobStatus::Merged);
                                    job.git_worktree_path = None;
                                    job.branch_name = None;
                                }
                            }
                        }
                        ApplyTarget::Group {
                            group_id,
                            selected_job_id,
                        } => {
                            if let Ok(mut gm) = self.group_manager.lock() {
                                gm.select_result(group_id, selected_job_id);
                                gm.mark_merged(group_id);
                            }

                            if let Ok(mut jm) = self.job_manager.lock() {
                                for job_id in &outcome.group_job_ids {
                                    if let Some(job) = jm.get_mut(*job_id) {
                                        if *job_id == selected_job_id {
                                            job.set_status(crate::JobStatus::Merged);
                                        } else {
                                            job.set_status(crate::JobStatus::Rejected);
                                        }
                                        job.git_worktree_path = None;
                                        job.branch_name = None;
                                    }
                                }
                            }

                            self.comparison_state.close();
                        }
                    }

                    self.logs.push(LogEvent::system(outcome.message));
                    self.apply_confirm_target = None;
                    self.apply_confirm_error = None;
                    self.view_mode = ViewMode::JobList;
                    self.refresh_jobs();
                }
                Err(err) => {
                    self.apply_confirm_error = Some(err);
                }
            }
        }

        // Bottom status bar - MUST be rendered before SidePanel/CentralPanel
        // so that those panels can properly account for the status bar's height
        super::status_bar::render_status_bar(
            ctx,
            &mut super::status_bar::StatusBarState {
                auto_run: &mut self.auto_run,
                view_mode: &mut self.view_mode,
                selected_mode: &mut self.selected_mode,
                mode_edit_status: &mut self.mode_edit_status,
                selected_agent: &mut self.selected_agent,
                agent_edit_status: &mut self.agent_edit_status,
                selected_chain: &mut self.selected_chain,
                chain_edit_status: &mut self.chain_edit_status,
                update_info: update_info.as_ref(),
                install_status: &mut self.update_install_status,
                workspace_registry: Some(&self.workspace_registry),
                active_workspace_id: &mut self.active_workspace_id,
                orchestrator_requested: &mut self.orchestrator_requested,
            },
        );

        if self.orchestrator_requested {
            self.orchestrator_requested = false;
            if let Err(e) = self.launch_orchestrator() {
                self.logs.push(LogEvent::error(format!(
                    "Failed to start orchestrator: {}",
                    e
                )));
            }
        }

        // Render based on view mode
        match self.view_mode {
            ViewMode::JobList => {
                egui::SidePanel::left("job_list")
                    .default_width(280.0)
                    .min_width(280.0)
                    .max_width(280.0)
                    .resizable(false)
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY).inner_margin(8.0))
                    .show(ctx, |ui| {
                        self.render_job_list(ui);
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY).inner_margin(8.0))
                    .show(ctx, |ui| {
                        self.render_detail_panel(ui);
                    });
            }
            ViewMode::SelectionPopup => {
                // Show main UI dimmed behind popup
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_selection_popup(ctx);
            }
            ViewMode::BatchPopup => {
                // Show main UI dimmed behind popup
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_batch_popup(ctx);
            }
            ViewMode::DiffView => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_diff_popup(ctx);
            }
            ViewMode::ApplyConfirmPopup => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_apply_confirm_popup(ctx);
            }
            ViewMode::ComparisonPopup => {
                // Show main UI dimmed behind popup
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(BG_PRIMARY.linear_multiply(0.3)))
                    .show(ctx, |_ui| {});

                self.render_comparison_popup(ctx);
            }
            ViewMode::Settings => {
                self.render_settings(ctx);
                // Apply voice config changes to VoiceManager after settings are saved
                if self.voice_config_changed {
                    self.voice_config_changed = false;
                    self.apply_voice_config();
                }
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

        // Render permission popup on top of everything if visible
        self.render_permission_popup_modal(ctx);

        // Render global voice overlay (small indicator when recording via hotkey)
        if self.show_voice_overlay {
            self.render_voice_overlay(ctx);
        }

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
