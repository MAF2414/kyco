//! Context injection for BugBounty jobs
//!
//! Automatically injects relevant context into agent prompts:
//! - Known findings (to avoid duplicates, build on existing work)
//! - Scope information (in-scope/out-of-scope assets)
//! - Tool policy (allowed/blocked commands)
//! - Project metadata

use anyhow::Result;

use super::{
    BugBountyManager, Finding, FindingStatus, MemoryType, Project, ProjectMemory, Severity,
};

/// Context to inject into agent prompts
#[derive(Debug, Clone)]
pub struct InjectedContext {
    /// Project context (name, platform, etc.)
    pub project_info: Option<String>,

    /// Known findings summary
    pub known_findings: Option<String>,

    /// Findings explicitly linked to this job (e.g., verification targets)
    pub focus_findings: Option<String>,

    /// Scope information
    pub scope_info: Option<String>,

    /// Tool policy
    pub tool_policy: Option<String>,

    /// Output schema requirements
    pub output_schema: Option<String>,

    /// Project memory (sources, sinks, dataflow, notes)
    pub project_memory: Option<String>,
}

impl InjectedContext {
    /// Create an empty context
    pub fn empty() -> Self {
        Self {
            project_info: None,
            known_findings: None,
            focus_findings: None,
            scope_info: None,
            tool_policy: None,
            output_schema: None,
            project_memory: None,
        }
    }

    /// Format as a system prompt section
    pub fn to_system_prompt(&self) -> String {
        let mut sections = Vec::new();

        if let Some(ref info) = self.project_info {
            sections.push(format!("## Project Context\n\n{}", info));
        }

        if let Some(ref findings) = self.known_findings {
            sections.push(format!("## Known Findings\n\n{}", findings));
        }

        if let Some(ref findings) = self.focus_findings {
            sections.push(format!("## Focus Findings\n\n{}", findings));
        }

        if let Some(ref scope) = self.scope_info {
            sections.push(format!("## Scope\n\n{}", scope));
        }

        if let Some(ref policy) = self.tool_policy {
            sections.push(format!("## Tool Policy\n\n{}", policy));
        }

        if let Some(ref schema) = self.output_schema {
            sections.push(format!("## Output Requirements\n\n{}", schema));
        }

        if let Some(ref memory) = self.project_memory {
            sections.push(format!("## Project Memory\n\n{}", memory));
        }

        if sections.is_empty() {
            String::new()
        } else {
            format!(
                "# BugBounty Context\n\n{}\n",
                sections.join("\n\n---\n\n")
            )
        }
    }

    /// Check if there's any context to inject
    pub fn is_empty(&self) -> bool {
        self.project_info.is_none()
            && self.known_findings.is_none()
            && self.focus_findings.is_none()
            && self.scope_info.is_none()
            && self.tool_policy.is_none()
            && self.output_schema.is_none()
            && self.project_memory.is_none()
    }
}

/// Builder for injected context
pub struct ContextInjector {
    manager: BugBountyManager,
}

impl ContextInjector {
    /// Create a new context injector
    pub fn new(manager: BugBountyManager) -> Self {
        Self { manager }
    }

    /// Build context for a project
    pub fn for_project(&self, project_id: &str) -> Result<InjectedContext> {
        let project = self.manager.get_project(project_id)?;
        let findings = self.manager.list_findings_by_project(project_id)?;

        let mut ctx = InjectedContext::empty();

        // Project info
        if let Some(ref proj) = project {
            ctx.project_info = Some(self.format_project_info(proj));
            ctx.scope_info = self.format_scope(proj);
            ctx.tool_policy = self.format_tool_policy(proj);
        }

        // Known findings
        if !findings.is_empty() {
            ctx.known_findings = Some(self.format_known_findings(&findings));
        }

        // Load and inject project memory
        let memory_entries = self.manager.memory().list_by_project(project_id)?;
        if !memory_entries.is_empty() {
            ctx.project_memory = Some(self.format_memory(&memory_entries));
        }

        // Output schema (always include for security audits)
        ctx.output_schema = Some(self.get_output_schema());

        Ok(ctx)
    }

