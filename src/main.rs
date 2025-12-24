use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;

use kyco::cli;

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
    /// Run the GUI (receives selections from IDE extensions via HTTP)
    Gui,

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

    /// Control jobs in a running KYCo GUI (local /ctl API)
    Job {
        #[command(subcommand)]
        command: JobCommands,
    },

    /// Manage modes in `.kyco/config.toml`
    Mode {
        #[command(subcommand)]
        command: ModeCommands,
    },

    /// List/show configured agents
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },

    /// List/show configured chains
    Chain {
        #[command(subcommand)]
        command: ChainCommands,
    },
}

#[derive(Subcommand)]
enum JobCommands {
    /// List jobs
    List {
        /// Print JSON instead of human output
        #[arg(long)]
        json: bool,
    },
    /// Get a single job by ID
    Get {
        job_id: u64,
        /// Print JSON instead of human output
        #[arg(long)]
        json: bool,
    },
    /// Start a job (creates it in the GUI and optionally queues it)
    Start {
        /// File path (relative to --path, or absolute)
        #[arg(long)]
        file: String,
        /// Start line (1-indexed)
        #[arg(long)]
        line_start: Option<usize>,
        /// End line (1-indexed)
        #[arg(long)]
        line_end: Option<usize>,
        /// Mode or chain name
        #[arg(long)]
        mode: String,
        /// Optional prompt/description text
        #[arg(long)]
        prompt: Option<String>,
        /// Primary agent id (e.g. "claude")
        #[arg(long)]
        agent: Option<String>,
        /// Optional list of agents for parallel execution (comma-separated)
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Create as pending only (do not queue immediately)
        #[arg(long)]
        pending: bool,
        /// Force running in a git worktree (like Shift+Enter in the UI)
        #[arg(long)]
        force_worktree: bool,
        /// Print JSON response
        #[arg(long)]
        json: bool,
    },
    /// Queue a job (set status=queued)
    Queue { job_id: u64 },
    /// Abort/stop a job
    Abort { job_id: u64 },
    /// Delete a job from the GUI list
    Delete {
        job_id: u64,
        /// Also remove the job's git worktree (if any)
        #[arg(long)]
        cleanup_worktree: bool,
    },
    /// Continue a session job with a follow-up prompt (creates a new job)
    Continue {
        job_id: u64,
        /// Follow-up prompt text
        #[arg(long)]
        prompt: String,
        /// Create as pending only (do not queue immediately)
        #[arg(long)]
        pending: bool,
        /// Print JSON response
        #[arg(long)]
        json: bool,
    },
    /// Wait until a job reaches a terminal state
    Wait {
        job_id: u64,
        /// Timeout in seconds
        #[arg(long)]
        timeout_secs: Option<u64>,
        /// Poll interval in milliseconds
        #[arg(long, default_value_t = 500)]
        poll_ms: u64,
        /// Print final job JSON
        #[arg(long)]
        json: bool,
    },
    /// Print a job's output / result
    Output {
        job_id: u64,
        /// Print full job JSON
        #[arg(long)]
        json: bool,
        /// Print parsed `result.summary` (or raw fallback)
        #[arg(long)]
        summary: bool,
        /// Print parsed `result.state`
        #[arg(long)]
        state: bool,
    },
}

#[derive(Subcommand)]
enum ModeCommands {
    /// List configured modes
    List {
        /// Print JSON instead of plain lines
        #[arg(long)]
        json: bool,
    },
    /// Show a mode definition
    Get {
        name: String,
        /// Print JSON instead of TOML
        #[arg(long)]
        json: bool,
    },
    /// Create or update a mode
    Set {
        name: String,
        /// Prompt template
        #[arg(long)]
        prompt: Option<String>,
        /// System prompt
        #[arg(long)]
        system_prompt: Option<String>,
        /// Default agent id
        #[arg(long)]
        agent: Option<String>,
        /// Aliases (comma-separated)
        #[arg(long, value_delimiter = ',')]
        aliases: Vec<String>,
        /// Session mode: oneshot|session
        #[arg(long)]
        session_mode: Option<String>,
        /// Max turns (0 = unlimited)
        #[arg(long)]
        max_turns: Option<u32>,
        /// Model override
        #[arg(long)]
        model: Option<String>,
        /// Disallowed tools (comma-separated)
        #[arg(long, value_delimiter = ',')]
        disallowed_tools: Vec<String>,
        /// Output states (comma-separated)
        #[arg(long, value_delimiter = ',')]
        output_states: Vec<String>,
        /// Custom state prompt
        #[arg(long)]
        state_prompt: Option<String>,
        /// Convenience: mark mode as read-only (disallow Write/Edit)
        #[arg(long)]
        readonly: bool,
        /// Print JSON for the saved mode
        #[arg(long)]
        json: bool,
    },
    /// Delete a mode
    Delete { name: String },
}

