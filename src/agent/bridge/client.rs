//! HTTP client for communicating with the SDK Bridge.
//!
//! Handles streaming NDJSON responses from the bridge server.

use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::types::*;

fn encode_url_path_segment(segment: &str) -> String {
    // RFC3986 unreserved = ALPHA / DIGIT / "-" / "." / "_" / "~"
    let mut out = String::with_capacity(segment.len());
    for &b in segment.as_bytes() {
        let is_unreserved =
            matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~');
        if is_unreserved {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

/// GitHub repository for downloading bridge
const GITHUB_REPO: &str = "MAF2414/kyco";

/// Default bridge server URL
const DEFAULT_BRIDGE_URL: &str = "http://127.0.0.1:17432";

/// Bridge client for communicating with the SDK Bridge server
#[derive(Clone)]
pub struct BridgeClient {
    base_url: String,
    client: ureq::Agent,
}

impl BridgeClient {
    /// Create a new bridge client with the default URL
    pub fn new() -> Self {
        Self::with_url(DEFAULT_BRIDGE_URL)
    }

    /// Create a new bridge client with a custom URL
    pub fn with_url(base_url: impl Into<String>) -> Self {
        let client = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout_read(Duration::from_secs(300)) // 5 minute read timeout for long operations
            .build();

        Self {
            base_url: base_url.into(),
            client,
        }
    }

    /// Check if the bridge server is healthy
    pub fn health_check(&self) -> Result<HealthResponse> {
        let url = format!("{}/health", self.base_url);
        let response: HealthResponse = self
            .client
            .get(&url)
            .call()
            .context("Failed to connect to bridge")?
            .into_json()
            .context("Failed to parse health response")?;
        Ok(response)
    }

    /// Get bridge server status
    pub fn status(&self) -> Result<StatusResponse> {
        let url = format!("{}/status", self.base_url);
        let response: StatusResponse = self
            .client
            .get(&url)
            .call()
            .context("Failed to get bridge status")?
            .into_json()
            .context("Failed to parse status response")?;
        Ok(response)
    }

    /// Execute a Claude query and stream events
    ///
    /// Returns an iterator over bridge events. The iterator will yield events
    /// until the session completes or an error occurs.
    ///
    /// Automatically retries up to 3 times on connection failures.
    pub fn claude_query(
        &self,
        request: &ClaudeQueryRequest,
    ) -> Result<impl Iterator<Item = Result<BridgeEvent>>> {
        let url = format!("{}/claude/query", self.base_url);

        const MAX_RETRIES: u32 = 3;
        let mut last_error = None;

        for attempt in 1..=MAX_RETRIES {
            match self.client.post(&url).send_json(request) {
                Ok(response) => {
                    return Ok(EventStream::new(response.into_reader()));
                }
                Err(e) => {
                    tracing::warn!(
                        "Claude query attempt {}/{} failed: {}",
                        attempt,
                        MAX_RETRIES,
                        e
                    );
                    last_error = Some(e);

                    if attempt < MAX_RETRIES {
                        // Wait before retrying (exponential backoff: 500ms, 1s, 2s)
                        let delay = Duration::from_millis(500 * (1 << (attempt - 1)));
                        std::thread::sleep(delay);
                    }
                }
            }
        }

        Err(last_error
            .map(|e| {
                anyhow::anyhow!(
                    "Failed to start Claude query after {} attempts: {}",
                    MAX_RETRIES,
                    e
                )
            })
            .unwrap_or_else(|| anyhow::anyhow!("Failed to start Claude query")))
    }

    /// Execute a Codex query and stream events
    pub fn codex_query(
        &self,
        request: &CodexQueryRequest,
    ) -> Result<impl Iterator<Item = Result<BridgeEvent>>> {
        let url = format!("{}/codex/query", self.base_url);

        let response = self
            .client
            .post(&url)
            .send_json(request)
            .context("Failed to start Codex query")?;

        Ok(EventStream::new(response.into_reader()))
    }

    /// Interrupt a running Claude session
    pub fn interrupt_claude(&self, session_id: &str) -> Result<bool> {
        let url = format!(
            "{}/claude/interrupt/{}",
            self.base_url,
            encode_url_path_segment(session_id)
        );

        #[derive(serde::Deserialize)]
        struct Response {
            success: bool,
        }

        let response: Response = self
            .client
            .post(&url)
            .call()
            .context("Failed to interrupt session")?
            .into_json()
            .context("Failed to parse interrupt response")?;

        Ok(response.success)
    }

    /// Change permission mode for a running Claude session
    pub fn set_claude_permission_mode(
        &self,
        session_id: &str,
        permission_mode: PermissionMode,
    ) -> Result<bool> {
        let url = format!(
            "{}/claude/set-permission-mode/{}",
            self.base_url,
            encode_url_path_segment(session_id)
        );

        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct RequestBody {
            permission_mode: PermissionMode,
        }

        #[derive(serde::Deserialize)]
        struct ResponseBody {
            success: bool,
        }

        let response: ResponseBody = self
            .client
            .post(&url)
            .send_json(RequestBody { permission_mode })
            .context("Failed to set Claude permission mode")?
            .into_json()
            .context("Failed to parse set permission mode response")?;

        Ok(response.success)
    }

    /// Interrupt a running Codex thread
    pub fn interrupt_codex(&self, thread_id: &str) -> Result<bool> {
        let url = format!(
            "{}/codex/interrupt/{}",
            self.base_url,
            encode_url_path_segment(thread_id)
        );

        #[derive(serde::Deserialize)]
        struct Response {
            success: bool,
        }

        let response: Response = self
            .client
            .post(&url)
            .call()
            .context("Failed to interrupt Codex thread")?
            .into_json()
            .context("Failed to parse Codex interrupt response")?;

        Ok(response.success)
    }

    /// List stored sessions
    pub fn list_sessions(&self, session_type: Option<&str>) -> Result<Vec<StoredSession>> {
        let mut url = format!("{}/sessions", self.base_url);
        if let Some(t) = session_type {
            url = format!("{}?type={}", url, t);
        }

        #[derive(serde::Deserialize)]
        struct Response {
            sessions: Vec<StoredSession>,
        }

        let response: Response = self
            .client
            .get(&url)
            .call()
            .context("Failed to list sessions")?
            .into_json()
            .context("Failed to parse sessions response")?;

        Ok(response.sessions)
    }

    /// Get a specific session
    pub fn get_session(&self, session_id: &str) -> Result<Option<StoredSession>> {
        let url = format!(
            "{}/sessions/{}",
            self.base_url,
            encode_url_path_segment(session_id)
        );

        let response = self.client.get(&url).call();

        match response {
            Ok(resp) => {
                #[derive(serde::Deserialize)]
                struct Response {
                    session: StoredSession,
                }
                let r: Response = resp.into_json().context("Failed to parse session")?;
                Ok(Some(r.session))
            }
            Err(ureq::Error::Status(404, _)) => Ok(None),
            Err(e) => Err(e).context("Failed to get session"),
        }
    }

    /// Send a tool approval response to the bridge
    ///
    /// This is used when running in permission mode (default/acceptEdits) and
    /// Claude requests permission to use a tool.
    pub fn send_tool_approval(&self, response: &ToolApprovalResponse) -> Result<bool> {
        let url = format!("{}/claude/tool-approval", self.base_url);

        #[derive(serde::Deserialize)]
        struct Response {
            success: bool,
        }

        let resp: Response = self
            .client
            .post(&url)
            .send_json(response)
            .context("Failed to send tool approval")?
            .into_json()
            .context("Failed to parse tool approval response")?;

        Ok(resp.success)
    }
}

impl Default for BridgeClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over NDJSON event stream
struct EventStream<R: std::io::Read> {
    reader: BufReader<R>,
    buffer: String,
}

impl<R: std::io::Read> EventStream<R> {
    fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            buffer: String::new(),
        }
    }
}

impl<R: std::io::Read> Iterator for EventStream<R> {
    type Item = Result<BridgeEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        // Use a loop instead of recursion to skip empty lines (avoids stack overflow)
        loop {
            self.buffer.clear();

            match self.reader.read_line(&mut self.buffer) {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    let trimmed = self.buffer.trim();
                    if trimmed.is_empty() {
                        continue; // Skip empty lines without recursion
                    }

                    return match serde_json::from_str::<BridgeEvent>(trimmed) {
                        Ok(event) => Some(Ok(event)),
                        Err(e) => Some(Err(anyhow::anyhow!(
                            "Failed to parse event: {} (line: {})",
                            e,
                            trimmed
                        ))),
                    };
                }
                Err(e) => return Some(Err(anyhow::anyhow!("Failed to read from stream: {}", e))),
            }
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bridge_event_text() {
        let json = r#"{"type":"text","sessionId":"abc123","timestamp":1234567890,"content":"Hello","partial":false}"#;
        let event: BridgeEvent = serde_json::from_str(json).unwrap();

        match event {
            BridgeEvent::Text {
                session_id,
                content,
                partial,
                ..
            } => {
                assert_eq!(session_id, "abc123");
                assert_eq!(content, "Hello");
                assert!(!partial);
            }
            _ => panic!("Expected Text event"),
        }
    }

    #[test]
    fn test_parse_bridge_event_session_complete() {
        let json = r#"{"type":"session.complete","sessionId":"xyz789","timestamp":1234567890,"success":true,"durationMs":5000}"#;
        let event: BridgeEvent = serde_json::from_str(json).unwrap();

        match event {
            BridgeEvent::SessionComplete {
                session_id,
                success,
                duration_ms,
                ..
            } => {
                assert_eq!(session_id, "xyz789");
                assert!(success);
                assert_eq!(duration_ms, 5000);
            }
            _ => panic!("Expected SessionComplete event"),
        }
    }
}