    /// Build context for a specific file within a project
    pub fn for_file(&self, project_id: &str, file_path: &str) -> Result<InjectedContext> {
        let mut ctx = self.for_project(project_id)?;

        // Filter findings to those related to this file
        let findings = self.manager.list_findings_by_project(project_id)?;
        let file_findings: Vec<_> = findings
            .into_iter()
            .filter(|f| {
                f.affected_assets
                    .iter()
                    .any(|a| a.contains(file_path) || file_path.contains(a))
                    || f.taint_path
                        .as_ref()
                        .map(|t| t.contains(file_path))
                        .unwrap_or(false)
            })
            .collect();

        if !file_findings.is_empty() {
            ctx.known_findings = Some(format!(
                "### Findings related to `{}`\n\n{}",
                file_path,
                self.format_known_findings(&file_findings)
            ));
        }

        Ok(ctx)
    }

    fn format_project_info(&self, project: &Project) -> String {
        let mut info = Vec::new();

        info.push(format!("- **Project ID:** {}", project.id));
        info.push(format!("- **Root Path:** {}", project.root_path));

        if let Some(ref platform) = project.platform {
            info.push(format!("- **Platform:** {}", platform));
        }

        if let Some(ref target) = project.target_name {
            info.push(format!("- **Target:** {}", target));
        }

        info.join("\n")
    }

    fn format_known_findings(&self, findings: &[Finding]) -> String {
        if findings.is_empty() {
            return "No known findings.".to_string();
        }

        // Group by status
        let mut raw: Vec<&Finding> = Vec::new();
        let mut in_progress: Vec<&Finding> = Vec::new();
        let mut verified: Vec<&Finding> = Vec::new();
        let mut resolved: Vec<&Finding> = Vec::new();

        for f in findings {
            match f.status {
                FindingStatus::Raw => raw.push(f),
                FindingStatus::NeedsRepro | FindingStatus::ReportDraft => in_progress.push(f),
                FindingStatus::Verified
                | FindingStatus::Submitted
                | FindingStatus::Triaged
                | FindingStatus::Accepted => verified.push(f),
                _ => resolved.push(f),
            }
        }

        let mut sections = Vec::new();

        // Summary counts
        sections.push(format!(
            "**Summary:** {} total ({} raw, {} in progress, {} verified, {} resolved)\n",
            findings.len(),
            raw.len(),
            in_progress.len(),
            verified.len(),
            resolved.len()
        ));

        // Show actionable findings in detail
        let actionable: Vec<&Finding> = raw.iter().chain(in_progress.iter()).copied().collect();
        if !actionable.is_empty() {
            sections.push("### Actionable Findings\n".to_string());
            for f in actionable {
                sections.push(self.format_finding_summary(f));
            }
        }

        // Show verified findings (brief)
        if !verified.is_empty() {
            sections.push("\n### Verified Findings\n".to_string());
            for f in verified {
                sections.push(self.format_finding_brief(f));
            }
        }

        sections.join("\n")
    }

    fn format_finding_summary(&self, f: &Finding) -> String {
        let mut lines = Vec::new();

        let severity_badge = f
            .severity
            .map(|s| format!("[{}]", s.as_str().to_uppercase()))
            .unwrap_or_default();

        lines.push(format!(
            "#### {} {} ({})",
            f.id,
            severity_badge,
            f.status.as_str()
        ));
        lines.push(format!("**{}**\n", f.title));

        if let Some(ref scenario) = f.attack_scenario {
            lines.push(format!("- Attack: {}", truncate(scenario, 100)));
        }

        if !f.affected_assets.is_empty() {
            lines.push(format!("- Assets: {}", f.affected_assets.join(", ")));
        }

        if let Some(ref cwe) = f.cwe_id {
            lines.push(format!("- CWE: {}", cwe));
        }

        lines.push(String::new());
        lines.join("\n")
    }

    fn format_finding_brief(&self, f: &Finding) -> String {
        let severity = f
            .severity
            .map(|s| s.as_str().to_uppercase())
            .unwrap_or_else(|| "-".to_string());

        format!(
            "- **{}** [{}] {} ({})",
            f.id,
            severity,
            f.title,
            f.status.as_str()
        )
    }

