//! Type definitions for the HTTP server.

use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};

use super::super::executor::ExecutorEvent;
use crate::config::Config;
use crate::job::{GroupManager, JobManager};
use crate::JobId;

/// Shared state for the local control API (used by `kyco job ...` and orchestrators).
#[derive(Clone)]
pub struct ControlApiState {
    pub work_dir: std::path::PathBuf,
    pub job_manager: Arc<Mutex<JobManager>>,
    pub group_manager: Arc<Mutex<GroupManager>>,
    pub executor_tx: Sender<ExecutorEvent>,
    pub config: Arc<RwLock<Config>>,
    pub config_path: std::path::PathBuf,
}

/// Dependency location from IDE
#[derive(Debug, Clone, Deserialize)]
pub struct Dependency {
    pub file_path: String,
    pub line: usize,
}

/// Diagnostic (error, warning, etc.) from IDE
#[derive(Debug, Clone, Deserialize)]
pub struct Diagnostic {
    /// Severity level: Error, Warning, Information, or Hint
    pub severity: String,
    /// The diagnostic message
    pub message: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Optional error/warning code from the language server
    pub code: Option<String>,
}

/// Selection data received from IDE extensions
#[derive(Debug, Clone, Deserialize)]
pub struct SelectionRequest {
    pub file_path: Option<String>,
    pub selected_text: Option<String>,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub workspace: Option<String>,
    /// Git repository root if file is in a git repo, None otherwise
    pub git_root: Option<String>,
    /// Project root: git_root > workspace_folder > file's parent dir
    /// This is the path that should be used as cwd for the agent
    pub project_root: Option<String>,
    pub dependencies: Option<Vec<Dependency>>,
    pub dependency_count: Option<usize>,
    pub additional_dependency_count: Option<usize>,
    pub related_tests: Option<Vec<String>>,
    /// Diagnostics (errors, warnings) from the IDE for this file
    pub diagnostics: Option<Vec<Diagnostic>>,
}

/// A single file in a batch request
#[derive(Debug, Clone, Deserialize)]
pub struct BatchFile {
    /// Path to the file
    pub path: String,
    /// Workspace root directory
    pub workspace: String,
    /// Git repository root if file is in a git repo
    pub git_root: Option<String>,
    /// Project root: git_root > workspace > file's parent dir
    pub project_root: Option<String>,
    /// Optional: start line for selection
    pub line_start: Option<usize>,
    /// Optional: end line for selection
    pub line_end: Option<usize>,
}

/// Batch processing request from IDE extensions
///
/// Note: Only contains file list. Mode, agents, and prompt are selected
/// in the KYCo GUI popup (same UX as single file selection).
#[derive(Debug, Clone, Deserialize)]
pub struct BatchRequest {
    /// Files to process
    pub files: Vec<BatchFile>,
}

/// Control API: create one or more jobs from a file selection.
#[derive(Debug, Clone, Deserialize)]
pub struct ControlJobCreateRequest {
    /// File path (relative to KYCo work_dir, or absolute).
    pub file_path: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub selected_text: Option<String>,
    /// Mode or chain name.
    pub mode: String,
    /// Optional freeform prompt/description.
    pub prompt: Option<String>,
    /// Primary agent id (e.g. "claude"). Ignored if `agents` is provided.
    pub agent: Option<String>,
    /// Optional list of agents for parallel execution (multi-agent group).
    pub agents: Option<Vec<String>>,
    /// If true, set status to queued immediately.
    #[serde(default = "default_true")]
    pub queue: bool,
    /// If true, force running in a git worktree (like Shift+Enter in UI).
    #[serde(default)]
    pub force_worktree: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ControlJobCreateResponse {
    pub job_ids: Vec<JobId>,
    pub group_id: Option<crate::AgentGroupId>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlLogRequest {
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlJobContinueRequest {
    pub prompt: String,
    #[serde(default = "default_true")]
    pub queue: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ControlJobContinueResponse {
    pub job_id: JobId,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlJobDeleteRequest {
    #[serde(default)]
    pub cleanup_worktree: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ControlJobDeleteResponse {
    pub status: String,
    pub job_id: JobId,
    pub cleanup_worktree: bool,
}

pub(crate) fn default_true() -> bool {
    true
}

impl SelectionRequest {
    /// Format IDE context as markdown for prompt injection
    pub fn format_ide_context(&self) -> String {
        let mut ctx = String::new();

        ctx.push_str("## IDE Selection Context\n");

        if let Some(ref path) = self.file_path {
            ctx.push_str(&format!("- **File:** `{}`\n", path));
        }

        if let (Some(start), Some(end)) = (self.line_start, self.line_end) {
            ctx.push_str(&format!("- **Lines:** {}-{}\n", start, end));
        }

        // Dependencies
        if let Some(count) = self.dependency_count {
            if count > 0 {
                ctx.push_str(&format!("\n### Dependencies ({} total", count));
                if let Some(additional) = self.additional_dependency_count {
                    if additional > 0 {
                        ctx.push_str(&format!(", showing {}", count - additional));
                    }
                }
                ctx.push_str("):\n");

                if let Some(ref deps) = self.dependencies {
                    for dep in deps {
                        ctx.push_str(&format!("- `{}:{}`\n", dep.file_path, dep.line));
                    }
                }

                if let Some(additional) = self.additional_dependency_count {
                    if additional > 0 {
                        ctx.push_str(&format!("- *...and {} more*\n", additional));
                    }
                }
            }
        }

        // Related Tests
        if let Some(ref tests) = self.related_tests {
            if !tests.is_empty() {
                ctx.push_str("\n### Related Tests:\n");
                for test in tests {
                    ctx.push_str(&format!("- `{}`\n", test));
                }
            }
        }

        // Diagnostics (Errors/Warnings)
        if let Some(ref diagnostics) = self.diagnostics {
            if !diagnostics.is_empty() {
                let errors: Vec<_> = diagnostics
                    .iter()
                    .filter(|d| d.severity == "Error")
                    .collect();
                let warnings: Vec<_> = diagnostics
                    .iter()
                    .filter(|d| d.severity == "Warning")
                    .collect();

                ctx.push_str("\n### Diagnostics:\n");

                if !errors.is_empty() {
                    ctx.push_str(&format!("**Errors ({}):**\n", errors.len()));
                    for diag in errors {
                        ctx.push_str(&format!(
                            "- Line {}: {}{}\n",
                            diag.line,
                            diag.message,
                            diag.code
                                .as_ref()
                                .map(|c| format!(" [{}]", c))
                                .unwrap_or_default()
                        ));
                    }
                }

                if !warnings.is_empty() {
                    ctx.push_str(&format!("**Warnings ({}):**\n", warnings.len()));
                    for diag in warnings {
                        ctx.push_str(&format!(
                            "- Line {}: {}{}\n",
                            diag.line,
                            diag.message,
                            diag.code
                                .as_ref()
                                .map(|c| format!(" [{}]", c))
                                .unwrap_or_default()
                        ));
                    }
                }
            }
        }

        ctx
    }
}
