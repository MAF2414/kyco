//! GUI-specific settings

use serde::{Deserialize, Serialize};

use super::orchestrator::OrchestratorSettings;
use super::voice::VoiceSettings;

/// GUI-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSettings {
    /// Global hotkey to trigger the popup
    /// Format: "modifier+modifier+key" (e.g., "cmd+shift+k", "ctrl+shift+space")
    /// Modifiers: cmd/super, ctrl, alt, shift
    /// Default: "cmd+shift+k" on macOS, "ctrl+shift+k" on Windows/Linux
    #[serde(default = "default_gui_hotkey")]
    pub hotkey: String,

    /// Default agent for GUI tasks
    #[serde(default = "default_gui_agent")]
    pub default_agent: String,

    /// Default mode for GUI tasks
    #[serde(default = "default_gui_mode")]
    pub default_mode: String,

    /// Output schema that agents should include in their response
    /// This YAML block helps structure the output for better GUI display
    #[serde(default = "default_output_schema")]
    pub output_schema: String,

    /// Optional JSON Schema used for SDK structured output.
    ///
    /// When set, Claude and Codex will be asked to produce JSON matching this schema.
    /// Leave empty to keep the YAML summary footer behavior.
    #[serde(default = "default_structured_output_schema")]
    pub structured_output_schema: String,

    /// Local HTTP server port for IDE extensions
    /// Default: 9876
    #[serde(default = "default_gui_http_port")]
    pub http_port: u16,

    /// Shared secret required for IDE extension requests (sent as `X-KYCO-Token`)
    ///
    /// If empty, the server will accept unauthenticated requests (not recommended).
    #[serde(default)]
    pub http_token: String,

    /// Voice input settings
    #[serde(default)]
    pub voice: VoiceSettings,

    /// Orchestrator settings for external CLI sessions
    #[serde(default)]
    pub orchestrator: OrchestratorSettings,
}

fn default_gui_hotkey() -> String {
    #[cfg(target_os = "macos")]
    return "cmd+option+k".to_string();
    #[cfg(not(target_os = "macos"))]
    return "ctrl+alt+k".to_string();
}

fn default_gui_agent() -> String {
    "claude".to_string()
}

fn default_gui_mode() -> String {
    "implement".to_string()
}

fn default_output_schema() -> String {
    r#"
IMPORTANT: End your response with a structured YAML summary block:
---
title: Short task title (max 60 chars)
commit_subject: Suggested git commit subject (max 72 chars)
commit_body: |
  Suggested git commit body (optional, can be multiline)
details: What was done (2-3 sentences)
status: success|partial|failed
summary: |
  Detailed summary of findings and actions (optional, can be multiline).
  This is passed to the next agent in a chain for context.
state: <state_identifier>
---

STATE VALUES: issues_found, no_issues, fixed, unfixable, tests_pass, tests_fail, implemented, blocked, refactored, documented
"#
    .to_string()
}

