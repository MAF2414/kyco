//! Bridge process lifecycle management.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::BridgeClient;

/// GitHub repository for downloading bridge
const GITHUB_REPO: &str = "MAF2414/kyco";

/// Manages the lifecycle of the bridge Node.js process
pub struct BridgeProcess {
    child: Option<Child>,
    running: Arc<AtomicBool>,
}

impl BridgeProcess {
    /// Spawn the bridge server process
    ///
    /// Looks for the bridge in the following locations:
    /// 1. KYCO_BRIDGE_PATH environment variable
    /// 2. ./bridge/ relative to the executable
    /// 3. ~/.kyco/bridge/
    pub fn spawn() -> Result<Self> {
        // If a bridge is already running (e.g., started externally), reuse it.
        if BridgeClient::new().health_check().is_ok() {
            tracing::info!("SDK bridge server already running");
            return Ok(Self {
                child: None,
                running: Arc::new(AtomicBool::new(true)),
            });
        }

        let bridge_dir = Self::find_bridge_dir()?;

        // Check if node_modules exists, if not install dependencies.
        // Prefer `npm ci` when a lockfile is present for deterministic installs.
        let node_modules = bridge_dir.join("node_modules");
        if !node_modules.exists() {
            let lockfile = bridge_dir.join("package-lock.json");
            let mut cmd = Command::new("npm");
            if lockfile.exists() {
                tracing::info!("Installing bridge dependencies (npm ci)...");
                cmd.arg("ci");
            } else {
                tracing::info!("Installing bridge dependencies (npm install)...");
                cmd.arg("install");
            }

            let status = cmd
                .current_dir(&bridge_dir)
                .status()
                .context("Failed to install bridge dependencies")?;

            if !status.success() {
                anyhow::bail!("npm install failed for bridge");
            }
        }

        let dist_dir = bridge_dir.join("dist");
        if !dist_dir.exists() {
            tracing::info!("Building bridge...");
            let status = Command::new("npm")
                .arg("run")
                .arg("build")
                .current_dir(&bridge_dir)
                .status()
                .context("Failed to build bridge")?;

            if !status.success() {
                anyhow::bail!("Bridge build failed");
            }
        }

        tracing::info!("Starting SDK bridge server...");
        // Use Stdio::null() instead of piped() since we don't read the output.
        // Piped stdout/stderr without consumption can cause the child process to
        // block when the pipe buffers fill up (~64KB).
        let mut child = Command::new("node")
            .arg("dist/server.js")
            .current_dir(&bridge_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn bridge process")?;

        let running = Arc::new(AtomicBool::new(true));

        std::thread::sleep(Duration::from_millis(1500));

        let client = BridgeClient::new();
        for attempt in 0..5 {
            match client.health_check() {
                Ok(_) => {
                    tracing::info!("SDK bridge server started successfully");
                    return Ok(Self {
                        child: Some(child),
                        running,
                    });
                }
                Err(_) if attempt < 4 => {
                    std::thread::sleep(Duration::from_millis(500));
                }
                Err(e) => {
                    // Kill the orphan process before bailing
                    let _ = child.kill();
                    let _ = child.wait();
                    anyhow::bail!("Bridge server failed to start: {}", e);
                }
            }
        }

        // Kill the orphan process before bailing
        let _ = child.kill();
        let _ = child.wait();
        anyhow::bail!("Bridge server failed to become healthy")
    }

    /// Find the bridge directory, downloading it if necessary
    fn find_bridge_dir() -> Result<PathBuf> {
        if let Ok(path) = std::env::var("KYCO_BRIDGE_PATH") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Ok(path);
            }
        }

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let bridge_dir = exe_dir.join("bridge");
                if bridge_dir.exists() {
                    return Ok(bridge_dir);
                }

                if let Some(parent) = exe_dir.parent() {
                    let bridge_dir = parent.join("bridge");
                    if bridge_dir.exists() {
                        return Ok(bridge_dir);
                    }
                }
            }
        }

        let kyco_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".kyco");
        let bridge_dir = kyco_dir.join("bridge");

        if bridge_dir.exists() {
            return Ok(bridge_dir);
        }

        let cwd_bridge = PathBuf::from("bridge");
        if cwd_bridge.exists() {
            return Ok(cwd_bridge);
        }

        tracing::info!("SDK Bridge not found, downloading from GitHub Releases...");
        Self::download_and_install_bridge(&kyco_dir)?;

        Ok(bridge_dir)
    }

    /// Download and install the bridge from GitHub Releases
    fn download_and_install_bridge(kyco_dir: &PathBuf) -> Result<()> {
        std::fs::create_dir_all(kyco_dir).context("Failed to create ~/.kyco directory")?;

        let download_url = format!(
            "https://github.com/{}/releases/latest/download/kyco-bridge.tar.gz",
            GITHUB_REPO
        );
        let tarball_path = kyco_dir.join("kyco-bridge.tar.gz");

        tracing::info!("Downloading bridge from {}...", download_url);
        let output = Command::new("curl")
            .args([
                "-L", // Follow redirects
                "-f", // Fail on HTTP errors
                "-#", // Progress bar
                "-o",
                tarball_path.to_str().unwrap_or("kyco-bridge.tar.gz"),
                &download_url,
            ])
            .output()
            .context("Failed to run curl - is curl installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to download bridge: {}", stderr);
        }

        tracing::info!("Extracting bridge...");
        let output = Command::new("tar")
            .args([
                "-xzf",
                tarball_path.to_str().unwrap_or("kyco-bridge.tar.gz"),
                "-C",
                kyco_dir.to_str().unwrap_or("."),
            ])
            .output()
            .context("Failed to run tar - is tar installed?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to extract bridge: {}", stderr);
        }

        let _ = std::fs::remove_file(&tarball_path);

        tracing::info!(
            "SDK Bridge installed to {}",
            kyco_dir.join("bridge").display()
        );
        Ok(())
    }

    /// Check if the bridge process is still running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Stop the bridge server
    pub fn stop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.running.store(false, Ordering::SeqCst);
        self.child = None;
    }
}

impl Drop for BridgeProcess {
    fn drop(&mut self) {
        self.stop();
    }
}
