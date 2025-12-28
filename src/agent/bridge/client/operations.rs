//! BridgeClient operations for sessions, interrupts, and permissions.

use anyhow::{Context, Result};

use super::super::types::*;
use super::{encode_url_path_segment, BridgeClient};

impl BridgeClient {
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
