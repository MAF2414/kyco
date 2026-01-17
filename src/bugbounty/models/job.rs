//! BugBounty job model - tracks KYCo runs against a BugBounty project.

use serde::{Deserialize, Serialize};

/// A persisted job record for BugBounty tracking.
///
/// Note: This is separate from KYCo's in-memory `Job` model. It is used for:
/// - Linking KYCo runs to findings (`job_findings`)
/// - Attaching artifacts to the job
/// - Filtering/reporting per project across sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugBountyJob {
    /// Primary key (string, e.g. UUID)
    pub id: String,

    /// BugBounty project ID (nullable)
    pub project_id: Option<String>,

    /// KYCo's in-memory job id (nullable; used for lookups from the running GUI)
    pub kyco_job_id: Option<u64>,

    /// Mode/skill name (e.g. "authz-bypass-hunter")
    pub mode: Option<String>,

    /// Target files (JSON array in DB)
    pub target_files: Vec<String>,

    /// Freeform prompt/description
    pub prompt: Option<String>,

    /// Status string (e.g. pending/running/done/failed)
    pub status: String,

    /// Agent result state (e.g. "issues_found")
    pub result_state: Option<String>,

    /// Raw `next_context` JSON (stored for debugging/auditing)
    pub next_context: Option<serde_json::Value>,

    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub created_at: i64,
}

impl BugBountyJob {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            project_id: None,
            kyco_job_id: None,
            mode: None,
            target_files: Vec::new(),
            prompt: None,
            status: "pending".to_string(),
            result_state: None,
            next_context: None,
            started_at: None,
            completed_at: None,
            created_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    pub fn with_kyco_job_id(mut self, kyco_job_id: u64) -> Self {
        self.kyco_job_id = Some(kyco_job_id);
        self
    }

    pub fn with_mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = Some(mode.into());
        self
    }

    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn with_target_file(mut self, path: impl Into<String>) -> Self {
        self.target_files.push(path.into());
        self
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    pub fn mark_started(mut self) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        self.started_at = Some(now);
        self.status = "running".to_string();
        self
    }

    pub fn mark_completed(mut self, ok: bool) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        self.completed_at = Some(now);
        self.status = if ok { "done".to_string() } else { "failed".to_string() };
        self
    }
}

