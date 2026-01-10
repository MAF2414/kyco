//! Init command implementation

use anyhow::{bail, Result};
use std::path::Path;
use std::path::PathBuf;
use tracing::info;

use crate::config::INTERNAL_DEFAULTS_TOML;

/// Settings template with documentation (settings section only)
/// The agents come from INTERNAL_DEFAULTS_TOML
const SETTINGS_TEMPLATE: &str = r#"# KYCo Configuration - Know Your Codebase
# =======================
#
# Jobs are created via IDE extensions (VSCode, JetBrains) that send
# code selections to KYCo's GUI for processing by AI agents.
#
# STRUCTURE:
# - [settings] - Global configuration options
# - [agent.*] - AI backend configurations (claude, codex)
# - [chain.*] - Sequential skill execution pipelines (user-defined)
#
# SKILLS:
# Skills are loaded from SKILL.md files (not from this config):
# - Project-local: .claude/skills/<name>/SKILL.md
# - Global: ~/.kyco/skills/<name>/SKILL.md
# Create with: kyco skill create <name> --description "..."
#
# VERSIONING:
# Internal agents have a `version` field. When KYCo updates,
# new versions are automatically merged into your config.

# ============================================================================
# SETTINGS - Global configuration options
# ============================================================================
#
# Available options:
#   max_concurrent_jobs - Max jobs PER AGENT (4 means 4 Claude + 4 Codex, default: 4)
#   auto_run            - Automatically start jobs when found (default: true)
#   use_worktree        - Run jobs in isolated Git worktrees (default: false)
#   max_jobs_per_file   - Max concurrent jobs per file when not using worktrees (default: 1)

[settings]
# Per-agent limit: 4 means up to 4 Claude AND 4 Codex jobs can run simultaneously
max_concurrent_jobs = 4
auto_run = true
use_worktree = false
# Maximum jobs per file (only when use_worktree = false)
# Set to 1 to prevent agents from overwriting each other's changes
# When a job is blocked, it shows as "Blocked" in the GUI with the blocking job ID
max_jobs_per_file = 1

# GUI / IDE extension communication (local HTTP server)
[settings.gui]
http_port = 9876
# Optional: Shared secret for IDE extension requests (sent as `X-KYCO-Token`)
# Leave empty to disable auth (recommended for local development)
http_token = ""

# Claude Agent SDK plugins (local allowlist)
#
# Security note: plugins are Node.js code that runs inside the KYCO bridge process.
# Only add trusted plugin directories here.
[settings.claude]
allowed_plugin_paths = []

# Orchestrator settings for external CLI sessions
# The orchestrator launches a CLI agent (claude/codex) in Terminal.app
# to help you run batch KYCo jobs interactively.
[settings.gui.orchestrator]
# CLI agent to use: "claude" or "codex"
cli_agent = "claude"
# Custom CLI command (optional). Use {prompt_file} as placeholder.
# If empty, auto-generates based on cli_agent.
# Examples:
#   "claude --append-system-prompt \"$(cat {prompt_file})\""
#   "codex \"$(cat {prompt_file})\""
#   "aider --model gpt-4"
cli_command = ""
# System prompt for the orchestrator (instructs the CLI agent on how to use KYCo)
system_prompt = """
You are an interactive KYCo Orchestrator running in the user's workspace.

Your job is to help the user run KYCo jobs (skills/chains) safely and iteratively.

Rules
- Do NOT directly edit repository files yourself. Use KYCo jobs so the user can review diffs in the KYCo GUI.
- Use the `Bash` tool to run `kyco ...` commands.
- Before starting a large batch of jobs, confirm the plan with the user.

Discovery
- List available agents: `kyco agent list`
- List available skills: `kyco skill list`
- List available chains: `kyco chain list`

Skill Registry (~50,000 community skills)
- Search skills: `kyco skill search "<query>"` (e.g., "code review", "refactor", "test")
- Show skill details: `kyco skill info <author>/<name>`
- Install from registry: `kyco skill install-from-registry <author>/<name>`

Job lifecycle (GUI must be running)
- Start a job: `kyco job start --file <path> --skill <skill_or_chain> --prompt "<what to do>"`
- Abort a job: `kyco job abort <job_id>`
- Wait for completion: `kyco job wait <job_id>`
- Get output: `kyco job output <job_id>` (or --summary, --state)
- Continue session: `kyco job continue <job_id> --prompt "<follow-up>"`

Batch job creation
- Use --pending flag to create jobs without auto-queueing (review first in GUI)
- For multi-agent comparison: --agents claude,codex creates parallel jobs
"""

"#;

/// Build the complete default configuration by combining settings template
/// with internal defaults (agents, modes, chains).
pub fn build_default_config() -> String {
    format!("{}{}", SETTINGS_TEMPLATE, INTERNAL_DEFAULTS_TOML)
}

/// Ensures the global config file exists (~/.kyco/config.toml), creating it if missing.
/// This is called automatically when a new workspace is registered.
/// Returns true if config was created, false if it already existed or couldn't be created.
pub fn ensure_config_exists(_workspace_path: &Path) -> bool {
    let config_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".kyco");
    let config_path = config_dir.join("config.toml");

    if config_path.exists() {
        return false;
    }

    if !config_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&config_dir) {
            info!("Failed to create ~/.kyco directory: {}", e);
            return false;
        }
    }

    let config_content = build_default_config();
    if let Err(e) = std::fs::write(&config_path, config_content) {
        info!("Failed to write default config: {}", e);
        return false;
    }

    info!("Auto-initialized global config: {}", config_path.display());
    true
}

/// Initialize a new KYCo configuration
/// By default creates the global config at ~/.kyco/config.toml
/// Use --config to specify a custom path
pub async fn init_command(
    _work_dir: &Path,
    config_path: Option<PathBuf>,
    force: bool,
) -> Result<()> {
    let config_path = config_path.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".kyco")
            .join("config.toml")
    });

    if config_path.exists() && !force {
        bail!(
            "Configuration already exists: {}\nUse --force to overwrite.",
            config_path.display()
        );
    }

    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let config_content = build_default_config();
    std::fs::write(&config_path, config_content)?;
    println!("Created: {}", config_path.display());

    Ok(())
}
