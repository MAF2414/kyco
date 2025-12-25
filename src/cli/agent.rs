//! Agent discovery commands (read `.kyco/config.toml`).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::Config;

/// Resolve the config path - uses global config (~/.kyco/config.toml) as default,
/// but allows override via --config flag for project-local configs.
fn resolve_config_path(work_dir: &Path, config_override: Option<&PathBuf>) -> PathBuf {
    match config_override {
        Some(p) if p.is_absolute() => p.clone(),
        Some(p) => work_dir.join(p),
        None => Config::global_config_path(), // Use global config as default
    }
}

fn load_or_init_config(_work_dir: &Path, config_override: Option<&PathBuf>) -> Result<Config> {
    // If using default global config, use Config::load() which handles auto-init
    if config_override.is_none() {
        return Config::load();
    }

    // For explicit config override, load from that file
    let config_path = resolve_config_path(_work_dir, config_override);
    if config_path.exists() {
        return Config::from_file(&config_path);
    }

    // Create the specified config with defaults
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let cfg = Config::with_defaults();
    let toml = toml::to_string_pretty(&cfg).context("Failed to serialize default config")?;
    std::fs::write(&config_path, toml)
        .with_context(|| format!("Failed to write {}", config_path.display()))?;

    Ok(cfg)
}

pub fn agent_list_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    json: bool,
) -> Result<()> {
    let cfg = load_or_init_config(work_dir, config_override)?;
    let mut names: Vec<String> = cfg.agent.keys().cloned().collect();
    names.sort();

    if json {
        println!("{}", serde_json::to_string_pretty(&names)?);
    } else {
        for name in names {
            println!("{name}");
        }
    }
    Ok(())
}

pub fn agent_get_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    name: &str,
    json: bool,
) -> Result<()> {
    let cfg = load_or_init_config(work_dir, config_override)?;
    let Some(agent) = cfg.agent.get(name) else {
        anyhow::bail!("Agent not found: {}", name);
    };

    if json {
        println!("{}", serde_json::to_string_pretty(agent)?);
    } else {
        println!("{}", toml::to_string_pretty(agent)?);
    }
    Ok(())
}
