//! Group manager for multi-agent parallel execution
//!
//! The GroupManager handles the lifecycle of agent run groups, which enable
//! running the same task on multiple agents in parallel and comparing results.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{AgentGroupId, AgentRunGroup, GroupStatus, Job, JobId, JobStatus};

/// Manages agent run groups for parallel multi-agent execution
pub struct GroupManager {
    /// All known groups
    groups: HashMap<AgentGroupId, AgentRunGroup>,

    /// Next group ID
    next_id: AtomicU64,
}

impl GroupManager {
    /// Create a new group manager
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Create a new agent run group
    pub fn create_group(&mut self, prompt: String, mode: String, target: String) -> AgentGroupId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let group = AgentRunGroup::new(id, prompt, mode, target);
        self.groups.insert(id, group);
        id
    }

    /// Add a job to a group
    pub fn add_job_to_group(&mut self, group_id: AgentGroupId, job_id: JobId, agent_name: String) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.add_job(job_id, agent_name);
        }
    }

    /// Get a group by ID
    pub fn get(&self, id: AgentGroupId) -> Option<&AgentRunGroup> {
        self.groups.get(&id)
    }

    /// Get a mutable group by ID
    pub fn get_mut(&mut self, id: AgentGroupId) -> Option<&mut AgentRunGroup> {
        self.groups.get_mut(&id)
    }

    /// Get all groups
    pub fn groups(&self) -> Vec<&AgentRunGroup> {
        self.groups.values().collect()
    }

    /// Get all groups in "Comparing" status (ready for user selection)
    pub fn groups_comparing(&self) -> Vec<&AgentRunGroup> {
        self.groups
            .values()
            .filter(|g| g.status == GroupStatus::Comparing)
            .collect()
    }

    /// Get all active groups (running or comparing)
    pub fn active_groups(&self) -> Vec<&AgentRunGroup> {
        self.groups
            .values()
            .filter(|g| matches!(g.status, GroupStatus::Running | GroupStatus::Comparing))
            .collect()
    }

    /// Update group status based on job statuses
    ///
    /// This should be called when a job's status changes to check if
    /// all jobs in the group are now finished.
    pub fn update_group_status(&mut self, group_id: AgentGroupId, jobs: &[&Job]) {
        let group = match self.groups.get_mut(&group_id) {
            Some(g) => g,
            None => return,
        };

        // Only update if group is still running
        if group.status != GroupStatus::Running {
            return;
        }

        // Get jobs that belong to this group
        let group_jobs: Vec<_> = jobs
            .iter()
            .filter(|j| j.group_id == Some(group_id))
            .collect();

        // Check if all jobs are finished
        let all_finished = group_jobs.iter().all(|j| j.is_finished());
        let any_succeeded = group_jobs.iter().any(|j| j.status == JobStatus::Done);

        if all_finished {
            if any_succeeded {
                // At least one job succeeded - ready for comparison
                group.set_status(GroupStatus::Comparing);
            } else {
                // All jobs failed - cancel the group
                group.set_status(GroupStatus::Cancelled);
            }
        }
    }

    /// Select a job as the winning result for a group
    pub fn select_result(&mut self, group_id: AgentGroupId, job_id: JobId) -> bool {
        if let Some(group) = self.groups.get_mut(&group_id) {
            if group.job_ids.contains(&job_id) {
                group.select_job(job_id);
                return true;
            }
        }
        false
    }

    /// Mark a group as merged
    pub fn mark_merged(&mut self, group_id: AgentGroupId) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.mark_merged();
        }
    }

    /// Cancel a group
    pub fn cancel_group(&mut self, group_id: AgentGroupId) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.set_status(GroupStatus::Cancelled);
        }
    }

    /// Get the group ID for a job (if any)
    pub fn group_for_job(&self, job_id: JobId) -> Option<AgentGroupId> {
        for group in self.groups.values() {
            if group.job_ids.contains(&job_id) {
                return Some(group.id);
            }
        }
        None
    }

    /// Get all job IDs in a group (for cleanup)
    pub fn job_ids_in_group(&self, group_id: AgentGroupId) -> Vec<JobId> {
        self.groups
            .get(&group_id)
            .map(|g| g.job_ids.clone())
            .unwrap_or_default()
    }

    /// Remove a group (after cleanup)
    pub fn remove_group(&mut self, group_id: AgentGroupId) -> Option<AgentRunGroup> {
        self.groups.remove(&group_id)
    }

    /// Check if a group has a selected result
    pub fn has_selection(&self, group_id: AgentGroupId) -> bool {
        self.groups
            .get(&group_id)
            .map(|g| g.selected_job.is_some())
            .unwrap_or(false)
    }

    /// Get the count of active groups
    pub fn active_count(&self) -> usize {
        self.groups
            .values()
            .filter(|g| matches!(g.status, GroupStatus::Running | GroupStatus::Comparing))
            .count()
    }

    /// Get the count of groups awaiting comparison
    pub fn comparing_count(&self) -> usize {
        self.groups
            .values()
            .filter(|g| g.status == GroupStatus::Comparing)
            .count()
    }
}

impl Default for GroupManager {
    fn default() -> Self {
        Self::new()
    }
}
