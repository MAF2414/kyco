//! Main TUI application

use anyhow::Result;
use crossterm::{
    event::KeyCode,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};

use super::event::{is_quit_event, AppEvent, EventHandler};
use super::ui;
use crate::agent::{get_terminal_session, AgentRegistry};
use crate::config::Config;
use crate::git::GitManager;
use crate::job::JobManager;
use crate::scanner::Scanner;
use crate::watcher::{FileWatcher, WatchEvent};
use crate::{Job, JobStatus, LogEvent};

/// Main TUI application state
pub struct App {
    /// Working directory
    work_dir: PathBuf,

    /// Configuration
    config: Config,

    /// Job manager
    job_manager: Arc<Mutex<JobManager>>,

    /// Git manager
    git_manager: Option<GitManager>,

    /// Agent registry
    agent_registry: AgentRegistry,

    /// Currently selected job index
    selected_job: usize,

    /// Log events for display
    logs: Vec<LogEvent>,

    /// Whether to show help
    show_help: bool,

    /// Maximum concurrent jobs
    max_jobs: usize,

    /// Auto-start pending jobs (stored for potential future use)
    #[allow(dead_code)]
    auto_start: bool,

    /// Channel for receiving log events
    log_rx: mpsc::Receiver<LogEvent>,

    /// Channel for sending log events (to be cloned for runners)
    log_tx: mpsc::Sender<LogEvent>,

    /// Whether the app should quit
    should_quit: bool,

    /// File watcher for detecting changes
    file_watcher: Option<FileWatcher>,

    /// Auto-run new jobs immediately
    auto_run_enabled: bool,

    /// Whether to show diff popup
    show_diff: bool,

    /// Content of the diff to display
    diff_content: Option<String>,

    /// Scroll offset for diff view
    diff_scroll: usize,
}

impl App {
    /// Create a new TUI application
    pub async fn new(
        work_dir: PathBuf,
        config: Config,
        max_jobs: usize,
        auto_start: bool,
    ) -> Result<Self> {
        let job_manager = Arc::new(Mutex::new(JobManager::load(&work_dir)?));

        let git_manager = match GitManager::new(&work_dir) {
            Ok(manager) => Some(manager),
            Err(e) => {
                eprintln!("Warning: Git manager not available: {}", e);
                None
            }
        };

        let (log_tx, log_rx) = mpsc::channel(1000);

        // Create file watcher with debounce from config
        let debounce_ms = config.settings.debounce_ms;
        let file_watcher = match FileWatcher::new(&work_dir, debounce_ms) {
            Ok(watcher) => Some(watcher),
            Err(e) => {
                eprintln!("Warning: Could not start file watcher: {}", e);
                None
            }
        };

        // Auto-run can be enabled via config or CLI flag
        let auto_run_enabled = auto_start || config.settings.auto_run;

        Ok(Self {
            work_dir,
            config,
            job_manager,
            git_manager,
            agent_registry: AgentRegistry::with_defaults(),
            selected_job: 0,
            logs: Vec::new(),
            show_help: false,
            max_jobs,
            auto_start,
            log_rx,
            log_tx,
            should_quit: false,
            file_watcher,
            auto_run_enabled,
            show_diff: false,
            diff_content: None,
            diff_scroll: 0,
        })
    }

    /// Run the TUI application
    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Initial scan
        self.scan_for_tasks(false).await?;

        // Add initial log
        if self.file_watcher.is_some() {
            self.logs.push(LogEvent::system("KYCo started (watching for changes)"));
        } else {
            self.logs.push(LogEvent::system("KYCo started (no file watcher)"));
        }

        // Event handler
        let event_handler = EventHandler::new(Duration::from_millis(100));