fn default_structured_output_schema() -> String {
    r#"{
  "type": "object",
  "description": "BugBounty security audit structured output",
  "properties": {
    "findings": {
      "type": "array",
      "description": "Security findings discovered during analysis",
      "items": {
        "type": "object",
        "properties": {
          "id": { "type": "string", "description": "Finding ID (e.g., VULN-001)" },
          "title": { "type": "string", "description": "Short descriptive title (max 100 chars)" },
          "severity": { "type": "string", "enum": ["critical", "high", "medium", "low", "info"], "description": "Severity rating based on CVSS-like impact" },
          "attack_scenario": { "type": "string", "description": "Step-by-step description of how an attacker exploits this vulnerability" },
          "preconditions": { "type": "string", "description": "What conditions must be true for successful exploitation (e.g., auth state, config)" },
          "reachability": { "type": "string", "enum": ["public", "auth_required", "internal_only"], "description": "Who can reach this vulnerability: public (unauthenticated), auth_required (any authenticated user), internal_only (admin/internal)" },
          "impact": { "type": "string", "description": "CIA triad impact (Confidentiality, Integrity, Availability) plus business impact" },
          "confidence": { "type": "string", "enum": ["high", "medium", "low"], "description": "Confidence level: high (verified/PoC), medium (likely but unverified), low (theoretical)" },
          "cwe_id": { "type": "string", "description": "CWE identifier (e.g., CWE-89 for SQL Injection)" },
          "affected_assets": { "type": "array", "items": { "type": "string" }, "description": "List of affected files, endpoints, or components (e.g., '/api/login', 'src/auth.rs:42')" },
          "taint_path": { "type": "string", "description": "Data flow path from source to sink (e.g., 'request.body.username -> sql.query()')" }
        },
        "required": ["title"]
      }
    },
    "memory": {
      "type": "array",
      "description": "Project memory entries for tracking sources, sinks, dataflow paths, and context across audit sessions",
      "items": {
        "type": "object",
        "properties": {
          "type": { "type": "string", "enum": ["source", "sink", "dataflow", "note", "context"], "description": "Memory type: source (user input entry), sink (dangerous operation), dataflow (taint path), note (observation), context (architecture knowledge)" },
          "title": { "type": "string", "description": "Short description of this memory entry" },
          "content": { "type": "string", "description": "Detailed content, explanation, or code snippet" },
          "file": { "type": "string", "description": "File path where this was found" },
          "line": { "type": "integer", "description": "Line number in file" },
          "symbol": { "type": "string", "description": "Symbol name (function, method, variable)" },
          "confidence": { "type": "string", "enum": ["high", "medium", "low"], "description": "Confidence level for this memory entry" },
          "tags": { "type": "array", "items": { "type": "string" }, "description": "Category tags (e.g., 'http', 'sql', 'auth', 'crypto')" },
          "from_file": { "type": "string", "description": "Source file path (for dataflow type)" },
          "from_line": { "type": "integer", "description": "Source line number (for dataflow type)" },
          "to_file": { "type": "string", "description": "Destination/sink file path (for dataflow type)" },
          "to_line": { "type": "integer", "description": "Destination/sink line number (for dataflow type)" }
        },
        "required": ["type", "title"]
      }
    },
    "flow_edges": {
      "type": "array",
      "description": "Dataflow/taint edges between specific code locations for visualization",
      "items": {
        "type": "object",
        "properties": {
          "finding_id": { "type": "string", "description": "Associated finding ID if this edge relates to a vulnerability" },
          "from_file": { "type": "string", "description": "Source file path" },
          "from_line": { "type": "integer", "description": "Source line number" },
          "from_symbol": { "type": "string", "description": "Source symbol (function, variable)" },
          "to_file": { "type": "string", "description": "Destination file path" },
          "to_line": { "type": "integer", "description": "Destination line number" },
          "to_symbol": { "type": "string", "description": "Destination symbol" },
          "kind": { "type": "string", "enum": ["taint", "dataflow", "controlflow", "authz"], "description": "Edge type: taint (untrusted data), dataflow (any data), controlflow (execution path), authz (authorization check)" },
          "notes": { "type": "string", "description": "Additional notes about this edge" }
        }
      }
    },
    "artifacts": {
      "type": "array",
      "description": "Evidence files such as HTTP requests/responses, screenshots, PoC code, or logs",
      "items": {
        "type": "object",
        "properties": {
          "finding_id": { "type": "string", "description": "Associated finding ID" },
          "type": { "type": "string", "enum": ["http_request", "http_response", "screenshot", "log", "poc_file", "other"], "description": "Type of artifact" },
          "path": { "type": "string", "description": "Path to artifact file" },
          "description": { "type": "string", "description": "What this artifact demonstrates" },
          "hash": { "type": "string", "description": "Content hash for deduplication (SHA256)" }
        },
        "required": ["path"]
      }
    },
    "state": { "type": "string", "description": "State identifier for chain control flow (e.g., 'issues_found', 'no_issues', 'fixed', 'tests_pass')" },
    "summary": { "type": "string", "description": "Human-readable summary of findings and actions, also passed to next agent in chain" }
  }
}"#
    .to_string()
}

fn default_gui_http_port() -> u16 {
    9876
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            hotkey: default_gui_hotkey(),
            default_agent: default_gui_agent(),
            default_mode: default_gui_mode(),
            output_schema: default_output_schema(),
            structured_output_schema: default_structured_output_schema(),
            http_port: default_gui_http_port(),
            http_token: String::new(),
            voice: VoiceSettings::default(),
            orchestrator: OrchestratorSettings::default(),
        }
    }
}
