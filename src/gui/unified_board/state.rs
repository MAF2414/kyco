//! State management for the Unified Board (Jobs + Findings)

use crate::bugbounty::{BugBountyJob, BugBountyManager, Finding, FindingStatus, Project, ProjectStats};
use crate::job::JobManager;
use crate::{Job, JobId, JobStatus};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Tab selection for the unified board
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnifiedBoardTab {
    Board,
    Dashboard,
}

/// State for the unified board view
pub struct UnifiedBoardState {
    /// BugBounty manager instance
    manager: Option<BugBountyManager>,

    /// Cached projects
    pub projects: Vec<Project>,

    /// Findings grouped by project ID
    pub findings_by_project: HashMap<String, Vec<Finding>>,

    /// Cached aggregated stats per project
    pub project_stats: HashMap<String, ProjectStats>,

    /// Currently selected project
    pub selected_project: Option<String>,

    /// Currently selected finding (for detail panel)
    pub selected_finding: Option<String>,

    /// Currently selected job (for detail panel)
    pub selected_job: Option<JobId>,

    /// Finding being dragged
    pub dragged_finding_id: Option<String>,

    /// Job being dragged
    pub dragged_job_id: Option<JobId>,

    /// Whether data needs refresh
    pub needs_refresh: bool,

    /// Selected tab
    pub selected_tab: UnifiedBoardTab,

    /// Job-Finding links (job_id -> finding_ids)
    pub job_finding_links: HashMap<String, Vec<String>>,

    /// Finding-Job links (finding_id -> job_ids) - reverse lookup
    pub finding_job_links: HashMap<String, Vec<String>>,

    /// Status message
    pub status_message: Option<(String, bool)>,

    /// Show new finding dialog
    pub show_new_finding_dialog: bool,
    pub new_finding_title: String,
    pub new_finding_severity: String,

    /// Show FP dialog
    pub show_fp_dialog: bool,
    pub fp_target_finding_id: Option<String>,
    pub fp_reason_input: String,

    /// Cached linked jobs for selected finding
    pub cached_linked_jobs: Option<(String, Vec<BugBountyJob>)>,

    /// Last generation of job manager (for detecting changes)
    last_job_generation: u64,

    /// Request to close the board and return to job list
    pub close_requested: bool,
}

impl Default for UnifiedBoardState {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedBoardState {
    /// Create a new unified board state
    pub fn new() -> Self {
        let mut state = Self {
            manager: None,
            projects: Vec::new(),
            findings_by_project: HashMap::new(),
            project_stats: HashMap::new(),
            selected_project: None,
            selected_finding: None,
            selected_job: None,
            dragged_finding_id: None,
            dragged_job_id: None,
            needs_refresh: true,
            selected_tab: UnifiedBoardTab::Board,
            job_finding_links: HashMap::new(),
            finding_job_links: HashMap::new(),
            status_message: None,
            show_new_finding_dialog: false,
            new_finding_title: String::new(),
            new_finding_severity: "medium".to_string(),
            show_fp_dialog: false,
            fp_target_finding_id: None,
            fp_reason_input: String::new(),
            cached_linked_jobs: None,
            last_job_generation: 0,
            close_requested: false,
        };

        // Try to initialize manager
        if let Ok(manager) = BugBountyManager::new() {
            state.manager = Some(manager);
        }

        // Load persisted active project
        state.selected_project = load_active_project();

        state
    }

    /// Check if job manager changed and trigger refresh
    pub fn check_job_changes(&mut self, job_manager: &Arc<Mutex<JobManager>>) {
        if let Ok(manager) = job_manager.lock() {
            let generation = manager.generation();
            if generation != self.last_job_generation {
                self.last_job_generation = generation;
                self.needs_refresh = true;
            }
        }
    }