        // Main loop
        while !self.should_quit {
            // Check for log events
            while let Ok(event) = self.log_rx.try_recv() {
                self.logs.push(event);
            }

            // Check for file system events - collect first to avoid borrow issues
            let mut should_scan = false;
            if let Some(ref watcher) = self.file_watcher {
                while let Some(event) = watcher.try_recv() {
                    match event {
                        WatchEvent::FileChanged(path) => {
                            // Only log at debug level, don't spam
                            tracing::debug!("File changed: {}", path.display());
                            should_scan = true;
                        }
                        WatchEvent::Error(e) => {
                            self.logs.push(LogEvent::error(format!("Watcher error: {}", e)));
                        }
                    }
                }
            }
            if should_scan {
                self.scan_for_tasks(true).await?;
            }

            // Draw UI
            self.draw(&mut terminal).await?;

            // Handle events
            match event_handler.next()? {
                AppEvent::Key(key) => {
                    if is_quit_event(&key) {
                        self.should_quit = true;
                    } else {
                        self.handle_key(key.code).await?;
                    }
                }
                AppEvent::Resize(_, _) => {
                    // Terminal will handle resize automatically
                }
                AppEvent::Tick => {
                    // Try to start queued jobs if we have capacity
                    self.process_queue().await?;
                }
            }
        }

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

