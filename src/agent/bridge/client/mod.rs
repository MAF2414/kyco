//! HTTP client for communicating with the SDK Bridge.
//!
//! Handles streaming NDJSON responses from the bridge server.

mod operations;
mod process;
mod stream;

#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use std::time::Duration;

use super::types::*;
use stream::EventStream;

pub use process::BridgeProcess;

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

/// Default bridge server URL
const DEFAULT_BRIDGE_URL: &str = "http://127.0.0.1:17432";

/// Bridge client for communicating with the SDK Bridge server
#[derive(Clone)]
pub struct BridgeClient {
    pub(super) base_url: String,
    pub(super) client: ureq::Agent,
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
}

impl Default for BridgeClient {
    fn default() -> Self {
        Self::new()
    }
}
