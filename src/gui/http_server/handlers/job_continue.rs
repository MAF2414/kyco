//! Job continuation handler.

use super::super::types::{
    ControlApiState, ControlJobContinueRequest, ControlJobContinueResponse,
};
use super::super::respond_json;
use super::{parse_job_id_from_path, ExecutorEvent};
use crate::gui::jobs;
use crate::{CommentTag, LogEvent, Target};

pub fn handle_control_job_continue(
    control: &ControlApiState,
    path: &str,
    body: &str,
    request: tiny_http::Request,
) {
    let job_id = match parse_job_id_from_path(path, Some("continue")) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

    let req: ControlJobContinueRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(e) => {
            respond_json(
                request,
                400,
                serde_json::json!({ "error": "invalid_json", "details": e.to_string() }),
            );
            return;
        }
    };

    let prompt = req.prompt.trim();
    if prompt.is_empty() {
        respond_json(
            request,
            400,
            serde_json::json!({ "error": "missing_prompt" }),
        );
        return;
    }

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

        let Some(session_id) = original.bridge_session_id.clone() else {
            respond_json(request, 400, serde_json::json!({ "error": "no_session" }));
            return;
        };

        let tag = CommentTag {
            file_path: original.source_file.clone(),
            line_number: original.source_line,
            raw_line: format!("// @{}:{} {}", &original.agent_id, &original.skill, prompt),
            agent: original.agent_id.clone(),
            agents: vec![original.agent_id.clone()],
            mode: original.skill.clone(),
            target: Target::Block,
            status_marker: None,
            description: Some(prompt.to_string()),
            job_id: None,
        };

        let continuation_id = match manager.create_job_with_range(&tag, &original.agent_id, None) {
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

        if let Some(job) = manager.get_mut(continuation_id) {
            job.raw_tag_line = None;
            job.bridge_session_id = Some(session_id);

            // Apply fork_session and plan_mode from request
            job.fork_session = req.fork_session;
            if req.plan_mode {
                job.permission_mode = Some("plan".to_string());
            }

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

        logs.push(LogEvent::system(format!(
            "Created continuation job #{} (from job #{})",
            continuation_id, job_id
        )));

        continuation_id
    };

    if req.queue {
        jobs::queue_job(&control.job_manager, created_id, &mut logs);
    }

    for log in &logs {
        let _ = control.executor_tx.send(ExecutorEvent::Log(log.clone()));
    }

    respond_json(
        request,
        200,
        serde_json::to_value(ControlJobContinueResponse { job_id: created_id })
            .unwrap_or_else(|_| serde_json::json!({ "error": "serialize" })),
    );
}
