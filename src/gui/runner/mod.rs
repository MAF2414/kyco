//! GUI runner - launches the main kyco GUI application
//!
//! The GUI receives selections from IDE extensions via HTTP server

mod fonts;

use anyhow::Result;
use eframe::egui::{self, IconData};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

use super::app::KycoApp;
use super::executor::{ExecutorEvent, start_executor};
use super::http_server::{BatchRequest, ControlApiState, SelectionRequest, start_http_server};
use crate::LogEvent;
use crate::agent::BridgeProcess;
use crate::config::Config;
use crate::job::{GroupManager, JobManager};

use fonts::configure_fonts;

fn start_config_watch_thread(
    config_path: PathBuf,
    config: Arc<RwLock<Config>>,
    max_concurrent_jobs: Arc<AtomicUsize>,
    event_tx: mpsc::Sender<ExecutorEvent>,
) {
    thread::spawn(move || {
        let mut last_modified = std::fs::metadata(&config_path)
            .and_then(|m| m.modified())
            .ok();

        loop {
            thread::sleep(Duration::from_millis(500));

            let modified = match std::fs::metadata(&config_path).and_then(|m| m.modified()) {
                Ok(m) => Some(m),
                Err(_) => None,
            };

            if modified.is_none() || modified == last_modified {
                continue;
            }

            // Debounce slightly to avoid reading partially-written files.
            thread::sleep(Duration::from_millis(50));

            match Config::from_file(&config_path) {
                Ok(new_config) => {
                    max_concurrent_jobs
                        .store(new_config.settings.max_concurrent_jobs, Ordering::Relaxed);

                    if let Ok(mut guard) = config.write() {
                        *guard = new_config;
                    }

                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::system(format!(
                        "Reloaded config from {}",
                        config_path.display()
                    ))));
                    last_modified = modified;
                }
                Err(e) => {
                    let _ = event_tx.send(ExecutorEvent::Log(LogEvent::error(format!(
                        "Failed to reload config ({}): {}",
                        config_path.display(),
                        e
                    ))));
                    // Update last_modified to prevent infinite retry loop.
                    // User must save the file again after fixing the error.
                    last_modified = modified;
                }
            }
        }
    });
}

/// Load the KYCo app icon from embedded PNG
fn load_kyco_icon() -> IconData {
    const LOGO_BYTES: &[u8] = include_bytes!("../../assets/Logo.png");

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
    let work_dir = match work_dir.canonicalize() {
        Ok(abs) => abs,
        Err(_) => {
            if work_dir.is_absolute() {
                work_dir
            } else if let Ok(cwd) = std::env::current_dir() {
                cwd.join(work_dir)
            } else {
                work_dir
            }
        }
    };

    // Use global config by default (~/.kyco/config.toml), allow override with --config
    let config_path = match config_override {
        Some(p) if p.is_absolute() => p,
        Some(p) => work_dir.join(p),
        None => Config::global_config_path(),
    };

    // Load config (auto-creates global config if missing)
    let config_was_present = config_path.exists();
    let config = match Config::from_file(&config_path) {
        Ok(cfg) => cfg,
        Err(_) if !config_was_present => {
            // Config doesn't exist - use Config::load() which auto-inits global config
            Config::load().unwrap_or_else(|e| {
                warn!(
                    "[kyco] Failed to initialize config: {}. Falling back to defaults.",
                    e
                );
                Config::with_defaults()
            })
        }
        Err(e) => {
            warn!(
                "[kyco] Failed to parse config ({}): {}. Falling back to defaults.",
                config_path.display(),
                e
            );
            Config::with_defaults()
        }
    };

    // Global config is auto-created by Config::load(), so it always exists after loading
    let config_exists = config_path.exists();
    if !config_was_present && config_exists {
        info!("[kyco] Created {}", config_path.display());
    }

    let config = Arc::new(RwLock::new(config));

    let job_manager = Arc::new(Mutex::new(
        JobManager::load(&work_dir).unwrap_or_else(|_| JobManager::new(&work_dir)),
    ));
    let group_manager = Arc::new(Mutex::new(GroupManager::new()));

    // Start SDK Bridge server (Node.js sidecar)
    // This provides Claude Agent SDK and Codex SDK functionality
    let _bridge_process = BridgeProcess::spawn()?;
    info!("[kyco] SDK Bridge server ready");

    let (http_port, http_token) = config
        .read()
        .map(|cfg| {
            (
                cfg.settings.gui.http_port,
                cfg.settings.gui.http_token.clone(),
            )
        })
        .unwrap_or((9876, String::new()));

    info!(
        "[kyco] Starting GUI with HTTP server on port {}...",
        http_port
    );

    // Create channel for HTTP server -> GUI communication (single selection)
    let (http_tx, http_rx): (
        mpsc::Sender<SelectionRequest>,
        mpsc::Receiver<SelectionRequest>,
    ) = mpsc::channel();

    // Create channel for batch requests from IDE
    let (batch_tx, batch_rx): (mpsc::Sender<BatchRequest>, mpsc::Receiver<BatchRequest>) =
        mpsc::channel();

    // Create channel for executor -> GUI communication
    let (executor_tx, executor_rx): (mpsc::Sender<ExecutorEvent>, mpsc::Receiver<ExecutorEvent>) =
        mpsc::channel();

    // Start HTTP server in background (handles both /selection and /batch)
    start_http_server(
        http_tx,
        batch_tx,
        http_port,
        Some(http_token).filter(|t| !t.trim().is_empty()),
        ControlApiState {
            work_dir: work_dir.clone(),
            job_manager: Arc::clone(&job_manager),
            group_manager: Arc::clone(&group_manager),
            executor_tx: executor_tx.clone(),
            config: Arc::clone(&config),
            config_path: config_path.clone(),
        },
    );

    // Create shared max_concurrent_jobs so GUI can update it at runtime
    let max_concurrent_jobs = Arc::new(AtomicUsize::new(
        config
            .read()
            .map(|cfg| cfg.settings.max_concurrent_jobs)
            .unwrap_or(1),
    ));

    start_config_watch_thread(
        config_path.clone(),
        Arc::clone(&config),
        Arc::clone(&max_concurrent_jobs),
        executor_tx.clone(),
    );

    // Start job executor in background
    start_executor(
        work_dir.clone(),
        Arc::clone(&config),
        job_manager.clone(),
        executor_tx,
        Arc::clone(&max_concurrent_jobs),
    );

    let icon = load_kyco_icon();

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

    let app = KycoApp::new(
        work_dir,
        config,
        config_exists,
        job_manager,
        group_manager,
        http_rx,
        batch_rx,
        executor_rx,
        max_concurrent_jobs,
    );

    eframe::run_native(
        "kyco",
        options,
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e))?;

    Ok(())
}