        Ok(())
    }

    /// Draw the UI
    async fn draw(&self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        let jobs = {
            let manager = self.job_manager.lock().await;
            manager.jobs().into_iter().cloned().collect::<Vec<_>>()
        };
        let job_refs: Vec<&Job> = jobs.iter().collect();

        terminal.draw(|frame| {
            ui::render(
                frame,
                &job_refs,
                self.selected_job,
                &self.logs,
                self.show_help,
                &self.config,
                self.show_diff,
                self.diff_content.as_deref(),
                self.diff_scroll,
            );
        })?;

        Ok(())
    }

    /// Handle a key press
    async fn handle_key(&mut self, code: KeyCode) -> Result<()> {
        // If diff popup is open, handle its keys first
        if self.show_diff {
            match code {
                KeyCode::Esc | KeyCode::Char('d') | KeyCode::Char('q') => {
                    self.show_diff = false;
                    self.diff_content = None;
                    self.diff_scroll = 0;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.diff_scroll = self.diff_scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.diff_scroll = self.diff_scroll.saturating_add(1);
                }
                KeyCode::PageUp => {
                    self.diff_scroll = self.diff_scroll.saturating_sub(20);
                }
                KeyCode::PageDown => {
                    self.diff_scroll = self.diff_scroll.saturating_add(20);
                }
                _ => {}
            }
            return Ok(());
        }

        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_job > 0 {
                    self.selected_job -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let job_count = self.job_manager.lock().await.jobs().len();
                if self.selected_job + 1 < job_count {
                    self.selected_job += 1;
                }
            }
            KeyCode::Enter => {
                self.start_selected_job().await?;
            }
            KeyCode::Char('a') => {
                self.apply_selected_job().await?;
            }
            KeyCode::Char('r') => {
                self.reject_selected_job().await?;
            }
            KeyCode::Char('s') => {
                self.logs.push(LogEvent::system("Manual scan triggered..."));
                self.scan_for_tasks(false).await?;
            }
            KeyCode::Char('d') => {
                self.show_diff_for_selected_job().await?;
            }
            KeyCode::Char('m') => {
                self.merge_selected_job().await?;
            }
            KeyCode::Char('f') => {
                self.focus_terminal_for_selected_job().await?;
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            _ => {}
        }

        Ok(())
    }

    /// Scan for new tasks in the repository
    ///
    /// If `silent` is true, don't log "Scanning..." messages (used for file watcher triggers)
    async fn scan_for_tasks(&mut self, silent: bool) -> Result<()> {
        let scanner = Scanner::with_config(
            &self.work_dir,
            &self.config.settings.scan_exclude,
            &self.config.settings.marker_prefix,
        );
        let tags = scanner.scan().await?;

        let mut manager = self.job_manager.lock().await;
        let mut new_job_ids = Vec::new();

        // Get existing job locations to avoid duplicates
        let existing_locations: std::collections::HashSet<(String, usize)> = manager
            .jobs()
            .iter()
            .map(|j| (j.source_file.display().to_string(), j.source_line))
            .collect();

        for tag in tags {
            // Skip if already linked to a job
            if tag.is_linked() {
                continue;
            }

            // Skip if we already have a job for this file+line
            let location = (tag.file_path.display().to_string(), tag.line_number);
            if existing_locations.contains(&location) {
                continue;
            }

            // Get the agent for this mode (use tag.agent if specified)
            let agent_id = if tag.agent != "claude" && !tag.agent.is_empty() {
                tag.agent.clone()
            } else {
                self.config.get_agent_for_mode(&tag.mode)
            };

            // Create a new job
            let job_id = manager.create_job(&tag, &agent_id)?;
            new_job_ids.push(job_id);
        }

        // Log and optionally auto-queue new jobs
        if !new_job_ids.is_empty() {
            if !silent {
                self.logs.push(LogEvent::system(format!(
                    "Found {} new task(s)",
                    new_job_ids.len()
                )));
            }

            // Auto-queue jobs if enabled
            if self.auto_run_enabled {
                for job_id in new_job_ids {
                    manager.set_status(job_id, JobStatus::Queued);
                    self.logs.push(LogEvent::system(format!(
                        "Auto-queued job #{}",
                        job_id
                    )));
                }
            }
        }

        Ok(())
    }

    /// Queue the currently selected job for execution
    async fn start_selected_job(&mut self) -> Result<()> {
        let job_id = {
            let manager = self.job_manager.lock().await;
            let jobs = manager.jobs();
            jobs.get(self.selected_job).map(|j| j.id)
        };

        let Some(job_id) = job_id else {
            return Ok(());
        };

        // Check if job is pending
        {
            let manager = self.job_manager.lock().await;
            if let Some(job) = manager.get(job_id) {
                if job.status != JobStatus::Pending {
                    self.logs.push(LogEvent::system(format!(
                        "Job #{} is not pending (status: {})",
                        job_id, job.status
                    )));
                    return Ok(());
                }
            }
        }

        // Add to queue
        {
            let mut manager = self.job_manager.lock().await;
            manager.set_status(job_id, JobStatus::Queued);
        }

        self.logs
            .push(LogEvent::system(format!("Job #{} added to queue", job_id)));

        // Try to start immediately if we have capacity
        self.process_queue().await?;

        Ok(())
    }

    /// Process the job queue - start jobs if we have capacity
    async fn process_queue(&mut self) -> Result<()> {
        // Count running jobs
        let (running_count, next_queued_job) = {
            let manager = self.job_manager.lock().await;
            let running = manager
                .jobs()
                .iter()
                .filter(|j| j.status == JobStatus::Running)
                .count();
            let next_queued = manager
                .jobs()
                .iter()
                .find(|j| j.status == JobStatus::Queued)
                .map(|j| j.id);
            (running, next_queued)
        };

        // Start next job if we have capacity
        if running_count < self.max_jobs {
            if let Some(job_id) = next_queued_job {
                self.run_job(job_id).await?;
            }
        }

        Ok(())
    }

    /// Actually run a job (internal - called from process_queue)
    async fn run_job(&mut self, job_id: u64) -> Result<()> {
        // Update status to running
        {
            let mut manager = self.job_manager.lock().await;
            manager.set_status(job_id, JobStatus::Running);
        }

        self.logs
            .push(LogEvent::system(format!("Starting job #{}", job_id)));

        // Determine working directory for the job
        // If use_worktree is enabled and we have git, create a worktree
        // Otherwise, use the main working directory
        let (worktree_path, _is_isolated_worktree) = if self.config.settings.use_worktree {
            if let Some(git) = &self.git_manager {
                match git.create_worktree(job_id) {
                    Ok(path) => {
                        self.logs.push(LogEvent::system(format!(
                            "Created worktree: {}",
                            path.display()
                        )));
                        (path, true)
                    }
                    Err(e) => {
                        self.logs.push(LogEvent::error(format!(
                            "Failed to create worktree: {}",
                            e
                        )));
                        // Fall back to working directory
                        (self.work_dir.clone(), false)
                    }
                }
            } else {
                self.logs.push(LogEvent::error(
                    "use_worktree enabled but Git not available - running in main directory"
                ));
                (self.work_dir.clone(), false)
            }
        } else {
            // use_worktree disabled - run in main working directory
            (self.work_dir.clone(), false)
        };

        // Get job and config for runner
        let (job, agent_config) = {
            let manager = self.job_manager.lock().await;
            let job = manager.get(job_id).cloned();
            let config = job
                .as_ref()
                .and_then(|j| self.config.get_agent(&j.agent_id));
            (job, config)
        };

        let Some(job) = job else {
            return Ok(());
        };

        // Remove the @@-tag from the source file before the agent runs
        // This prevents the AI model from being confused by seeing the tag in the code
        //
        // Note: When using isolated worktrees, this only affects the worktree copy.
        // When NOT using worktrees, this modifies the main file - the scanner will
        // no longer find this tag, but that's okay because the job already exists.
        if let Err(e) = self.remove_tag_from_source(&job, &worktree_path) {
            // Log to internal event system, not tracing (which interferes with TUI)
            self.logs.push(LogEvent::error(format!(
                "Failed to remove tag: {}",
                e
            )));
            // Continue anyway - this is not fatal
        }

        let agent_config = agent_config.unwrap_or_default();

        // Get the appropriate adapter for this agent (respects print/repl mode)
        let adapter = match self.agent_registry.get_for_config(&agent_config) {
            Some(adapter) => adapter,
            None => {
                self.logs.push(LogEvent::error(format!(
                    "No adapter found for agent '{}' (cli_type: {:?}, mode: {:?})",
                    job.agent_id, agent_config.cli_type, agent_config.mode
                )));
                return Ok(());
            }
        };

        let log_tx = self.log_tx.clone();
        let job_manager = self.job_manager.clone();

        // Spawn the agent run in a background task
        tokio::spawn(async move {
            match adapter.run(&job, &worktree_path, &agent_config, log_tx.clone()).await {
                Ok(result) => {
                    let mut manager = job_manager.lock().await;
                    if let Some(j) = manager.get_mut(job_id) {
                        // Store the sent prompt
                        j.sent_prompt = result.sent_prompt;
                    }
                    if result.success {
                        if let Some(j) = manager.get_mut(job_id) {
                            j.set_status(JobStatus::Done);
                            j.changed_files = result.changed_files;
                        }
                        let _ = log_tx
                            .send(LogEvent::system(format!("Job #{} completed", job_id)))
                            .await;
                    } else {
                        if let Some(j) = manager.get_mut(job_id) {
                            j.fail(result.error.unwrap_or_else(|| "Unknown error".to_string()));
                        }
                        let _ = log_tx
                            .send(LogEvent::error(format!("Job #{} failed", job_id)))
                            .await;
                    }
                }
                Err(e) => {
                    let mut manager = job_manager.lock().await;
                    if let Some(j) = manager.get_mut(job_id) {
                        j.fail(e.to_string());
                    }
                    let _ = log_tx
                        .send(LogEvent::error(format!("Job #{} error: {}", job_id, e)))
                        .await;
                }
            }
        });

        Ok(())
    }

    /// Apply changes from the selected job
    async fn apply_selected_job(&mut self) -> Result<()> {
        let job_id = {
            let manager = self.job_manager.lock().await;
            let jobs = manager.jobs();
            jobs.get(self.selected_job).map(|j| j.id)
        };

        let Some(job_id) = job_id else {
            return Ok(());
        };

        // Check if job is done
        {
            let manager = self.job_manager.lock().await;
            if let Some(job) = manager.get(job_id) {
                if job.status != JobStatus::Done {
                    self.logs.push(LogEvent::system(format!(
                        "Job #{} is not done (status: {})",
                        job_id, job.status
                    )));
                    return Ok(());
                }
            }
        }

        // Apply changes from worktree
        if let Some(git) = &self.git_manager {
            let worktree_path = self.work_dir.join(".kyco").join("worktrees").join(format!("job-{}", job_id));

            if worktree_path.exists() {
                match git.apply_changes(&worktree_path) {
                    Ok(()) => {
                        self.logs.push(LogEvent::system(format!(
                            "Applied changes from job #{}",
                            job_id
                        )));

                        // Clean up worktree
                        if let Err(e) = git.remove_worktree(job_id) {
                            self.logs.push(LogEvent::error(format!(
                                "Failed to remove worktree: {}",
                                e
                            )));
                        }
                    }
                    Err(e) => {
                        self.logs.push(LogEvent::error(format!(
                            "Failed to apply changes: {}",
                            e
                        )));
                    }
                }
            } else {
                self.logs.push(LogEvent::error("No worktree found for this job"));
            }
        } else {
            self.logs.push(LogEvent::error("Git not available"));
        }

        Ok(())
    }

    /// Focus the terminal window for the selected job (if running in REPL mode)
    async fn focus_terminal_for_selected_job(&mut self) -> Result<()> {
        let job_id = {
            let manager = self.job_manager.lock().await;
            let jobs = manager.jobs();
            jobs.get(self.selected_job).map(|j| j.id)
        };

        let Some(job_id) = job_id else {
            return Ok(());
        };

        // Check if there's a terminal session for this job
        if let Some(session) = get_terminal_session(job_id) {
            if session.is_running() {
                if let Err(e) = session.focus() {
                    self.logs.push(LogEvent::error(format!(
                        "Failed to focus terminal: {}",
                        e
                    )));
                } else {
                    self.logs.push(LogEvent::system(format!(
                        "Focused terminal for job #{}",
                        job_id
                    )));
                }
            } else {
                self.logs.push(LogEvent::system(format!(
                    "Job #{} terminal session has ended",
                    job_id
                )));
            }
        } else {
            self.logs.push(LogEvent::system(format!(
                "No terminal session for job #{} (not a REPL job or not running)",
                job_id
            )));
        }

        Ok(())
    }

    /// Reject the selected job
    async fn reject_selected_job(&mut self) -> Result<()> {
        let job_id = {
            let manager = self.job_manager.lock().await;
            let jobs = manager.jobs();
            jobs.get(self.selected_job).map(|j| j.id)
        };

        let Some(job_id) = job_id else {
            return Ok(());
        };

        // Update status
        {
            let mut manager = self.job_manager.lock().await;
            manager.set_status(job_id, JobStatus::Rejected);
        }

        self.logs
            .push(LogEvent::system(format!("Rejected job #{}", job_id)));

        // Clean up worktree if exists
        if let Some(git) = &self.git_manager {
            if let Err(e) = git.remove_worktree(job_id) {
                self.logs.push(LogEvent::error(format!(
                    "Failed to remove worktree: {}",
                    e
                )));
            }
        }

        Ok(())
    }

    /// Show diff for the selected job's worktree
    async fn show_diff_for_selected_job(&mut self) -> Result<()> {
        let job_id = {
            let manager = self.job_manager.lock().await;
            let jobs = manager.jobs();
            jobs.get(self.selected_job).map(|j| j.id)
        };

        let Some(job_id) = job_id else {
            return Ok(());
        };

        let Some(git) = &self.git_manager else {
            self.logs.push(LogEvent::error("Git not available"));
            return Ok(());
        };

        let worktree_path = self.work_dir.join(".kyco").join("worktrees").join(format!("job-{}", job_id));

        if !worktree_path.exists() {
            self.logs.push(LogEvent::error(format!(
                "No worktree found for job #{} (path: {})",
                job_id,
                worktree_path.display()
            )));
            return Ok(());
        }

        match git.diff(&worktree_path) {
            Ok(diff) => {
                if diff.is_empty() {
                    self.logs.push(LogEvent::system(format!(
                        "No changes in worktree for job #{}",
                        job_id
                    )));
                } else {
                    self.diff_content = Some(diff);
                    self.diff_scroll = 0;
                    self.show_diff = true;
                }
            }
            Err(e) => {
                self.logs.push(LogEvent::error(format!(
                    "Failed to get diff: {}",
                    e
                )));
            }
        }

        Ok(())
    }

    /// Merge changes from the selected job's worktree into the main branch
    async fn merge_selected_job(&mut self) -> Result<()> {
        // Single lock acquisition to get job_id and validate status
        let job_id = {
            let manager = self.job_manager.lock().await;
            let jobs = manager.jobs();
            match jobs.get(self.selected_job) {
                Some(job) if job.status != JobStatus::Done => {
                    self.logs.push(LogEvent::error(format!(
                        "Job #{} is not done (status: {}). Complete the job first.",
                        job.id, job.status
                    )));
                    return Ok(());
                }
                Some(job) => job.id,
                None => return Ok(()),
            }
        };

        let Some(git) = &self.git_manager else {
            self.logs.push(LogEvent::error("Git not available"));
            return Ok(());
        };

        let worktree_path = self.work_dir.join(".kyco").join("worktrees").join(format!("job-{}", job_id));

        if !worktree_path.exists() {
            self.logs.push(LogEvent::error(format!(
                "No worktree found for job #{}",
                job_id
            )));
            return Ok(());
        }

        // Apply changes from worktree to main
        match git.apply_changes(&worktree_path) {
            Ok(()) => {
                self.logs.push(LogEvent::system(format!(
                    "Merged changes from job #{} into main working directory",
                    job_id
                )));

                // Update job status
                {
                    let mut manager = self.job_manager.lock().await;
                    if let Some(job) = manager.get_mut(job_id) {
                        job.set_status(JobStatus::Rejected); // Use Rejected as "merged/closed"
                    }
                }

                // Clean up worktree
                if let Err(e) = git.remove_worktree(job_id) {
                    self.logs.push(LogEvent::error(format!(
                        "Failed to remove worktree after merge: {}",
                        e
                    )));
                } else {
                    self.logs.push(LogEvent::system("Worktree cleaned up"));
                }
            }
            Err(e) => {
                self.logs.push(LogEvent::error(format!(
                    "Failed to merge changes: {}",
                    e
                )));
            }
        }

        Ok(())
    }

    /// Remove the @@-tag from the source file before running the agent
    ///
    /// Simply removes the exact line that the scanner found (raw_tag_line).
    /// This is much simpler and more reliable than trying to re-parse.
    fn remove_tag_from_source(&self, job: &Job, work_path: &PathBuf) -> Result<()> {
        let Some(raw_tag_line) = &job.raw_tag_line else {
            return Ok(());
        };

        let relative_path = job.source_file.strip_prefix(&self.work_dir)
            .unwrap_or(&job.source_file);
        let target_file = work_path.join(relative_path);

        if !target_file.exists() {
            anyhow::bail!("Source file not found: {}", target_file.display());
        }

        let content = std::fs::read_to_string(&target_file)?;
        let marker_prefix = &self.config.settings.marker_prefix;
        let trimmed_tag = raw_tag_line.trim();

        // Check if standalone comment or inline
        let is_standalone = trimmed_tag.starts_with("//")
            || trimmed_tag.starts_with('#')
            || trimmed_tag.starts_with("/*")
            || trimmed_tag.starts_with("--")
            || trimmed_tag.starts_with(marker_prefix);

        let has_trailing_newline = content.ends_with('\n');

        // Single-pass: pre-allocate capacity to avoid reallocations
        let mut new_content = String::with_capacity(content.len());
        let mut first_line = true;

        for line in content.lines() {
            let should_skip = is_standalone && line.trim() == trimmed_tag;

            if should_skip {
                continue;
            }

            if !first_line {
                new_content.push('\n');
            }
            first_line = false;

            if !is_standalone && (line == raw_tag_line || line.trim() == trimmed_tag) {
                // Inline: remove just the tag comment part, keep the code
                if let Some(marker_pos) = line.find(marker_prefix) {
                    let before_marker = &line[..marker_pos];
                    let comment_start = before_marker.rfind("//")
                        .or_else(|| before_marker.rfind('#'))
                        .or_else(|| before_marker.rfind("--"))
                        .or_else(|| before_marker.rfind("/*"));

                    if let Some(start) = comment_start {
                        new_content.push_str(line[..start].trim_end());
                        continue;
                    }
                }
            }

            new_content.push_str(line);
        }

        // Preserve trailing newline
        if has_trailing_newline {
            new_content.push('\n');
        }

        std::fs::write(&target_file, new_content)?;
        Ok(())
    }
}

