use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;

use kyco::cli;

mod commands;
use commands::{AgentCommands, ChainCommands, Commands, JobCommands, ModeCommands};

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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .init();

    let work_dir = cli.path.unwrap_or_else(|| PathBuf::from("."));
    let config_path = cli.config.clone();

    match cli.command {
        Some(Commands::Gui) => {
            kyco::gui::run_gui(work_dir.clone(), config_path.clone())?;
        }
        Some(Commands::Status { filter }) => {
            cli::status::status_command(&work_dir, config_path.as_ref(), filter).await?;
        }
        Some(Commands::Init { force }) => {
            cli::init::init_command(&work_dir, config_path.clone(), force).await?;
        }
        Some(Commands::Job { command }) => match command {
            JobCommands::List {
                json,
                status,
                limit,
                search,
                mode,
            } => {
                cli::job::job_list_command(
                    &work_dir,
                    config_path.as_ref(),
                    json,
                    status.as_deref(),
                    limit,
                    search.as_deref(),
                    mode.as_deref(),
                )?;
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
            JobCommands::Kill { job_id } => {
                cli::job::job_kill_command(&work_dir, config_path.as_ref(), job_id)?;
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
            JobCommands::Merge { job_id, message } => {
                cli::job::job_merge_command(&work_dir, config_path.as_ref(), job_id, message)?;
            }
            JobCommands::Reject { job_id } => {
                cli::job::job_reject_command(&work_dir, config_path.as_ref(), job_id)?;
            }
            JobCommands::Restart { job_id } => {
                cli::job::job_restart_command(&work_dir, config_path.as_ref(), job_id)?;
            }
            JobCommands::Diff { job_id, json } => {
                cli::job::job_diff_command(&work_dir, config_path.as_ref(), job_id, json)?;
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
            ChainCommands::Set {
                name,
                description,
                steps,
                stop_on_failure,
                no_stop_on_failure,
                pass_full_response,
                no_pass_full_response,
                max_loops,
                use_worktree,
                no_use_worktree,
                json,
            } => {
                cli::chain::chain_set_command(
                    &work_dir,
                    config_path.as_ref(),
                    cli::chain::ChainSetArgs {
                        name,
                        description,
                        steps,
                        stop_on_failure: if stop_on_failure {
                            Some(true)
                        } else if no_stop_on_failure {
                            Some(false)
                        } else {
                            None
                        },
                        pass_full_response: if pass_full_response {
                            Some(true)
                        } else if no_pass_full_response {
                            Some(false)
                        } else {
                            None
                        },
                        max_loops,
                        use_worktree: if use_worktree {
                            Some(true)
                        } else if no_use_worktree {
                            Some(false)
                        } else {
                            None
                        },
                        json,
                    },
                )?;
            }
            ChainCommands::Delete { name } => {
                cli::chain::chain_delete_command(&work_dir, config_path.as_ref(), &name)?;
            }
        },
        None => {
            kyco::gui::run_gui(work_dir.clone(), config_path.clone())?;
        }
    }

    Ok(())
}