    fn format_scope(&self, project: &Project) -> Option<String> {
        project.scope.as_ref().map(|scope| {
            let mut lines = Vec::new();

            if !scope.in_scope.is_empty() {
                lines.push("### In Scope\n".to_string());
                for item in &scope.in_scope {
                    lines.push(format!("- {}", item));
                }
            }

            if !scope.out_of_scope.is_empty() {
                lines.push("\n### Out of Scope\n".to_string());
                for item in &scope.out_of_scope {
                    lines.push(format!("- {} (**DO NOT TEST**)", item));
                }
            }

            if let Some(rate) = scope.rate_limit {
                lines.push(format!("\n**Rate Limit:** {} requests/second", rate));
            }

            lines.join("\n")
        })
    }

    fn format_tool_policy(&self, project: &Project) -> Option<String> {
        project.tool_policy.as_ref().map(|policy| {
            let mut lines = Vec::new();

            if !policy.blocked_commands.is_empty() {
                lines.push("### Blocked Commands\n".to_string());
                lines.push(
                    "The following commands are **NOT ALLOWED**. Use the provided wrapper instead:\n"
                        .to_string(),
                );
                for cmd in &policy.blocked_commands {
                    lines.push(format!("- `{}`", cmd));
                }
            }

            if let Some(ref wrapper) = policy.network_wrapper {
                lines.push(format!("\n**Network Wrapper:** Use `{}` for all HTTP requests", wrapper));
            }

            lines.join("\n")
        })
    }

    fn get_output_schema(&self) -> String {
        // Output format is enforced via JSON Schema (SDK structured output).
        // This text provides semantic guidance for the agent.
        r#"Your response will be validated against a JSON Schema. Include:

**findings** - Security vulnerabilities found:
- title (required): Short descriptive title
- severity: critical, high, medium, low, or info
- attack_scenario: How an attacker exploits this
- preconditions: What must be true for exploitation
- reachability: public, auth_required, or internal_only
- impact: CIA impact + business impact
- confidence: high, medium, or low
- cwe_id: CWE identifier (e.g., CWE-89)
- affected_assets: List of affected files/endpoints
- taint_path: Data flow path (e.g., "user_input -> db.query()")

**memory** - Track sources, sinks, and dataflow for project memory:
- type (required): source, sink, dataflow, note, or context
- title (required): Short description
- file: File path
- line: Line number
- symbol: Function/variable name
- confidence: high, medium, or low
- tags: Category tags
- from_file/from_line/to_file/to_line: For dataflow edges

**Memory type guidance:**
- source: User input entry points (request.body, argv, query params)
- sink: Dangerous operations (SQL execute, shell exec, file write)
- dataflow: Taint path from source to sink
- note: Important observations
- context: Architecture/design knowledge
"#
        .to_string()
    }