    /// Refresh data from database and job manager
    pub fn refresh(&mut self, job_manager: &Arc<Mutex<JobManager>>) {
        self.needs_refresh = false;
        self.cached_linked_jobs = None;
        self.project_stats.clear();
        self.job_finding_links.clear();
        self.finding_job_links.clear();

        let manager = match &self.manager {
            Some(m) => m,
            None => {
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

        // Load findings for selected project
        self.findings_by_project.clear();

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
                    // Build finding -> job links
                    for finding in &findings {
                        if let Ok(job_ids) = manager.job_findings().list_jobs_for_finding(&finding.id) {
                            if !job_ids.is_empty() {
                                self.finding_job_links.insert(finding.id.clone(), job_ids.clone());
                                // Reverse lookup
                                for job_id in job_ids {
                                    self.job_finding_links
                                        .entry(job_id)
                                        .or_default()
                                        .push(finding.id.clone());
                                }
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

        // Update job generation
        if let Ok(jm) = job_manager.lock() {
            self.last_job_generation = jm.generation();
        }
    }

    /// Get jobs for the selected project
    pub fn get_project_jobs<'a>(&self, all_jobs: &'a [Job]) -> Vec<&'a Job> {
        let project_id = match &self.selected_project {
            Some(id) => id,
            None => return Vec::new(),
        };

        all_jobs
            .iter()
            .filter(|j| j.bugbounty_project_id.as_deref() == Some(project_id.as_str()))
            .collect()
    }

    /// Get jobs grouped by status
    pub fn get_jobs_by_status<'a>(&self, all_jobs: &'a [Job]) -> HashMap<JobStatus, Vec<&'a Job>> {
        let project_jobs = self.get_project_jobs(all_jobs);
        let mut grouped: HashMap<JobStatus, Vec<&Job>> = HashMap::new();

        for job in project_jobs {
            grouped.entry(job.status).or_default().push(job);
        }

        grouped
    }

    /// Get findings for the selected project
    pub fn get_project_findings(&self) -> Vec<&Finding> {
        let project_id = match &self.selected_project {
            Some(id) => id,
            None => return Vec::new(),
        };

        self.findings_by_project
            .get(project_id)
            .map(|f| f.iter().collect())
            .unwrap_or_default()
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

    /// Queue a job (change status from Pending to Queued)
    pub fn queue_job(&mut self, job_id: JobId, job_manager: &Arc<Mutex<JobManager>>) {
        if let Ok(mut manager) = job_manager.lock() {
            manager.set_status(job_id, JobStatus::Queued);
            self.status_message = Some((format!("Queued job #{}", job_id), true));
        }
    }

    /// Kill a job (set to Failed)
    pub fn kill_job(&mut self, job_id: JobId, job_manager: &Arc<Mutex<JobManager>>) {
        if let Ok(mut manager) = job_manager.lock() {
            if let Some(job) = manager.get_mut(job_id) {
                job.error_message = Some("Killed by user".to_string());
            }
            manager.set_status(job_id, JobStatus::Failed);
            self.status_message = Some((format!("Killed job #{}", job_id), true));
        }
    }

    /// Get linked finding IDs for a job
    pub fn get_linked_findings(&self, job: &Job) -> Vec<String> {
        // First check job's own finding IDs
        if !job.bugbounty_finding_ids.is_empty() {
            return job.bugbounty_finding_ids.clone();
        }

        // Then check our cache from job_findings table
        // Note: KYCo job ID needs to be mapped to BugBounty job ID
        // For now, return empty if no direct links
        Vec::new()
    }

    /// Get linked job count for a finding
    pub fn get_linked_job_count(&self, finding_id: &str) -> usize {
        self.finding_job_links
            .get(finding_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Select a project and persist the selection
    pub fn select_project(&mut self, project_id: Option<String>) {
        self.selected_project = project_id.clone();
        save_active_project(project_id.as_deref());
        self.selected_tab = UnifiedBoardTab::Board;
        self.needs_refresh = true;
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

    /// Mark a finding as false positive
    pub fn mark_false_positive(&mut self, finding_id: &str, reason: &str) {
        let manager = match &self.manager {
            Some(m) => m,
            None => return,
        };

        if let Err(e) = manager.findings().mark_false_positive(finding_id, reason) {
            self.status_message = Some((format!("Failed to mark FP: {}", e), false));
            return;
        }

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

    /// Load linked jobs for a finding
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
