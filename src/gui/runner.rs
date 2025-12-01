//! GUI runner - launches the main kyco GUI application
//!
//! The GUI receives selections from IDE extensions via HTTP server

use anyhow::Result;
use eframe::egui::{self, IconData};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tracing::info;

use super::app::KycoApp;
use super::executor::{start_executor, ExecutorEvent};
use super::http_server::{start_http_server, SelectionRequest};
use crate::config::Config;
use crate::job::JobManager;

/// Create the KYCo app icon programmatically
/// A stylized "K" with rail tracks - representing "Kyco" / "CodeRail"
fn create_kyco_icon() -> IconData {
    const SIZE: usize = 64;
    let mut rgba = vec![0u8; SIZE * SIZE * 4];

    // Colors (RGBA)
    let amber = [255u8, 176, 0, 255];      // Primary amber/gold color (matching theme)
    let cyan = [0u8, 255, 200, 255];        // Accent cyan
    let dark_bg = [18u8, 20, 24, 255];      // Dark background

    // Fill background
    for y in 0..SIZE {
        for x in 0..SIZE {
            let idx = (y * SIZE + x) * 4;
            rgba[idx..idx + 4].copy_from_slice(&dark_bg);
        }
    }

    // Helper to draw a filled rectangle
    let fill_rect = |rgba: &mut [u8], x0: usize, y0: usize, w: usize, h: usize, color: [u8; 4]| {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                if x < SIZE && y < SIZE {
                    let idx = (y * SIZE + x) * 4;
                    rgba[idx..idx + 4].copy_from_slice(&color);
                }
            }
        }
    };

    // Draw stylized "K" for Kyco
    // Left vertical bar of K
    fill_rect(&mut rgba, 12, 12, 8, 40, amber);

    // Upper diagonal of K (going right-up from middle)
    for i in 0..20 {
        let x = 20 + i;
        let y = 32 - i;
        fill_rect(&mut rgba, x, y, 8, 4, amber);
    }

    // Lower diagonal of K (going right-down from middle)
    for i in 0..20 {
        let x = 20 + i;
        let y = 32 + i;
        fill_rect(&mut rgba, x, y, 8, 4, amber);
    }

    // Add rail track lines (representing CodeRail)
    // Two horizontal cyan lines at bottom
    fill_rect(&mut rgba, 4, 56, 56, 2, cyan);
    fill_rect(&mut rgba, 4, 60, 56, 2, cyan);

    // Rail ties (vertical short bars connecting the tracks)
    for i in 0..7 {
        let x = 8 + i * 8;
        fill_rect(&mut rgba, x, 55, 3, 8, cyan);
    }

    IconData {
        rgba,
        width: SIZE as u32,
        height: SIZE as u32,
    }
}

/// Run the main GUI application
pub fn run_gui() -> Result<()> {
    // Get working directory
    let work_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Load config
    let config_path = work_dir.join(".kyco").join("config.toml");
    let config = if config_path.exists() {
        Config::from_file(&config_path).unwrap_or_default()
    } else {
        Config::default()
    };

    // Load job manager
    let job_manager = Arc::new(Mutex::new(JobManager::load(&work_dir).unwrap_or_else(|_| JobManager::new(&work_dir))));

    info!("[kyco] Starting GUI with HTTP server on port 9876...");

    // Create channel for HTTP server -> GUI communication
    let (http_tx, http_rx): (mpsc::Sender<SelectionRequest>, mpsc::Receiver<SelectionRequest>) = mpsc::channel();

    // Create channel for executor -> GUI communication
    let (executor_tx, executor_rx): (mpsc::Sender<ExecutorEvent>, mpsc::Receiver<ExecutorEvent>) = mpsc::channel();

    // Start HTTP server in background
    start_http_server(http_tx);

    // Start job executor in background
    start_executor(
        work_dir.clone(),
        config.clone(),
        job_manager.clone(),
        executor_tx,
        config.settings.max_concurrent_jobs,
    );

    // Create app icon
    let icon = create_kyco_icon();

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

    let app = KycoApp::new(work_dir, config, job_manager, http_rx, executor_rx);

    eframe::run_native("kyco", options, Box::new(|_cc| Ok(Box::new(app))))
        .map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e))?;

    Ok(())
}
