//! Mode CRUD commands (edit `.kyco/config.toml`).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::{Config, ModeConfig, ModeSessionType};

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

fn load_or_init_config(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
) -> Result<(Config, PathBuf)> {
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

pub fn mode_list_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    json: bool,
) -> Result<()> {
    let (cfg, _) = load_or_init_config(work_dir, config_override)?;
    let mut names: Vec<String> = cfg.mode.keys().cloned().collect();
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

pub fn mode_get_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    name: &str,
    json: bool,
) -> Result<()> {
    let (cfg, _) = load_or_init_config(work_dir, config_override)?;
    let Some(mode) = cfg.mode.get(name) else {
        anyhow::bail!("Mode not found: {}", name);
    };

    if json {
        println!("{}", serde_json::to_string_pretty(mode)?);
    } else {
        println!("{}", toml::to_string_pretty(mode)?);
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct ModeSetArgs {
    pub name: String,
    pub prompt: Option<String>,
    pub system_prompt: Option<String>,
    pub agent: Option<String>,
    pub aliases: Vec<String>,
    pub session_mode: Option<String>,
    pub max_turns: Option<u32>,
    pub model: Option<String>,
    pub disallowed_tools: Vec<String>,
    pub output_states: Vec<String>,
    pub state_prompt: Option<String>,
    pub readonly: bool,
    pub json: bool,
}

pub fn mode_set_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    args: ModeSetArgs,
) -> Result<()> {
    let (mut cfg, config_path) = load_or_init_config(work_dir, config_override)?;

    let mut mode = cfg.mode.remove(&args.name).unwrap_or_else(|| ModeConfig {
        version: 0, // User-created modes start at version 0
        agent: None,
        target_default: None,
        scope_default: None,
        prompt: None,
        system_prompt: None,
        session_mode: ModeSessionType::Oneshot,
        max_turns: 0,
        model: None,
        disallowed_tools: Vec::new(),
        claude: None,
        codex: None,
        aliases: Vec::new(),
        output_states: Vec::new(),
        state_prompt: None,
        allowed_tools: Vec::new(),
    });

    if let Some(prompt) = args.prompt {
        mode.prompt = Some(prompt);
    }
    if let Some(system_prompt) = args.system_prompt {
        mode.system_prompt = Some(system_prompt);
    }
    if let Some(agent) = args.agent {
        mode.agent = Some(agent);
    }
    if !args.aliases.is_empty() {
        mode.aliases = args.aliases;
    }
    if let Some(session_mode) = args.session_mode.as_deref() {
        mode.session_mode = match session_mode {
            "session" => ModeSessionType::Session,
            _ => ModeSessionType::Oneshot,
        };
    }
    if let Some(max_turns) = args.max_turns {
        mode.max_turns = max_turns;
    }
    if let Some(model) = args.model {
        mode.model = Some(model);
    }

    if args.readonly {
        mode.disallowed_tools = vec!["Write".to_string(), "Edit".to_string()];
    } else if !args.disallowed_tools.is_empty() {
        mode.disallowed_tools = args.disallowed_tools;
    }

    if !args.output_states.is_empty() {
        mode.output_states = args.output_states;
    }
    if let Some(state_prompt) = args.state_prompt {
        mode.state_prompt = Some(state_prompt);
    }

    cfg.mode.insert(args.name.clone(), mode.clone());
    save_config(&cfg, &config_path)?;
    notify_gui_config_changed(&cfg);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&mode)?);
    } else {
        println!("Mode saved: {}", args.name);
    }
    Ok(())
}

pub fn mode_delete_command(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
    name: &str,
) -> Result<()> {
    let (mut cfg, config_path) = load_or_init_config(work_dir, config_override)?;
    if cfg.mode.remove(name).is_none() {
        anyhow::bail!("Mode not found: {}", name);
    }
    save_config(&cfg, &config_path)?;
    notify_gui_config_changed(&cfg);
    println!("Mode deleted: {}", name);
    Ok(())
}
