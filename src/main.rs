use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod cli;

#[derive(Parser)]
#[command(name = "kyco")]
#[command(about = "Know Your Codebase - AI-powered code tasks with transparency")]
#[command(version)]
struct Cli {
    /// Path to the repository (defaults to current directory)
    #[arg(short, long, global = true)]
    path: Option<PathBuf>,

    /// Path to the config file (defaults to .kyco/config.toml in repo root)
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan the repository for cr: comments and show found tasks
    Scan {
        /// Only show pending tasks (no status marker)
        #[arg(long)]
        pending_only: bool,
    },

    /// Run the TUI and execute jobs
    Run {
        /// Maximum number of concurrent jobs (overrides config.toml)
        #[arg(long)]
        max_jobs: Option<usize>,

        /// Auto-start pending jobs without confirmation (overrides config.toml)
        #[arg(long)]
        auto_start: bool,

        /// Debounce interval for file watcher in milliseconds (overrides config.toml)
        #[arg(long)]
        debounce_ms: Option<u64>,

        /// Marker prefix for comment detection (overrides config.toml, e.g., "@@", "::", "TODO:")
        #[arg(long)]
        marker_prefix: Option<String>,

        /// Run jobs in isolated Git worktrees (overrides config.toml)
        #[arg(long)]
        use_worktree: bool,

        /// Disable Git worktree isolation (overrides config.toml)
        #[arg(long, conflicts_with = "use_worktree")]
        no_worktree: bool,

        /// Additional glob patterns to exclude from scanning (can be specified multiple times)
        #[arg(long = "exclude", short = 'x')]
        scan_exclude: Vec<String>,
    },

    /// Show the status of all jobs
    Status {
        /// Show only jobs with this status
        #[arg(long)]
        filter: Option<String>,
    },

    /// Initialize a new .kyco/config.toml configuration file
    Init {
        /// Overwrite existing config file
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .init();

    // Determine the working directory
    let work_dir = cli.path.unwrap_or_else(|| PathBuf::from("."));

    match cli.command {
        Some(Commands::Scan { pending_only }) => {
            cli::scan::scan_command(&work_dir, pending_only).await?;
        }
        Some(Commands::Run {
            max_jobs,
            auto_start,
            debounce_ms,
            marker_prefix,
            use_worktree,
            no_worktree,
            scan_exclude,
        }) => {
            let worktree_override = if use_worktree {
                Some(true)
            } else if no_worktree {
                Some(false)
            } else {
                None
            };
            cli::run::run_command(
                &work_dir,
                cli.config,
                cli::run::CliSettings {
                    max_jobs,
                    auto_start,
                    debounce_ms,
                    marker_prefix,
                    use_worktree: worktree_override,
                    scan_exclude,
                },
            )
            .await?;
        }
        Some(Commands::Status { filter }) => {
            cli::status::status_command(&work_dir, filter).await?;
        }
        Some(Commands::Init { force }) => {
            cli::init::init_command(&work_dir, force).await?;
        }
        None => {
            // Default: run the TUI with config defaults
            cli::run::run_command(
                &work_dir,
                cli.config,
                cli::run::CliSettings::default(),
            )
            .await?;
        }
    }

    Ok(())
}