#[derive(Subcommand)]
enum AgentCommands {
    /// List configured agents
    List {
        /// Print JSON instead of plain lines
        #[arg(long)]
        json: bool,
    },
    /// Show an agent definition
    Get {
        name: String,
        /// Print JSON instead of TOML
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ChainCommands {
    /// List configured chains
    List {
        /// Print JSON instead of plain lines
        #[arg(long)]
        json: bool,
    },
    /// Show a chain definition
    Get {
        name: String,
        /// Print JSON instead of TOML
        #[arg(long)]
        json: bool,
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
    let config_path = cli.config.clone();

    match cli.command {
        Some(Commands::Gui) => {
            // Run the main GUI application
            kyco::gui::run_gui(work_dir.clone(), config_path.clone())?;
        }
        Some(Commands::Status { filter }) => {
            cli::status::status_command(&work_dir, config_path.as_ref(), filter).await?;
        }
        Some(Commands::Init { force }) => {
            cli::init::init_command(&work_dir, config_path.clone(), force).await?;
        }
        Some(Commands::Job { command }) => match command {
            JobCommands::List { json } => {
                cli::job::job_list_command(&work_dir, config_path.as_ref(), json)?;
            }
            JobCommands::Get { job_id, json } => {
                cli::job::job_get_command(&work_dir, config_path.as_ref(), job_id, json)?;
            }
            JobCommands::Start {
                file,
                line_start,
                line_end,
                mode,
                prompt,
                agent,
                agents,
                pending,
                force_worktree,
                json,
            } => {
                cli::job::job_start_command(
                    &work_dir,
                    config_path.as_ref(),
                    cli::job::JobStartArgs {
                        file_path: file,
                        line_start,
                        line_end,
                        selected_text: None,
                        mode,
                        prompt,
                        agent,
                        agents,
                        queue: !pending,
                        force_worktree,
                        json,
                    },
                )?;
            }
            JobCommands::Queue { job_id } => {
                cli::job::job_queue_command(&work_dir, config_path.as_ref(), job_id)?;
            }
            JobCommands::Abort { job_id } => {
                cli::job::job_abort_command(&work_dir, config_path.as_ref(), job_id)?;
            }
            JobCommands::Delete {
                job_id,
                cleanup_worktree,
            } => {
                cli::job::job_delete_command(
                    &work_dir,
                    config_path.as_ref(),
                    job_id,
                    cleanup_worktree,
                )?;
            }
            JobCommands::Continue {
                job_id,
                prompt,
                pending,
                json,
            } => {
                cli::job::job_continue_command(
                    &work_dir,
                    config_path.as_ref(),
                    job_id,
                    prompt,
                    !pending,
                    json,
                )?;
            }
            JobCommands::Wait {
                job_id,
                timeout_secs,
                poll_ms,
                json,
            } => {
                cli::job::job_wait_command(
                    &work_dir,
                    config_path.as_ref(),
                    job_id,
                    timeout_secs.map(Duration::from_secs),
                    Duration::from_millis(poll_ms),
                    json,
                )?;
            }
            JobCommands::Output {
                job_id,
                json,
                summary,
                state,
            } => {
                cli::job::job_output_command(
                    &work_dir,
                    config_path.as_ref(),
                    job_id,
                    json,
                    summary,
                    state,
                )?;
            }
        },
        Some(Commands::Mode { command }) => match command {
            ModeCommands::List { json } => {
                cli::mode::mode_list_command(&work_dir, config_path.as_ref(), json)?;
            }
            ModeCommands::Get { name, json } => {
                cli::mode::mode_get_command(&work_dir, config_path.as_ref(), &name, json)?;
            }
            ModeCommands::Set {
                name,
                prompt,
                system_prompt,
                agent,
                aliases,
                session_mode,
                max_turns,
                model,
                disallowed_tools,
                output_states,
                state_prompt,
                readonly,
                json,
            } => {
                cli::mode::mode_set_command(
                    &work_dir,
                    config_path.as_ref(),
                    cli::mode::ModeSetArgs {
                        name,
                        prompt,
                        system_prompt,
                        agent,
                        aliases,
                        session_mode,
                        max_turns,
                        model,
                        disallowed_tools,
                        output_states,
                        state_prompt,
                        readonly,
                        json,
                    },
                )?;
            }
            ModeCommands::Delete { name } => {
                cli::mode::mode_delete_command(&work_dir, config_path.as_ref(), &name)?;
            }
        },
        Some(Commands::Agent { command }) => match command {
            AgentCommands::List { json } => {
                cli::agent::agent_list_command(&work_dir, config_path.as_ref(), json)?;
            }
            AgentCommands::Get { name, json } => {
                cli::agent::agent_get_command(&work_dir, config_path.as_ref(), &name, json)?;
            }
        },
        Some(Commands::Chain { command }) => match command {
            ChainCommands::List { json } => {
                cli::chain::chain_list_command(&work_dir, config_path.as_ref(), json)?;
            }
            ChainCommands::Get { name, json } => {
                cli::chain::chain_get_command(&work_dir, config_path.as_ref(), &name, json)?;
            }
        },
        None => {
            // Default: run the GUI
            kyco::gui::run_gui(work_dir.clone(), config_path.clone())?;
        }
    }

    Ok(())
}
