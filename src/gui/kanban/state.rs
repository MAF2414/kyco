//! Kanban board state management

use crate::bugbounty::{
    BugBountyJob, BugBountyManager, Finding, FindingStatus, FlowTrace, Project, ProjectStats,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KanbanTab {
    Dashboard,
    Board,
    Jobs,
}

/// State for the Kanban board view
pub struct KanbanState {
    /// BugBounty manager instance
    manager: Option<BugBountyManager>,

    /// Cached projects
    pub projects: Vec<Project>,

    /// Findings grouped by project ID
    pub findings_by_project: HashMap<String, Vec<Finding>>,

    /// Cached aggregated stats per project
    pub project_stats: HashMap<String, ProjectStats>,

    /// Cached job counts per finding
    pub job_count_by_finding: HashMap<String, usize>,

    /// Currently selected project
    pub selected_project: Option<String>,

    /// Currently selected finding (for detail panel)
    pub selected_finding: Option<String>,

    /// Finding being dragged
    pub dragged_finding_id: Option<String>,

    /// Whether data needs refresh
    pub needs_refresh: bool,

    /// Show new finding dialog
    pub show_new_finding_dialog: bool,

    /// New finding form state
    pub new_finding_title: String,
    pub new_finding_severity: String,

    /// Selected tab for the current project
    pub selected_tab: KanbanTab,

    /// Show mark-FP dialog
    pub show_fp_dialog: bool,
    pub fp_target_finding_id: Option<String>,
    pub fp_reason_input: String,

    /// Jobs view filters (applied to KYCo jobs)
    pub jobs_filter_agent: Option<String>,
    pub jobs_filter_state: String,
    pub jobs_filter_file: String,
    pub jobs_filter_finding: String,

    /// Status message
    pub status_message: Option<(String, bool)>,

    /// Export result
    pub last_export: Option<String>,

    /// Cached flow trace for selected finding
    pub cached_flow_trace: Option<FlowTrace>,

    /// Cached linked jobs for selected finding (finding_id, jobs)
    pub cached_linked_jobs: Option<(String, Vec<BugBountyJob>)>,

    /// Cached recent jobs for selected project (project_id, jobs)
    pub cached_recent_jobs: Option<(String, Vec<BugBountyJob>)>,
}

impl Default for KanbanState {
    fn default() -> Self {
        Self::new()
    }
}

impl KanbanState {
    /// Create a new Kanban state
    pub fn new() -> Self {
        let mut state = Self {
            manager: None,
            projects: Vec::new(),
            findings_by_project: HashMap::new(),
            project_stats: HashMap::new(),
            job_count_by_finding: HashMap::new(),
            selected_project: None,
            selected_finding: None,
            dragged_finding_id: None,
            needs_refresh: true,
            show_new_finding_dialog: false,
            new_finding_title: String::new(),
            new_finding_severity: "medium".to_string(),
            selected_tab: KanbanTab::Board,
            show_fp_dialog: false,
            fp_target_finding_id: None,
            fp_reason_input: String::new(),
            jobs_filter_agent: None,
            jobs_filter_state: String::new(),
            jobs_filter_file: String::new(),
            jobs_filter_finding: String::new(),
            status_message: None,
            last_export: None,
            cached_flow_trace: None,
            cached_linked_jobs: None,
            cached_recent_jobs: None,
        };

        // Try to initialize manager
        if let Ok(manager) = BugBountyManager::new() {
            state.manager = Some(manager);
        }

        // Load persisted active project
        state.selected_project = load_active_project();

        state
    }

    /// Refresh data from database
    pub fn refresh(&mut self) {
        self.needs_refresh = false;
        self.cached_flow_trace = None;
        self.cached_linked_jobs = None;
        self.cached_recent_jobs = None;
        self.project_stats.clear();

        let manager = match &self.manager {
            Some(m) => m,
            None => {
                // Try to reconnect
                if let Ok(m) = BugBountyManager::new() {
                    self.manager = Some(m);
                } else {
                    self.status_message = Some(("Failed to connect to database".to_string(), false));
                    return;
                }
                self.manager.as_ref().unwrap()
            }
        };

        // Load projects
        match manager.list_projects() {
            Ok(projects) => {
                self.projects = projects;
            }
            Err(e) => {
                self.status_message = Some((format!("Failed to load projects: {}", e), false));
                return;
            }
        }

        // Load findings for all projects (or just selected)
        self.findings_by_project.clear();
        self.job_count_by_finding.clear();

        let projects_to_load = if let Some(ref pid) = self.selected_project {
            vec![pid.clone()]
        } else {
            self.projects.iter().map(|p| p.id.clone()).collect()
        };

        for project_id in projects_to_load {
            if let Ok(stats) = manager.projects().get_stats(&project_id) {
                self.project_stats.insert(project_id.clone(), stats);
            }

            match manager.list_findings_by_project(&project_id) {
                Ok(findings) => {
                    for finding in &findings {
                        if let Ok(job_ids) = manager.job_findings().list_jobs_for_finding(&finding.id) {
                            if !job_ids.is_empty() {
                                self.job_count_by_finding.insert(finding.id.clone(), job_ids.len());
                            }
                        }
                    }
                    self.findings_by_project.insert(project_id, findings);
                }
                Err(e) => {
                    tracing::warn!("Failed to load findings for {}: {}", project_id, e);
                }
            }
        }
    }

    /// Move a finding to a new status
    pub fn move_finding(&mut self, finding_id: &str, new_status: FindingStatus) {
        let manager = match &self.manager {
            Some(m) => m,
            None => return,
        };

        if let Err(e) = manager.set_finding_status(finding_id, new_status) {
            self.status_message = Some((format!("Failed to update status: {}", e), false));
            return;
        }

        // Update local cache
        for findings in self.findings_by_project.values_mut() {
            for finding in findings.iter_mut() {
                if finding.id == finding_id {
                    finding.status = new_status;
                    break;
                }
            }
        }

        self.status_message = Some((
            format!("Moved {} to {}", finding_id, new_status.as_str()),
            true,
        ));
    }

    /// Create a new finding
    pub fn create_finding(&mut self, title: &str, severity: &str) {
        let manager = match &self.manager {
            Some(m) => m,
            None => return,
        };

        let project_id = match &self.selected_project {
            Some(p) => p.clone(),
            None => return,
        };

        // Get next finding number
        let number = match manager.next_finding_number(&project_id) {
            Ok(n) => n,
            Err(e) => {
                self.status_message = Some((format!("Failed to get finding number: {}", e), false));
                return;
            }
        };

        let mut finding = Finding::new(
            Finding::generate_id(&project_id, number),
            &project_id,
            title,
        );

        if let Some(sev) = crate::bugbounty::Severity::from_str(severity) {
            finding = finding.with_severity(sev);
        }

        if let Err(e) = manager.create_finding(&finding) {
            self.status_message = Some((format!("Failed to create finding: {}", e), false));
            return;
        }

        self.status_message = Some((format!("Created {}", finding.id), true));
        self.needs_refresh = true;
        self.show_new_finding_dialog = false;
        self.new_finding_title.clear();
    }

    /// Export a finding to a format
    pub fn export_finding(&mut self, finding_id: &str, format: &str) {
        let manager = match &self.manager {
            Some(m) => m,
            None => return,
        };

        let finding = match manager.get_finding(finding_id) {
            Ok(Some(f)) => f,
            _ => {
                self.status_message = Some(("Finding not found".to_string(), false));
                return;
            }
        };

        let content = match format {
            "markdown" | "md" => export_markdown(&finding),
            "hackerone" | "h1" => export_hackerone(&finding),
            "intigriti" => export_intigriti(&finding),
            _ => {
                self.status_message = Some(("Unknown format".to_string(), false));
                return;
            }
        };

        // Copy to clipboard if possible
        #[cfg(feature = "clipboard")]
        {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&content);
                self.status_message = Some(("Copied to clipboard".to_string(), true));
            }
        }

        self.last_export = Some(content);
        self.status_message = Some((format!("Exported {} as {}", finding_id, format), true));
    }

    /// Select a project and persist the selection
    pub fn select_project(&mut self, project_id: Option<String>) {
        self.selected_project = project_id.clone();
        save_active_project(project_id.as_deref());
        self.selected_tab = KanbanTab::Board;
        self.needs_refresh = true;
    }

    pub fn mark_false_positive(&mut self, finding_id: &str, reason: &str) {
        let manager = match &self.manager {
            Some(m) => m,
            None => return,
        };

        if let Err(e) = manager.findings().mark_false_positive(finding_id, reason) {
            self.status_message = Some((format!("Failed to mark FP: {}", e), false));
            return;
        }

        // Update local cache
        for findings in self.findings_by_project.values_mut() {
            for finding in findings.iter_mut() {
                if finding.id == finding_id {
                    finding.status = FindingStatus::FalsePositive;
                    finding.fp_reason = Some(reason.to_string());
                    break;
                }
            }
        }

        self.status_message = Some((format!("Marked {} as FP", finding_id), true));
    }

    /// Load flow trace for a finding
    pub fn load_flow_trace(&mut self, finding_id: &str) {
        let manager = match &self.manager {
            Some(m) => m,
            None => return,
        };

        match manager.get_flow_trace(finding_id) {
            Ok(trace) => {
                self.cached_flow_trace = Some(trace);
            }
            Err(e) => {
                tracing::warn!("Failed to load flow trace for {}: {}", finding_id, e);
                self.cached_flow_trace = None;
            }
        }
    }

    /// Load linked jobs for a finding (best-effort)
    pub fn load_linked_jobs(&mut self, finding_id: &str) {
        let manager = match &self.manager {
            Some(m) => m,
            None => return,
        };

        let job_ids = match manager.job_findings().list_jobs_for_finding(finding_id) {
            Ok(ids) => ids,
            Err(e) => {
                tracing::warn!("Failed to load job links for {}: {}", finding_id, e);
                self.cached_linked_jobs = Some((finding_id.to_string(), Vec::new()));
                return;
            }
        };

        let mut jobs: Vec<BugBountyJob> = Vec::new();
        for job_id in job_ids {
            if let Ok(Some(job)) = manager.jobs().get(&job_id) {
                jobs.push(job);
            }
        }
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        self.cached_linked_jobs = Some((finding_id.to_string(), jobs));
    }

    pub fn load_recent_jobs(&mut self, project_id: &str, limit: usize) {
        let manager = match &self.manager {
            Some(m) => m,
            None => return,
        };

        match manager.jobs().list_recent_by_project(project_id, limit) {
            Ok(jobs) => {
                self.cached_recent_jobs = Some((project_id.to_string(), jobs));
            }
            Err(e) => {
                tracing::warn!("Failed to load recent jobs for {}: {}", project_id, e);
                self.cached_recent_jobs = Some((project_id.to_string(), Vec::new()));
            }
        }
    }

    /// Process a job's output to extract findings
    /// Returns the number of findings extracted
    pub fn process_job_output(&mut self, job_response: &str, job_id: Option<&str>) -> Option<usize> {
        let project_id = self.selected_project.as_ref()?;
        let manager = self.manager.as_ref()?;

        match manager.process_agent_output(project_id, job_response, job_id) {
            Ok(Some(ids)) => {
                let count = ids.len();
                self.status_message = Some((format!("Extracted {} findings from job output", count), true));
                self.needs_refresh = true;
                Some(count)
            }
            Ok(None) => {
                self.status_message = Some(("No findings found in job output".to_string(), false));
                None
            }
            Err(e) => {
                self.status_message = Some((format!("Failed to process job output: {}", e), false));
                None
            }
        }
    }
}