    /// Format project memory entries for injection into prompts
    fn format_memory(&self, entries: &[ProjectMemory]) -> String {
        if entries.is_empty() {
            return String::new();
        }

        // Group by memory type
        let sources: Vec<_> = entries
            .iter()
            .filter(|e| e.memory_type == MemoryType::Source)
            .collect();
        let sinks: Vec<_> = entries
            .iter()
            .filter(|e| e.memory_type == MemoryType::Sink)
            .collect();
        let dataflows: Vec<_> = entries
            .iter()
            .filter(|e| e.memory_type == MemoryType::Dataflow)
            .collect();
        let notes: Vec<_> = entries
            .iter()
            .filter(|e| e.memory_type == MemoryType::Note || e.memory_type == MemoryType::Context)
            .collect();

        let mut sections = Vec::new();

        if !sources.is_empty() {
            let mut lines = vec!["### Known Sources (User Input Entry Points)\n".to_string()];
            for src in sources.iter().take(20) {
                // Limit to 20 to avoid token bloat
                let conf = src
                    .confidence
                    .map(|c| format!(" [{}]", c.as_str()))
                    .unwrap_or_default();
                let loc = src.location_string().unwrap_or_default();
                lines.push(format!("- **{}**{}: {}", src.title, conf, loc));
            }
            if sources.len() > 20 {
                lines.push(format!("... and {} more sources", sources.len() - 20));
            }
            sections.push(lines.join("\n"));
        }

        if !sinks.is_empty() {
            let mut lines = vec!["### Known Sinks (Dangerous Operations)\n".to_string()];
            for sink in sinks.iter().take(20) {
                let conf = sink
                    .confidence
                    .map(|c| format!(" [{}]", c.as_str()))
                    .unwrap_or_default();
                let loc = sink.location_string().unwrap_or_default();
                lines.push(format!("- **{}**{}: {}", sink.title, conf, loc));
            }
            if sinks.len() > 20 {
                lines.push(format!("... and {} more sinks", sinks.len() - 20));
            }
            sections.push(lines.join("\n"));
        }

        if !dataflows.is_empty() {
            let mut lines = vec!["### Known Dataflow Paths\n".to_string()];
            for df in dataflows.iter().take(15) {
                let from_loc = df
                    .from_location
                    .as_ref()
                    .map(|l| l.format())
                    .unwrap_or_else(|| "?".to_string());
                let to_loc = df
                    .to_location
                    .as_ref()
                    .map(|l| l.format())
                    .unwrap_or_else(|| "?".to_string());
                let desc = df.content.as_deref().unwrap_or(&df.title);
                lines.push(format!("- {} → {}: {}", from_loc, to_loc, desc));
            }
            if dataflows.len() > 15 {
                lines.push(format!("... and {} more dataflow paths", dataflows.len() - 15));
            }
            sections.push(lines.join("\n"));
        }

        if !notes.is_empty() {
            let mut lines = vec!["### Notes & Context\n".to_string()];
            for note in notes.iter().take(10) {
                let content = note.content.as_deref().unwrap_or("");
                if content.is_empty() {
                    lines.push(format!("- {}", note.title));
                } else {
                    lines.push(format!("- **{}**: {}", note.title, truncate(content, 100)));
                }
            }
            if notes.len() > 10 {
                lines.push(format!("... and {} more notes", notes.len() - 10));
            }
            sections.push(lines.join("\n"));
        }

        sections.join("\n\n")
    }
}

/// Generate a summary of findings for embedding in prompts
pub fn generate_findings_summary(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return String::new();
    }

    let critical: Vec<_> = findings
        .iter()
        .filter(|f| f.severity == Some(Severity::Critical))
        .collect();
    let high: Vec<_> = findings
        .iter()
        .filter(|f| f.severity == Some(Severity::High))
        .collect();
    let other = findings.len() - critical.len() - high.len();

    let mut summary = format!(
        "Known findings: {} critical, {} high, {} other\n",
        critical.len(),
        high.len(),
        other
    );

    // List critical/high briefly
    for f in critical.iter().chain(high.iter()).take(5) {
        summary.push_str(&format!("- {}: {} ({})\n", f.id, f.title, f.status.as_str()));
    }

    if critical.len() + high.len() > 5 {
        summary.push_str(&format!(
            "... and {} more critical/high findings\n",
            critical.len() + high.len() - 5
        ));
    }

    summary
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_context() {
        let ctx = InjectedContext::empty();
        assert!(ctx.is_empty());
        assert!(ctx.to_system_prompt().is_empty());
    }

    #[test]
    fn test_context_with_project() {
        let ctx = InjectedContext {
            project_info: Some("- **Project:** test-project".to_string()),
            known_findings: None,
            focus_findings: None,
            scope_info: None,
            tool_policy: None,
            output_schema: None,
            project_memory: None,
        };

        assert!(!ctx.is_empty());
        let prompt = ctx.to_system_prompt();
        assert!(prompt.contains("Project Context"));
        assert!(prompt.contains("test-project"));
    }

    #[test]
    fn test_findings_summary() {
        let findings = vec![
            Finding::new("VULN-001", "test", "Critical SQLi").with_severity(Severity::Critical),
            Finding::new("VULN-002", "test", "High XSS").with_severity(Severity::High),
            Finding::new("VULN-003", "test", "Medium CSRF").with_severity(Severity::Medium),
        ];

        let summary = generate_findings_summary(&findings);
        assert!(summary.contains("1 critical"));
        assert!(summary.contains("1 high"));
        assert!(summary.contains("VULN-001"));
    }
}