/// Remove tag and description lines from content (extracted for testing)
///
/// Returns the modified content with the tag and its description removed.
#[cfg(test)]
fn remove_tag_from_content(
    content: &str,
    tag_line_number: usize, // 1-indexed
    marker_prefix: &str,
) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let tag_line_idx = tag_line_number.saturating_sub(1);

    if tag_line_idx >= lines.len() {
        return None;
    }

    let tag_line = lines[tag_line_idx];

    if !tag_line.contains(marker_prefix) {
        return None;
    }

    // Check if this is a standalone tag line or an inline tag
    let trimmed = tag_line.trim();
    let is_standalone = trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("/*")
        || trimmed.starts_with("--")
        || trimmed.starts_with("*")
        || trimmed.starts_with(marker_prefix);

    let mut new_lines: Vec<String> = Vec::with_capacity(lines.len());

    for (idx, line) in lines.iter().enumerate() {
        if idx == tag_line_idx {
            if is_standalone {
                continue;
            } else {
                // Inline tag: remove only the tag comment part, keep the code
                if let Some(marker_pos) = line.find(marker_prefix) {
                    let before_marker = &line[..marker_pos];
                    let comment_start = before_marker.rfind("//")
                        .or_else(|| before_marker.rfind('#'))
                        .or_else(|| before_marker.rfind("--"))
                        .or_else(|| before_marker.rfind("/*"));

                    if let Some(start) = comment_start {
                        let code_part = line[..start].trim_end();
                        if !code_part.is_empty() {
                            new_lines.push(code_part.to_string());
                        }
                        continue;
                    }
                }
                continue;
            }
        }

        // For lines after the tag line, check if they're description continuations
        if idx > tag_line_idx && is_standalone {
            let line_trimmed = line.trim();

            let is_comment = line_trimmed.starts_with("//")
                || line_trimmed.starts_with('#')
                || line_trimmed.starts_with("/*")
                || line_trimmed.starts_with("--")
                || line_trimmed.starts_with('*');

            let has_marker = line_trimmed.contains(marker_prefix);

            if is_comment && !has_marker {
                let still_in_continuation = (tag_line_idx + 1..idx).all(|i| {
                    let prev = lines[i].trim();
                    prev.starts_with("//") || prev.starts_with('#')
                        || prev.starts_with("--") || prev.starts_with('*')
                        || prev.is_empty()
                });

                if still_in_continuation {
                    continue;
                }
            }
        }

        new_lines.push(line.to_string());
    }

    let new_content = new_lines.join("\n");
    let final_content = if content.ends_with('\n') {
        format!("{}\n", new_content)
    } else {
        new_content
    };

    Some(final_content)
}

#[cfg(test)]
mod tests;
