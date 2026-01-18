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

/// Default JSON Schema for SDK structured output (findings, memory, flow_edges, artifacts)
pub fn default_structured_output_schema() -> String {
    // Strict-enough schema to keep backend ingestion reliable:
    // - Top-level keys are fixed (no accidental typos like flowEdges vs flow_edges)
    // - Finding items require the security-audit contract fields when present
    // - Memory items require (type, title), artifacts require (path)
    r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "description": "KYCo NextContext structured output for BugBounty jobs",
  "additionalProperties": false,
  "properties": {
    "findings": {
      "type": "array",
      "description": "Security findings discovered during analysis",
      "default": [],
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["title", "attack_scenario", "preconditions", "reachability", "impact", "confidence"],
        "properties": {
          "id": { "type": "string", "description": "Finding ID (e.g., <project>-VULN-001) if updating an existing finding" },
          "title": { "type": "string", "minLength": 1, "description": "Short descriptive title" },
          "severity": { "type": "string", "description": "critical|high|medium|low|info (or UNKNOWN - <reason>)" },
          "attack_scenario": { "type": "string", "minLength": 1, "description": "How an attacker exploits this (or UNKNOWN - <reason>)" },
          "preconditions": { "type": "string", "minLength": 1, "description": "What must be true for exploitation (or UNKNOWN - <reason>)" },
          "reachability": { "type": "string", "minLength": 1, "description": "public|auth_required|internal_only (or UNKNOWN - <reason>)" },
          "impact": { "type": "string", "minLength": 1, "description": "CIA impact + business impact (or UNKNOWN - <reason>)" },
          "confidence": { "type": "string", "minLength": 1, "description": "high|medium|low (or UNKNOWN - <reason>)" },
          "cwe_id": { "type": "string", "description": "CWE identifier (e.g., CWE-89)" },
          "cvss_score": { "type": "number", "description": "Optional CVSS score (e.g., 7.5)" },
          "affected_assets": { "type": "array", "items": { "type": "string" }, "description": "Affected files/endpoints" },
          "taint_path": { "type": "string", "description": "Data flow from source to sink" }
        }
      }
    },
    "memory": {
      "type": "array",
      "description": "Project memory for tracking sources, sinks, dataflows across sessions",
      "default": [],
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["type", "title"],
        "properties": {
          "type": { "type": "string", "minLength": 1, "description": "source|sink|dataflow|note|context" },
          "title": { "type": "string", "minLength": 1, "description": "Short description" },
          "content": { "type": "string", "description": "Detailed content or code snippet" },
          "file": { "type": "string", "description": "File path" },
          "line": { "type": "integer", "minimum": 1, "description": "Line number (1-based)" },
          "symbol": { "type": "string", "description": "Symbol name (function, variable)" },
          "confidence": { "type": "string", "description": "high|medium|low" },
          "tags": { "type": "array", "items": { "type": "string" }, "description": "Category tags" },
          "from_file": { "type": "string", "description": "Source file (for dataflow)" },
          "from_line": { "type": "integer", "minimum": 1, "description": "Source line (for dataflow)" },
          "to_file": { "type": "string", "description": "Sink file (for dataflow)" },
          "to_line": { "type": "integer", "minimum": 1, "description": "Sink line (for dataflow)" }
        }
      }
    },
    "flow_edges": {
      "type": "array",
      "description": "Dataflow edges between code locations",
      "default": [],
      "items": {
        "type": "object",
        "additionalProperties": false,
        "properties": {
          "finding_id": { "type": "string", "description": "Associated finding ID (optional)" },
          "from_file": { "type": "string", "description": "Source file path" },
          "from_line": { "type": "integer", "minimum": 1, "description": "Source line number (1-based)" },
          "from_symbol": { "type": "string", "description": "Source symbol" },
          "to_file": { "type": "string", "description": "Destination file path" },
          "to_line": { "type": "integer", "minimum": 1, "description": "Destination line number (1-based)" },
          "to_symbol": { "type": "string", "description": "Destination symbol" },
          "kind": { "type": "string", "description": "taint|dataflow|controlflow|authz" },
          "notes": { "type": "string", "description": "Additional notes" }
        }
      }
    },
    "artifacts": {
      "type": "array",
      "description": "Evidence files (requests, screenshots, PoCs)",
      "default": [],
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["path"],
        "properties": {
          "finding_id": { "type": "string", "description": "Associated finding ID (optional)" },
          "type": { "type": "string", "description": "http_request|http_response|screenshot|log|poc_file|other" },
          "path": { "type": "string", "minLength": 1, "description": "Path to artifact file" },
          "description": { "type": "string", "description": "What this demonstrates" },
          "hash": { "type": "string", "description": "SHA256 hash (optional)" }
        }
      }
    },
    "state": { "type": "string", "description": "State for chain control (e.g., issues_found, no_issues)" },
    "summary": { "type": "string", "description": "Human-readable summary" }
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
