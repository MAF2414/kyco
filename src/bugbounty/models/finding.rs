//! Finding model - the Kanban card for vulnerability tracking

use serde::{Deserialize, Serialize};

/// Severity levels for findings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
            Severity::Info => "info",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "critical" | "crit" => Some(Severity::Critical),
            "high" => Some(Severity::High),
            "medium" | "med" => Some(Severity::Medium),
            "low" => Some(Severity::Low),
            "info" | "informational" => Some(Severity::Info),
            _ => None,
        }
    }
}

/// Finding status - represents Kanban columns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    /// Fresh from agent, unreviewed
    Raw,
    /// Needs manual reproduction
    NeedsRepro,
    /// Verified/reproduced
    Verified,
    /// Report is being drafted
    ReportDraft,
    /// Submitted to platform
    Submitted,
    /// Platform triaged/acknowledged
    Triaged,
    /// Accepted by program
    Accepted,
    /// Bounty paid
    Paid,
    /// Marked as duplicate
    Duplicate,
    /// Won't fix (accepted risk)
    WontFix,
    /// False positive
    FalsePositive,
    /// Out of scope
    OutOfScope,
}

impl FindingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FindingStatus::Raw => "raw",
            FindingStatus::NeedsRepro => "needs_repro",
            FindingStatus::Verified => "verified",
            FindingStatus::ReportDraft => "report_draft",
            FindingStatus::Submitted => "submitted",
            FindingStatus::Triaged => "triaged",
            FindingStatus::Accepted => "accepted",
            FindingStatus::Paid => "paid",
            FindingStatus::Duplicate => "duplicate",
            FindingStatus::WontFix => "wont_fix",
            FindingStatus::FalsePositive => "false_positive",
            FindingStatus::OutOfScope => "out_of_scope",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "raw" => Some(FindingStatus::Raw),
            "needs_repro" | "needsrepro" => Some(FindingStatus::NeedsRepro),
            "verified" => Some(FindingStatus::Verified),
            "report_draft" | "reportdraft" | "draft" => Some(FindingStatus::ReportDraft),
            "submitted" => Some(FindingStatus::Submitted),
            "triaged" => Some(FindingStatus::Triaged),
            "accepted" => Some(FindingStatus::Accepted),
            "paid" => Some(FindingStatus::Paid),
            "duplicate" | "dupe" => Some(FindingStatus::Duplicate),
            "wont_fix" | "wontfix" => Some(FindingStatus::WontFix),
            "false_positive" | "fp" => Some(FindingStatus::FalsePositive),
            "out_of_scope" | "oos" => Some(FindingStatus::OutOfScope),
            _ => None,
        }
    }

    /// Returns true if this is a "terminal" state (no more action needed)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            FindingStatus::Paid
                | FindingStatus::Duplicate
                | FindingStatus::WontFix
                | FindingStatus::FalsePositive
                | FindingStatus::OutOfScope
        )
    }

    /// Returns true if this finding is actionable (needs attention)
    pub fn is_actionable(&self) -> bool {
        matches!(
            self,
            FindingStatus::Raw | FindingStatus::NeedsRepro | FindingStatus::ReportDraft
        )
    }

    /// Kanban column index (for sorting)
    pub fn column_index(&self) -> u8 {
        match self {
            FindingStatus::Raw => 0,
            FindingStatus::NeedsRepro => 1,
            FindingStatus::Verified => 2,
            FindingStatus::ReportDraft => 3,
            FindingStatus::Submitted => 4,
            FindingStatus::Triaged => 5,
            FindingStatus::Accepted => 6,
            FindingStatus::Paid => 7,
            FindingStatus::Duplicate => 8,
            FindingStatus::WontFix => 9,
            FindingStatus::FalsePositive => 10,
            FindingStatus::OutOfScope => 11,
        }
    }
}

/// Confidence level for findings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Confidence::High => "high",
            Confidence::Medium => "medium",
            Confidence::Low => "low",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "high" => Some(Confidence::High),
            "medium" | "med" => Some(Confidence::Medium),
            "low" => Some(Confidence::Low),
            _ => None,
        }
    }
}

/// Reachability of the vulnerable code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reachability {
    /// Publicly accessible without auth
    Public,
    /// Requires authentication
    AuthRequired,
    /// Internal only (not exposed)
    InternalOnly,
    /// Unknown/needs investigation
    Unknown,
}

impl Reachability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Reachability::Public => "public",
            Reachability::AuthRequired => "auth_required",
            Reachability::InternalOnly => "internal_only",
            Reachability::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "public" => Some(Reachability::Public),
            "auth_required" | "authrequired" | "auth" => Some(Reachability::AuthRequired),
            "internal_only" | "internalonly" | "internal" => Some(Reachability::InternalOnly),
            "unknown" => Some(Reachability::Unknown),
            _ => None,
        }
    }
}

