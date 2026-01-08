//! Shared utilities for bridge adapters.

use std::path::{Path, PathBuf};

use super::super::types::PermissionMode;
use crate::Job;

/// Resolved paths for prompt building.
pub struct ResolvedPaths {
    pub file_path: String,
    /// Target path (may be needed for future template-based prompts)
    #[allow(dead_code)]
    pub target: String,
    pub ide_context: String,
}

/// Resolve paths for a job, converting to relative paths when in a worktree.
pub fn resolve_prompt_paths(job: &Job) -> ResolvedPaths {
    if job.git_worktree_path.is_some() {
        if let Some(workspace) = &job.workspace_path {
            let ws_str = workspace.display().to_string();
            let ws_prefix = if ws_str.ends_with('/') { ws_str.clone() } else { format!("{}/", ws_str) };

            let file_path = job.source_file.strip_prefix(workspace)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| job.source_file.display().to_string());

            let target = if job.target.starts_with(&ws_prefix) {
                job.target.replacen(&ws_prefix, "", 1)
            } else if job.target.starts_with(&ws_str) {
                job.target.replacen(&ws_str, "", 1).trim_start_matches('/').to_string()
            } else {
                job.target.clone()
            };

            let ide_context = job.ide_context.as_deref()
                .map(|ctx| ctx.replace(&ws_prefix, "").replace(&ws_str, ""))
                .unwrap_or_default();

            return ResolvedPaths { file_path, target, ide_context };
        }
    }
    ResolvedPaths {
        file_path: job.source_file.display().to_string(),
        target: job.target.clone(),
        ide_context: job.ide_context.as_deref().unwrap_or("").to_string(),
    }
}

/// Extract output text from structured result if output_text is empty.
pub fn extract_output_from_result(output_text: &mut Option<String>, structured_result: Option<serde_json::Value>) {
    if output_text.is_some() {
        return;
    }
    if let Some(value) = structured_result {
        if !value.is_null() {
            *output_text = match value {
                serde_json::Value::String(s) => Some(s),
                other => serde_json::to_string_pretty(&other).ok(),
            };
        }
    }
}

/// Compute the canonical working directory path for the bridge.
pub fn bridge_cwd_path(worktree: &Path) -> PathBuf {
    if let Ok(abs) = worktree.canonicalize() {
        return abs;
    }
    if worktree.is_absolute() {
        return worktree.to_path_buf();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(worktree))
        .unwrap_or_else(|_| worktree.to_path_buf())
}

/// Convert a worktree path to a string for bridge requests.
pub fn bridge_cwd(worktree: &Path) -> String {
    bridge_cwd_path(worktree).to_string_lossy().to_string()
}

/// Format a tool call for display.
pub fn format_tool_call(name: &str, input: &serde_json::Value) -> String {
    match name {
        "Read" | "read" => input
            .get("file_path")
            .or_else(|| input.get("path"))
            .and_then(|v| v.as_str())
            .map(|p| format!("Read {}", p))
            .unwrap_or_else(|| "Read file".to_string()),

        "Write" | "write" => input
            .get("file_path")
            .or_else(|| input.get("path"))
            .and_then(|v| v.as_str())
            .map(|p| format!("Write {}", p))
            .unwrap_or_else(|| "Write file".to_string()),

        "Edit" | "edit" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|p| format!("Edit {}", p))
            .unwrap_or_else(|| "Edit file".to_string()),

        "Bash" | "bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .map(|c| format!("Bash: {}", c))
            .unwrap_or_else(|| "Bash command".to_string()),

        _ => name.to_string(),
    }
}

/// Parse a permission mode string into the enum.
pub fn parse_claude_permission_mode(mode: &str) -> PermissionMode {
    match mode {
        "default" => PermissionMode::Default,
        "acceptEdits" | "accept_edits" | "accept-edits" => PermissionMode::AcceptEdits,
        "bypassPermissions" | "bypass_permissions" | "bypass-permissions" => {
            PermissionMode::BypassPermissions
        }
        "plan" => PermissionMode::Plan,
        _ => PermissionMode::Default,
    }
}

/// Parse an optional JSON schema string.
pub fn parse_json_schema(schema: Option<&str>) -> Option<serde_json::Value> {
    let schema = schema?.trim();
    if !(schema.starts_with('{') || schema.starts_with('[')) {
        return None;
    }

    let value: serde_json::Value = serde_json::from_str(schema).ok()?;
    if value.is_object() { Some(value) } else { None }
}
