//! Init command implementation

use anyhow::{bail, Result};
use std::path::Path;
use std::path::PathBuf;
use tracing::info;

use crate::config::INTERNAL_DEFAULTS_TOML;

/// Settings template with documentation (settings section only)
/// The agents, modes, and chains come from INTERNAL_DEFAULTS_TOML
const SETTINGS_TEMPLATE: &str = r#"# KYCo Configuration - Know Your Codebase
# =======================
#
# Jobs are created via IDE extensions (VSCode, JetBrains) that send
# code selections to KYCo's GUI for processing by AI agents.
#
# STRUCTURE:
# - [settings] - Global configuration options
# - [agent.*] - AI backend configurations (claude, codex)
# - [mode.*] - Prompt templates for different task types
# - [chain.*] - Sequential mode execution pipelines
#
# VERSIONING:
# Internal modes/chains/agents have a `version` field. When KYCo updates,
# new versions are automatically merged into your config. To keep your
# customizations, don't modify the version number.

# ============================================================================
# SETTINGS - Global configuration options
# ============================================================================
#
# Available options:
#   max_concurrent_jobs - Maximum number of jobs to run simultaneously (default: 4)
#   auto_run            - Automatically start jobs when found (default: true)
#   use_worktree        - Run jobs in isolated Git worktrees (default: false)
#   max_jobs_per_file   - Max concurrent jobs per file when not using worktrees (default: 1)

[settings]
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