/// A security finding (vulnerability)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Unique ID, e.g., "VULN-001"
    pub id: String,
    /// Project this finding belongs to
    pub project_id: String,
    /// Short title
    pub title: String,
    /// Severity level
    pub severity: Option<Severity>,
    /// Current status (Kanban column)
    pub status: FindingStatus,

    // Structured output fields (from security-audit profile)
    /// How an attacker would exploit this
    pub attack_scenario: Option<String>,
    /// What must be true for exploitation
    pub preconditions: Option<String>,
    /// Is the code reachable?
    pub reachability: Option<Reachability>,
    /// Impact description (CIA + business)
    pub impact: Option<String>,
    /// Confidence level with reasoning
    pub confidence: Option<Confidence>,

    // Optional classification
    /// CWE ID if applicable
    pub cwe_id: Option<String>,
    /// CVSS score if calculated
    pub cvss_score: Option<f64>,
    /// Affected assets (endpoints, domains, modules)
    pub affected_assets: Vec<String>,
    /// Taint path: Entry -> ... -> Sink
    pub taint_path: Option<String>,

    // Metadata
    /// Reason if marked false positive
    pub fp_reason: Option<String>,
    /// Additional notes
    pub notes: Option<String>,
    /// Path to source file (notes/findings/VULN-XXX.md)
    pub source_file: Option<String>,

    /// Created timestamp (ms since epoch)
    pub created_at: i64,
    /// Updated timestamp (ms since epoch)
    pub updated_at: i64,
}

impl Finding {
    /// Create a new finding with minimal required fields
    pub fn new(
        id: impl Into<String>,
        project_id: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: id.into(),
            project_id: project_id.into(),
            title: title.into(),
            severity: None,
            status: FindingStatus::Raw,
            attack_scenario: None,
            preconditions: None,
            reachability: None,
            impact: None,
            confidence: None,
            cwe_id: None,
            cvss_score: None,
            affected_assets: Vec::new(),
            taint_path: None,
            fp_reason: None,
            notes: None,
            source_file: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Generate the next finding ID for a project
    pub fn generate_id(project_id: &str, number: u32) -> String {
        format!("{project_id}-VULN-{:03}", number)
    }

    // Builder methods
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    pub fn with_status(mut self, status: FindingStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_attack_scenario(mut self, scenario: impl Into<String>) -> Self {
        self.attack_scenario = Some(scenario.into());
        self
    }

    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = Some(confidence);
        self
    }

    pub fn with_reachability(mut self, reachability: Reachability) -> Self {
        self.reachability = Some(reachability);
        self
    }

    pub fn with_impact(mut self, impact: impl Into<String>) -> Self {
        self.impact = Some(impact.into());
        self
    }

    pub fn with_cwe(mut self, cwe_id: impl Into<String>) -> Self {
        self.cwe_id = Some(cwe_id.into());
        self
    }

    pub fn with_affected_asset(mut self, asset: impl Into<String>) -> Self {
        self.affected_assets.push(asset.into());
        self
    }

    pub fn with_taint_path(mut self, taint_path: impl Into<String>) -> Self {
        self.taint_path = Some(taint_path.into());
        self
    }

    pub fn with_preconditions(mut self, preconditions: impl Into<String>) -> Self {
        self.preconditions = Some(preconditions.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finding_builder() {
        let finding = Finding::new("VULN-001", "hackerone-nextcloud", "IDOR in /api/users")
            .with_severity(Severity::High)
            .with_confidence(Confidence::High)
            .with_reachability(Reachability::AuthRequired)
            .with_attack_scenario("Attacker modifies user_id parameter")
            .with_cwe("CWE-639");

        assert_eq!(finding.id, "VULN-001");
        assert_eq!(finding.severity, Some(Severity::High));
        assert_eq!(finding.cwe_id, Some("CWE-639".to_string()));
    }

    #[test]
    fn test_status_parsing() {
        assert_eq!(FindingStatus::from_str("raw"), Some(FindingStatus::Raw));
        assert_eq!(
            FindingStatus::from_str("needs_repro"),
            Some(FindingStatus::NeedsRepro)
        );
        assert_eq!(FindingStatus::from_str("fp"), Some(FindingStatus::FalsePositive));
        assert_eq!(FindingStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_terminal_status() {
        assert!(FindingStatus::Paid.is_terminal());
        assert!(FindingStatus::FalsePositive.is_terminal());
        assert!(!FindingStatus::Raw.is_terminal());
        assert!(!FindingStatus::Verified.is_terminal());
    }
}
