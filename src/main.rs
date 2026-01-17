use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;

use kyco::cli;

mod commands;
use commands::{
    AgentCommands, ChainCommands, Commands, FindingCommands, ImportCommands, JobCommands, ModeCommands,
    ProjectCommands, ScopeCommands, SkillCommands,
};

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
                project,
                finding,
                status,
                state,
                limit,
                search,
                skill,
            } => {
                cli::job::job_list_command(
                    &work_dir,
                    config_path.as_ref(),
                    json,
                    project.as_deref(),
                    finding.as_deref(),
                    status.as_deref(),
                    state.as_deref(),
                    limit,
                    search.as_deref(),
                    skill.as_deref(), // CLI uses --skill, internally still called mode
                )?;
            }
            JobCommands::Get { job_id, json } => {
                cli::job::job_get_command(&work_dir, config_path.as_ref(), job_id, json)?;
            }
            JobCommands::Start {
                file,
                input,
                batch,
                line_start,
                line_end,
                skill,
                prompt,
                project,
                finding,
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
                        input,
                        batch,
                        line_start,
                        line_end,
                        selected_text: None,
                        mode: skill, // CLI uses --skill, internally still called mode
                        prompt,
                        bugbounty_project_id: project,
                        bugbounty_finding_ids: finding,
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
                next_context,
                findings,
                flow,
                artifacts,
                summary,
                state,
            } => {
                cli::job::job_output_command(
                    &work_dir,
                    config_path.as_ref(),
                    job_id,
                    json,
                    next_context,
                    findings,
                    flow,
                    artifacts,
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
        Some(Commands::Skill { command }) => match command {
            SkillCommands::List { json, agent } => {
                cli::skill::skill_list_command(&work_dir, json, agent.as_deref())?;
            }
            SkillCommands::Get { name, json } => {
                cli::skill::skill_get_command(&work_dir, &name, json)?;
            }
            SkillCommands::Create {
                name,
                description,
                agent,
                global,
                json,
            } => {
                cli::skill::skill_create_command(
                    &work_dir,
                    cli::skill::SkillCreateArgs {
                        name,
                        description,
                        agent,
                        global,
                        json,
                    },
                )?;
            }
            SkillCommands::Delete { name, agent, global } => {
                cli::skill::skill_delete_command(&work_dir, &name, agent.as_deref(), global)?;
            }
            SkillCommands::Path { name, agent } => {
                cli::skill::skill_path_command(&work_dir, &name, agent.as_deref())?;
            }
            SkillCommands::Install { name, agent, global } => {
                cli::skill::skill_install_command(&work_dir, &name, agent.as_deref(), global)?;
            }
            // Registry commands (search & install from community)
            SkillCommands::Search { query, limit, json } => {
                cli::skill::skill_search_command(&query, limit, json)?;
            }
            SkillCommands::Info { name, json } => {
                cli::skill::skill_info_command(&name, json)?;
            }
            SkillCommands::InstallFromRegistry { name, agent, global } => {
                cli::skill::skill_install_from_registry_command(
                    &work_dir,
                    &name,
                    agent.as_deref(),
                    global,
                )?;
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
        Some(Commands::Finding { command }) => match command {
            FindingCommands::List {
                project,
                status,
                severity,
                search,
                json,
            } => {
                cli::finding::list(project, status, severity, search, json)?;
            }
            FindingCommands::Show { id, json } => {
                cli::finding::show(&id, json)?;
            }
            FindingCommands::Create {
                title,
                project,
                severity,
                attack_scenario,
                preconditions,
                impact,
                confidence,
                cwe,
                assets,
                write_notes,
                json,
            } => {
                cli::finding::create(
                    &work_dir,
                    &title,
                    &project,
                    severity,
                    attack_scenario,
                    preconditions,
                    impact,
                    confidence,
                    cwe,
                    assets,
                    write_notes,
                    json,
                )?;
            }
            FindingCommands::SetStatus { id, status } => {
                cli::finding::set_status(&id, &status)?;
            }
            FindingCommands::Fp { id, reason } => {
                cli::finding::mark_fp(&id, &reason)?;
            }
            FindingCommands::Delete { id, yes } => {
                cli::finding::delete(&id, yes)?;
            }
            FindingCommands::Export { id, format, output } => {
                cli::finding::export(&id, &format, output)?;
            }
            FindingCommands::ExportNotes {
                id,
                dry_run,
                force,
                json,
            } => {
                cli::finding::export_notes(&work_dir, &id, dry_run, force, json)?;
            }
            FindingCommands::Import { file, project, format, json } => {
                cli::finding::import(&file, &project, &format, json)?;
            }
            FindingCommands::ImportNotes {
                project,
                dry_run,
                json,
            } => {
                cli::finding::import_notes(&work_dir, &project, dry_run, json)?;
            }
            FindingCommands::ExtractFromJob { job_id, project, json } => {
                cli::finding::extract_from_job(job_id, project, json)?;
            }
            FindingCommands::Link { finding, job, link_type } => {
                cli::finding::link_job(&finding, &job, &link_type)?;
            }
            FindingCommands::Unlink { finding, job } => {
                cli::finding::unlink_job(&finding, &job)?;
            }
        },
        Some(Commands::Import { command }) => match command {
            ImportCommands::Semgrep {
                file,
                project,
                create_jobs,
                queue,
                skill,
                agent,
                agents,
                json,
            } => {
                cli::import::import_tool(
                    &work_dir,
                    config_path.as_ref(),
                    "semgrep",
                    &file,
                    project,
                    "semgrep",
                    create_jobs,
                    queue,
                    &skill,
                    agent,
                    agents,
                    json,
                )?;
            }
            ImportCommands::Codeql {
                file,
                project,
                create_jobs,
                queue,
                skill,
                agent,
                agents,
                json,
            } => {
                cli::import::import_tool(
                    &work_dir,
                    config_path.as_ref(),
                    "codeql",
                    &file,
                    project,
                    "sarif",
                    create_jobs,
                    queue,
                    &skill,
                    agent,
                    agents,
                    json,
                )?;
            }
            ImportCommands::Sarif {
                file,
                project,
                create_jobs,
                queue,
                skill,
                agent,
                agents,
                json,
            } => {
                cli::import::import_tool(
                    &work_dir,
                    config_path.as_ref(),
                    "sarif",
                    &file,
                    project,
                    "sarif",
                    create_jobs,
                    queue,
                    &skill,
                    agent,
                    agents,
                    json,
                )?;
            }
            ImportCommands::Snyk {
                file,
                project,
                create_jobs,
                queue,
                skill,
                agent,
                agents,
                json,
            } => {
                cli::import::import_tool(
                    &work_dir,
                    config_path.as_ref(),
                    "snyk",
                    &file,
                    project,
                    "snyk",
                    create_jobs,
                    queue,
                    &skill,
                    agent,
                    agents,
                    json,
                )?;
            }
            ImportCommands::Nuclei {
                file,
                project,
                create_jobs,
                queue,
                skill,
                agent,
                agents,
                json,
            } => {
                cli::import::import_tool(
                    &work_dir,
                    config_path.as_ref(),
                    "nuclei",
                    &file,
                    project,
                    "nuclei",
                    create_jobs,
                    queue,
                    &skill,
                    agent,
                    agents,
                    json,
                )?;
            }
            ImportCommands::Auto {
                file,
                project,
                create_jobs,
                queue,
                skill,
                agent,
                agents,
                json,
            } => {
                cli::import::import_tool(
                    &work_dir,
                    config_path.as_ref(),
                    "auto",
                    &file,
                    project,
                    "auto",
                    create_jobs,
                    queue,
                    &skill,
                    agent,
                    agents,
                    json,
                )?;
            }
        },
        Some(Commands::Project { command }) => match command {
            ProjectCommands::List { platform, json } => {
                cli::project::list(platform, json)?;
            }
            ProjectCommands::Show { id, json } => {
                cli::project::show(&id, json)?;
            }
            ProjectCommands::Discover { path, dry_run } => {
                cli::project::discover(path, dry_run)?;
            }
            ProjectCommands::Select { id } => {
                cli::project::select(&id)?;
            }
            ProjectCommands::Init { id, root_path, platform } => {
                cli::project::init(&id, &root_path, platform)?;
            }
            ProjectCommands::Delete { id, yes } => {
                cli::project::delete(&id, yes)?;
            }
            ProjectCommands::Overview {
                project,
                output,
                update_global,
                json,
            } => {
                cli::project::overview(project, output, update_global, json)?;
            }
        },
        Some(Commands::Scope { command }) => match command {
            ScopeCommands::Show { project, json } => {
                cli::scope::show(project, json)?;
            }
            ScopeCommands::Check { url, project } => {
                cli::scope::check(&url, project)?;
            }
            ScopeCommands::Policy { project, json } => {
                cli::scope::policy(project, json)?;
            }
        },
        None => {
            kyco::gui::run_gui(work_dir.clone(), config_path.clone())?;
        }
    }

    Ok(())
}