// Export functions (duplicated from cli/finding.rs for GUI use)
fn export_markdown(f: &Finding) -> String {
    let mut s = String::new();
    s.push_str(&format!("# {}: {}\n\n", f.id, f.title));
    s.push_str(&format!(
        "**Severity:** {}  \n",
        f.severity.map(|s| s.as_str()).unwrap_or("-")
    ));
    s.push_str(&format!("**Status:** {}  \n", f.status.as_str()));

    if let Some(ref cwe) = f.cwe_id {
        s.push_str(&format!("**CWE:** {}  \n", cwe));
    }

    s.push_str("\n## Attack Scenario\n\n");
    s.push_str(f.attack_scenario.as_deref().unwrap_or("(not specified)"));
    s.push_str("\n\n## Impact\n\n");
    s.push_str(f.impact.as_deref().unwrap_or("(not specified)"));

    if !f.affected_assets.is_empty() {
        s.push_str("\n\n## Affected Assets\n\n");
        for asset in &f.affected_assets {
            s.push_str(&format!("- {}\n", asset));
        }
    }

    s
}

fn export_hackerone(f: &Finding) -> String {
    let mut s = String::new();
    s.push_str(&format!("## Summary\n\n{}\n\n", f.title));

    if let Some(ref scenario) = f.attack_scenario {
        s.push_str(&format!("{}\n\n", scenario));
    }

    s.push_str("## Steps To Reproduce\n\n1. (Add steps)\n\n");
    s.push_str("## Impact\n\n");
    s.push_str(f.impact.as_deref().unwrap_or("(Describe impact)"));

    s
}

fn export_intigriti(f: &Finding) -> String {
    let mut s = String::new();
    s.push_str(&format!("{}\n\n", f.title));
    s.push_str("Summary\n\n");
    s.push_str(f.attack_scenario.as_deref().unwrap_or("(not specified)"));
    s.push_str("\n\nSteps to reproduce\n\n1. (Add steps)\n\n");
    s.push_str("Impact\n\n");
    s.push_str(f.impact.as_deref().unwrap_or("(not specified)"));
    s
}

// Active project persistence
fn load_active_project() -> Option<String> {
    let path = dirs::home_dir()?.join(".kyco").join("active_project");
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn save_active_project(project_id: Option<&str>) {
    if let Some(home) = dirs::home_dir() {
        let path = home.join(".kyco").join("active_project");
        if let Some(pid) = project_id {
            let _ = std::fs::write(&path, pid);
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }
}
