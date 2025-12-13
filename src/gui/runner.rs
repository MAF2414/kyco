//! GUI runner - launches the main kyco GUI application
//!
//! The GUI receives selections from IDE extensions via HTTP server

use anyhow::Result;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily, IconData};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

use super::app::KycoApp;
use super::executor::{start_executor, ExecutorEvent};
use super::http_server::{start_http_server, BatchRequest, SelectionRequest};
use crate::agent::BridgeProcess;
use crate::config::Config;
use crate::job::JobManager;

/// Load the KYCo app icon from embedded PNG
fn load_kyco_icon() -> IconData {
    // Embed the logo at compile time
    const LOGO_BYTES: &[u8] = include_bytes!("../assets/Logo.png");

    // Decode PNG to RGBA
    let img = image::load_from_memory(LOGO_BYTES)
        .expect("Failed to decode embedded logo")
        .into_rgba8();

    let (width, height) = img.dimensions();
    let rgba = img.into_raw();

    IconData {
        rgba,
        width,
        height,
    }
}

/// Run the main GUI application
pub fn run_gui(work_dir: PathBuf, config_override: Option<PathBuf>) -> Result<()> {
    let work_dir = if work_dir.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        work_dir
    };

    let config_override_path = config_override.map(|p| if p.is_absolute() { p } else { work_dir.join(p) });
    let config_override_provided = config_override_path.is_some();
    let config_path = config_override_path.unwrap_or_else(|| work_dir.join(".kyco").join("config.toml"));

    // Load config
    let config_was_present = config_path.exists();
    let config = if config_was_present {
        match Config::from_file(&config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                warn!(
                    "[kyco] Failed to parse config ({}): {}. Falling back to defaults.",
                    config_path.display(),
                    e
                );
                Config::with_defaults()
            }
        }
    } else if config_override_provided {
        // User explicitly requested a config path: create it if missing.
        let cfg = Config::with_defaults();
        // Note: http_token is intentionally left empty by default for local development.
        // Users can set it manually in config if they need auth.

        if let Some(parent) = config_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!(
                    "[kyco] Failed to create config directory ({}): {}",
                    parent.display(),
                    e
                );
            }
        }

        match toml::to_string_pretty(&cfg) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&config_path, content) {
                    warn!(
                        "[kyco] Failed to write config ({}): {}",
                        config_path.display(),
                        e
                    );
                }
            }
            Err(e) => {
                warn!(
                    "[kyco] Failed to serialize default config for {}: {}",
                    config_path.display(),
                    e
                );
            }
        }

        cfg
    } else {
        // Create a config on first GUI launch so IDE extension auth works out-of-the-box.
        Config::from_dir(&work_dir).unwrap_or_else(|e| {
            warn!(
                "[kyco] Failed to initialize config in {}: {}. Falling back to defaults.",
                work_dir.display(),
                e
            );
            Config::with_defaults()
        })
    };

    // Note: http_token is intentionally left empty by default.
    // Auth is only enforced when http_token is explicitly set in config.
    // This avoids the "chicken-and-egg" problem with new workspaces.

    let config_exists = config_path.exists();
    if !config_was_present && config_exists {
        info!("[kyco] Created {}", config_path.display());
    }

    // Load job manager
    let job_manager = Arc::new(Mutex::new(JobManager::load(&work_dir).unwrap_or_else(|_| JobManager::new(&work_dir))));

    // Start SDK Bridge server (Node.js sidecar)
    // This provides Claude Agent SDK and Codex SDK functionality
    let _bridge_process = BridgeProcess::spawn()?;
    info!("[kyco] SDK Bridge server ready");

    info!(
        "[kyco] Starting GUI with HTTP server on port {}...",
        config.settings.gui.http_port
    );

    // Create channel for HTTP server -> GUI communication (single selection)
    let (http_tx, http_rx): (mpsc::Sender<SelectionRequest>, mpsc::Receiver<SelectionRequest>) = mpsc::channel();

    // Create channel for batch requests from IDE
    let (batch_tx, batch_rx): (mpsc::Sender<BatchRequest>, mpsc::Receiver<BatchRequest>) = mpsc::channel();

    // Create channel for executor -> GUI communication
    let (executor_tx, executor_rx): (mpsc::Sender<ExecutorEvent>, mpsc::Receiver<ExecutorEvent>) = mpsc::channel();

    // Start HTTP server in background (handles both /selection and /batch)
    start_http_server(
        http_tx,
        batch_tx,
        config.settings.gui.http_port,
        Some(config.settings.gui.http_token.clone()).filter(|t| !t.trim().is_empty()),
    );

    // Create shared max_concurrent_jobs so GUI can update it at runtime
    let max_concurrent_jobs = Arc::new(AtomicUsize::new(config.settings.max_concurrent_jobs));

    // Start job executor in background
    start_executor(
        work_dir.clone(),
        config.clone(),
        job_manager.clone(),
        executor_tx,
        Arc::clone(&max_concurrent_jobs),
    );

    // Create app icon
    let icon = load_kyco_icon();

    // Run GUI
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 600.0])
            .with_min_inner_size([800.0, 400.0])
            .with_decorations(true)
            .with_resizable(true)
            .with_icon(std::sync::Arc::new(icon)),
        centered: true,
        ..Default::default()
    };

    let app = KycoApp::new(work_dir, config, config_exists, job_manager, http_rx, batch_rx, executor_rx, max_concurrent_jobs);

    eframe::run_native("kyco", options, Box::new(|cc| {
        configure_fonts(&cc.egui_ctx);
        Ok(Box::new(app))
    }))
        .map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e))?;

    Ok(())
}

/// Configure fonts with system fallbacks for Unicode symbols and emojis
fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Platform-specific font configurations
    // Each entry: (name, path) - will be tried in order
    #[cfg(target_os = "macos")]
    let font_fallbacks: &[(&str, &str)] = &[
        ("symbols", "/System/Library/Fonts/Apple Symbols.ttf"),
        ("arial_unicode", "/System/Library/Fonts/Supplemental/Arial Unicode.ttf"),
    ];

    #[cfg(target_os = "windows")]
    let font_fallbacks: &[(&str, &str)] = &[
        ("symbols", "C:\\Windows\\Fonts\\seguisym.ttf"),
        ("segoe", "C:\\Windows\\Fonts\\segoeui.ttf"),
    ];

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let font_fallbacks: &[(&str, &str)] = &[
        ("symbols", "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf"),
        ("symbols_alt", "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf"),
        ("dejavu", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
    ];

    // Load all available fallback fonts
    for (name, path) in font_fallbacks {
        if let Ok(font_data) = std::fs::read(path) {
            fonts.font_data.insert(
                (*name).to_owned(),
                FontData::from_owned(font_data).into(),
            );

            // Add as fallback for both font families
            if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
                family.push((*name).to_owned());
            }
            if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
                family.push((*name).to_owned());
            }

            info!("[kyco] Loaded fallback font '{}' from: {}", name, path);
        }
    }

    ctx.set_fonts(fonts);
}
