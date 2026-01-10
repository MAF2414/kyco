//! Job restart handler: create a new job from a failed/rejected job's parameters.

use super::super::respond_json;
use super::{parse_job_id_from_path, ExecutorEvent};
use crate::gui::http_server::types::ControlApiState;
use crate::gui::jobs;
use crate::{CommentTag, JobStatus, LogEvent, Target};

pub fn handle_control_job_restart(
    control: &ControlApiState,
    path: &str,
    request: tiny_http::Request,
) {
    let job_id = match parse_job_id_from_path(path, Some("restart")) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

    let mut logs: Vec<LogEvent> = Vec::new();

    let created_id = {
        let mut manager = match control.job_manager.lock() {
            Ok(m) => m,
            Err(_) => {
                respond_json(
                    request,
                    500,
                    serde_json::json!({ "error": "job_manager_lock" }),
                );
                return;
            }
        };

        let Some(original) = manager.get(job_id).cloned() else {
            respond_json(request, 404, serde_json::json!({ "error": "not_found" }));
            return;
        };

        // Only allow restart for failed or rejected jobs
        if !matches!(original.status, JobStatus::Failed | JobStatus::Rejected) {
            respond_json(
                request,
                400,
                serde_json::json!({
                    "error": "not_restartable",
                    "message": "Only failed or rejected jobs can be restarted",
                    "status": original.status.to_string()
                }),
            );
            return;
        }

        let description = original.description.clone().unwrap_or_default();

        let tag = CommentTag {
            file_path: original.source_file.clone(),
            line_number: original.source_line,
            raw_line: String::new(),
            agent: original.agent_id.clone(),
            agents: vec![original.agent_id.clone()],
            mode: original.skill.clone(),
            target: Target::Block,
            status_marker: None,
            description: if description.is_empty() {
                None
            } else {
                Some(description)
            },
            job_id: None,
        };

        let new_job_id =
            match manager.create_job_with_range(&tag, &original.agent_id, None)
            {
                Ok(id) => id,
                Err(e) => {
                    respond_json(
                        request,
                        500,
                        serde_json::json!({ "error": "create_failed", "details": e.to_string() }),
                    );
                    return;
                }
            };

        // Copy relevant context from original job to new job
        if let Some(job) = manager.get_mut(new_job_id) {
            job.raw_tag_line = None;
            job.ide_context = original.ide_context;
            job.force_worktree = original.force_worktree;
            job.workspace_path = original.workspace_path.clone();
            job.scope = original.scope.clone();
            job.target = original.target;
        }

        logs.push(LogEvent::system(format!(
            "Restarted job #{} as #{}",
            job_id, new_job_id
        )));

        new_job_id
    };

    // Queue the new job immediately
    jobs::queue_job(&control.job_manager, created_id, &mut logs);

    for log in &logs {
        let _ = control.executor_tx.send(ExecutorEvent::Log(log.clone()));
    }

    respond_json(
        request,
        200,
        serde_json::json!({
            "status": "ok",
            "old_job_id": job_id,
            "new_job_id": created_id
        }),
    );
}
