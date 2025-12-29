//! Session continuation for REPL-style jobs

use super::super::app::KycoApp;
use crate::{JobId, LogEvent};

impl KycoApp {
    /// Continue a session job with a follow-up prompt
    pub(crate) fn continue_job_session(&mut self, job_id: JobId, prompt: String) {
        let (continuation_id, continuation_mode) = {
            let mut manager = match self.job_manager.lock() {
                Ok(m) => m,
                Err(_) => {
                    self.logs
                        .push(LogEvent::error("Failed to lock job manager"));
                    return;
                }
            };

            let Some(original) = manager.get(job_id).cloned() else {
                self.logs
                    .push(LogEvent::error(format!("Job #{} not found", job_id)));
                return;
            };

            let Some(session_id) = original.bridge_session_id.clone() else {
                self.logs.push(LogEvent::error(format!(
                    "Job #{} has no session to continue",
                    job_id
                )));
                return;
            };

            let tag = crate::CommentTag {
                file_path: original.source_file.clone(),
                line_number: original.source_line,
                raw_line: format!("// @{}:{} {}", &original.agent_id, &original.mode, &prompt),
                agent: original.agent_id.clone(),
                agents: vec![original.agent_id.clone()],
                mode: original.mode.clone(),
                target: crate::Target::Block,
                status_marker: None,
                description: Some(prompt),
                job_id: None,
            };

            let continuation_id =
                match manager.create_job_with_range(&tag, &original.agent_id, None) {
                    Ok(id) => id,
                    Err(e) => {
                        self.logs.push(LogEvent::error(format!(
                            "Failed to create continuation job: {}",
                            e
                        )));
                        return;
                    }
                };

            if let Some(job) = manager.get_mut(continuation_id) {
                job.raw_tag_line = None;
                job.bridge_session_id = Some(session_id);

                // Reuse the same worktree and job context
                job.git_worktree_path = original.git_worktree_path.clone();
                job.branch_name = original.branch_name.clone();
                job.base_branch = original.base_branch.clone();
                job.scope = original.scope.clone();
                job.target = original.target;
                job.ide_context = original.ide_context;
                job.force_worktree = original.force_worktree;
                job.workspace_path = original.workspace_path.clone();
            }

            (continuation_id, original.mode)
        };

        self.logs.push(LogEvent::system(format!(
            "Created continuation job #{} (mode: {})",
            continuation_id, continuation_mode
        )));

        self.queue_job(continuation_id);
        self.selected_job_id = Some(continuation_id);
        self.refresh_jobs();
    }
}
