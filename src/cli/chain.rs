//! Chain CRUD commands (edit `.kyco/config.toml`).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::{ChainStep, Config, ModeChain};

const AUTH_HEADER: &str = "X-KYCO-Token";

/// Resolve the config path - uses global config (~/.kyco/config.toml) as default,
/// but allows override via --config flag for project-local configs.
fn resolve_config_path(work_dir: &Path, config_override: Option<&PathBuf>) -> PathBuf {
    match config_override {
        Some(p) if p.is_absolute() => p.clone(),
        Some(p) => work_dir.join(p),
        None => Config::global_config_path(),
    }
}

/// Notify running GUI to reload config immediately (best-effort, fails silently).
fn notify_gui_config_changed(config: &Config) {
    let port = config.settings.gui.http_port;
    let token = &config.settings.gui.http_token;
    let url = format!("http://127.0.0.1:{port}/ctl/config/reload");

    let mut req = ureq::post(&url).set("Content-Type", "application/json");
    if !token.trim().is_empty() {
        req = req.set(AUTH_HEADER, token);
    }

    let _ = req.send_string("{}");
}

fn load_or_init_config(work_dir: &Path, config_override: Option<&PathBuf>) -> Result<(Config, PathBuf)> {
    let config_path = resolve_config_path(work_dir, config_override);

    // If using default global config, use Config::load() which handles auto-init
    if config_override.is_none() {
        let cfg = Config::load()?;
        return Ok((cfg, config_path));
    }

    if config_path.exists() {
        let cfg = Config::from_file(&config_path)?;
        return Ok((cfg, config_path));
    }

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let cfg = Config::with_defaults();
    let toml = toml::to_string_pretty(&cfg).context("Failed to serialize default config")?;
    std::fs::write(&config_path, toml)
        .with_context(|| format!("Failed to write {}", config_path.display()))?;

    Ok((cfg, config_path))
}

fn save_config(config: &Config, config_path: &Path) -> Result<()> {
    config.save_to_file(config_path)
}

pub fn chain_list_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    json: bool,
) -> Result<()> {
    let (cfg, _) = load_or_init_config(work_dir, config_override)?;
    let mut names: Vec<String> = cfg.chain.keys().cloned().collect();
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

pub fn chain_get_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    name: &str,
    json: bool,
) -> Result<()> {
    let (cfg, _) = load_or_init_config(work_dir, config_override)?;
    let Some(chain) = cfg.chain.get(name) else {
        anyhow::bail!("Chain not found: {}", name);
    };

    if json {
        println!("{}", serde_json::to_string_pretty(chain)?);
    } else {
        println!("{}", toml::to_string_pretty(chain)?);
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct ChainSetArgs {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<String>,
    pub stop_on_failure: Option<bool>,
    pub pass_full_response: Option<bool>,
    pub max_loops: Option<u32>,
    pub use_worktree: Option<bool>,
    pub json: bool,
}

pub fn chain_set_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    args: ChainSetArgs,
) -> Result<()> {
    let (mut cfg, config_path) = load_or_init_config(work_dir, config_override)?;

    let mut chain = cfg.chain.remove(&args.name).unwrap_or_else(|| ModeChain {
        version: 0,
        description: None,
        states: Vec::new(),
        steps: Vec::new(),
        stop_on_failure: true,
        pass_full_response: true,
        max_loops: 1,
        use_worktree: None,
    });

    if let Some(description) = args.description {
        chain.description = Some(description);
    }

    // If steps provided, replace the entire steps array with simple steps
    if !args.steps.is_empty() {
        chain.steps = args
            .steps
            .into_iter()
            .map(|skill| ChainStep {
                skill,
                trigger_on: None,
                skip_on: None,
                agent: None,
                inject_context: None,
                loop_to: None,
            })
            .collect();
    }

    if let Some(stop_on_failure) = args.stop_on_failure {
        chain.stop_on_failure = stop_on_failure;
    }
    if let Some(pass_full_response) = args.pass_full_response {
        chain.pass_full_response = pass_full_response;
    }
    if let Some(max_loops) = args.max_loops {
        chain.max_loops = max_loops;
    }
    if args.use_worktree.is_some() {
        chain.use_worktree = args.use_worktree;
    }

    // Validate that steps are not empty
    if chain.steps.is_empty() {
        anyhow::bail!("Chain must have at least one step. Use --steps mode1,mode2,...");
    }

    cfg.chain.insert(args.name.clone(), chain.clone());
    save_config(&cfg, &config_path)?;
    notify_gui_config_changed(&cfg);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&chain)?);
    } else {
        println!("Chain saved: {}", args.name);
    }
    Ok(())
}

pub fn chain_delete_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    name: &str,
) -> Result<()> {
    let (mut cfg, config_path) = load_or_init_config(work_dir, config_override)?;
    if cfg.chain.remove(name).is_none() {
        anyhow::bail!("Chain not found: {}", name);
    }
    save_config(&cfg, &config_path)?;
    notify_gui_config_changed(&cfg);
    println!("Chain deleted: {}", name);
    Ok(())
}
