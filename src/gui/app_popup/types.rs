//! Types for apply/merge popup operations.

use crate::{AgentGroupId, JobId};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) enum ApplyTarget {
    Single {
        job_id: JobId,
    },
    Group {
        group_id: AgentGroupId,
        selected_job_id: JobId,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct ApplyThreadOutcome {
    pub(in crate::gui) target: ApplyTarget,
    /// For group merges: all job IDs in the group (empty for single jobs).
    pub(in crate::gui) group_job_ids: Vec<JobId>,
    pub(in crate::gui) message: String,
}

#[derive(Debug, Clone)]
pub(crate) enum ApplyThreadInput {
    Single(SingleApplyInput),
    Group(GroupApplyInput),
}

#[derive(Debug, Clone)]
pub(crate) struct SingleApplyInput {
    pub(super) job_id: JobId,
    pub(super) workspace_root: PathBuf,
    pub(super) worktree_path: Option<PathBuf>,
    pub(super) base_branch: Option<String>,
    pub(super) commit_message: crate::git::CommitMessage,
}

#[derive(Debug, Clone)]
pub(crate) struct GroupApplyInput {
    pub(super) group_id: AgentGroupId,
    pub(super) selected_job_id: JobId,
    pub(super) selected_agent_id: String,
    pub(super) workspace_root: PathBuf,
    pub(super) selected_worktree_path: PathBuf,
    pub(super) base_branch: String,
    pub(super) commit_message: crate::git::CommitMessage,
    pub(super) cleanup_worktrees: Vec<(JobId, PathBuf)>,
    pub(super) group_job_ids: Vec<JobId>,
}
