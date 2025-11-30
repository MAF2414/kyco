//! Run command implementation (TUI mode)

use anyhow::Result;
use std::path::{Path, PathBuf};

use kyco::config::Config;
use kyco::tui::App;

/// CLI settings that can override config.toml values
#[derive(Debug, Default)]
pub struct CliSettings {
    /// Maximum concurrent jobs (overrides config if Some)
    pub max_jobs: Option<usize>,
    /// Auto-start jobs when found
    pub auto_start: bool,
    /// Debounce interval for file watcher (overrides config if Some)
    pub debounce_ms: Option<u64>,
    /// Marker prefix for comment detection (overrides config if Some)
    pub marker_prefix: Option<String>,
    /// Use git worktrees for isolation (overrides config if Some)
    pub use_worktree: Option<bool>,
    /// Additional glob patterns to exclude from scanning
    pub scan_exclude: Vec<String>,
}

/// Run the TUI and execute jobs
pub async fn run_command(
    work_dir: &Path,
    config_path: Option<PathBuf>,
    cli_settings: CliSettings,
) -> Result<()> {
    // Load configuration
    let mut config = match config_path {
        Some(path) => Config::from_file(&path)?,
        None => Config::from_dir(work_dir)?,
    };

    // Override config settings with CLI arguments where provided
    if let Some(max_jobs) = cli_settings.max_jobs {
        config.settings.max_concurrent_jobs = max_jobs;
    }
    if cli_settings.auto_start {
        config.settings.auto_run = true;
    }
    if let Some(debounce_ms) = cli_settings.debounce_ms {
        config.settings.debounce_ms = debounce_ms;
    }
    if let Some(marker_prefix) = cli_settings.marker_prefix {
        config.settings.marker_prefix = marker_prefix;
    }
    if let Some(use_worktree) = cli_settings.use_worktree {
        config.settings.use_worktree = use_worktree;
    }
    // Append additional exclude patterns from CLI
    if !cli_settings.scan_exclude.is_empty() {
        config.settings.scan_exclude.extend(cli_settings.scan_exclude);
    }

    // Get max_jobs from config (already merged with CLI override)
    let max_jobs = config.settings.max_concurrent_jobs;
    let auto_start = config.settings.auto_run;

    // Create and run the TUI application
    let mut app = App::new(work_dir.to_path_buf(), config, max_jobs, auto_start).await?;
    app.run().await?;

    Ok(())
}
