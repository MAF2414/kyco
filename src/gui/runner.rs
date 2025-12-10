//! GUI runner - launches the main kyco GUI application
//!
//! The GUI receives selections from IDE extensions via HTTP server

use anyhow::Result;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily, IconData};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tracing::info;

use super::app::KycoApp;
use super::executor::{start_executor, ExecutorEvent};
use super::http_server::{start_http_server, BatchRequest, SelectionRequest};
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
pub fn run_gui() -> Result<()> {
    // Get working directory
    let work_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Load config
    let config_path = work_dir.join(".kyco").join("config.toml");
    let config_exists = config_path.exists();
    let config = if config_exists {
        Config::from_file(&config_path).unwrap_or_default()
    } else {
        Config::default()
    };

    // Load job manager
    let job_manager = Arc::new(Mutex::new(JobManager::load(&work_dir).unwrap_or_else(|_| JobManager::new(&work_dir))));

    info!("[kyco] Starting GUI with HTTP server on port 9876...");

    // Create channel for HTTP server -> GUI communication (single selection)
    let (http_tx, http_rx): (mpsc::Sender<SelectionRequest>, mpsc::Receiver<SelectionRequest>) = mpsc::channel();

    // Create channel for batch requests from IDE
    let (batch_tx, batch_rx): (mpsc::Sender<BatchRequest>, mpsc::Receiver<BatchRequest>) = mpsc::channel();

    // Create channel for executor -> GUI communication
    let (executor_tx, executor_rx): (mpsc::Sender<ExecutorEvent>, mpsc::Receiver<ExecutorEvent>) = mpsc::channel();

    // Start HTTP server in background (handles both /selection and /batch)
    start_http_server(http_tx, batch_tx);

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
