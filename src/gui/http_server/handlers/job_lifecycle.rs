//! Job lifecycle handlers: list, get, queue, abort.

use super::super::types::ControlApiState;
use super::super::respond_json;
use super::{parse_job_id_from_path, ExecutorEvent};
use crate::agent::bridge::BridgeClient;
use crate::{Job, JobStatus, LogEvent};

pub fn handle_control_jobs_list(control: &ControlApiState, request: tiny_http::Request) {
    let jobs: Vec<Job> = match control.job_manager.lock() {
        Ok(manager) => manager.jobs().into_iter().cloned().collect(),
        Err(_) => {
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "job_manager_lock" }),
            );
            return;
        }
    };

    respond_json(request, 200, serde_json::json!({ "jobs": jobs }));
}

pub fn handle_control_job_get(control: &ControlApiState, path: &str, request: tiny_http::Request) {
    let job_id = match parse_job_id_from_path(path, None) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

    let job = match control.job_manager.lock() {
        Ok(manager) => manager.get(job_id).cloned(),
        Err(_) => {
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "job_manager_lock" }),
            );
            return;
        }
    };

    let Some(job) = job else {
        respond_json(request, 404, serde_json::json!({ "error": "not_found" }));
        return;
    };

    respond_json(request, 200, serde_json::json!({ "job": job }));
}

pub fn handle_control_job_queue(control: &ControlApiState, path: &str, request: tiny_http::Request) {
    let job_id = match parse_job_id_from_path(path, Some("queue")) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

    let status = match control.job_manager.lock() {
        Ok(mut manager) => match manager.get(job_id).is_some() {
            true => {
                manager.set_status(job_id, JobStatus::Queued);
                Some(JobStatus::Queued)
            }
            false => None,
        },
        Err(_) => {
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "job_manager_lock" }),
            );
            return;
        }
    };

    let Some(status) = status else {
        respond_json(request, 404, serde_json::json!({ "error": "not_found" }));
        return;
    };

    let _ = control
        .executor_tx
        .send(ExecutorEvent::Log(LogEvent::system(format!(
            "Queued job #{}",
            job_id
        ))));

    respond_json(
        request,
        200,
        serde_json::json!({ "status": "ok", "job_id": job_id, "job_status": status }),
    );
}

pub fn handle_control_job_abort(control: &ControlApiState, path: &str, request: tiny_http::Request) {
    let job_id = match parse_job_id_from_path(path, Some("abort")) {
        Ok(id) => id,
        Err(err) => {
            respond_json(request, 400, serde_json::json!({ "error": err }));
            return;
        }
    };

    let (agent_id, session_id, status) = match control.job_manager.lock() {
        Ok(mut manager) => match manager.get_mut(job_id) {
            Some(job) => {
                let status = job.status;
                let agent_id = job.agent_id.clone();
                let mut session_id = job.bridge_session_id.clone();

                let mut touch = false;
                let mut release_locks = false;

                // For running jobs: request cancellation, but keep status as Running until the
                // executor actually stops (prevents "can't stop" when session_id isn't known yet).
                if status == JobStatus::Running {
                    job.cancel_requested = true;
                    touch = true;
                } else if matches!(
                    status,
                    JobStatus::Queued | JobStatus::Pending | JobStatus::Blocked
                ) {
                    // For jobs that haven't started yet: fail immediately so the executor won't pick them up.
                    job.cancel_requested = true;
                    job.cancel_sent = true;
                    job.fail("Job aborted by user".to_string());
                    session_id = None;
                    release_locks = true;
                    touch = true;
                }

                // End the mutable borrow of `job` before calling other `manager` methods.
                if release_locks {
                    manager.release_job_locks(job_id);
                }
                if touch {
                    manager.touch();
                }

                (agent_id, session_id, status)
            }
            None => {
                respond_json(request, 404, serde_json::json!({ "error": "not_found" }));
                return;
            }
        },
        Err(_) => {
            respond_json(
                request,
                500,
                serde_json::json!({ "error": "job_manager_lock" }),
            );
            return;
        }
    };

    if matches!(
        status,
        JobStatus::Running | JobStatus::Queued | JobStatus::Pending | JobStatus::Blocked
    ) {
        if let Some(session_id) = session_id.as_deref() {
            let client = BridgeClient::new();
            let agent_id_lower = agent_id.to_ascii_lowercase();
            let likely_codex = agent_id_lower == "codex" || agent_id_lower.contains("codex");

            let interrupt_attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if likely_codex {
                    client
                        .interrupt_codex(session_id)
                        .or_else(|_| client.interrupt_claude(session_id))
                } else {
                    client
                        .interrupt_claude(session_id)
                        .or_else(|_| client.interrupt_codex(session_id))
                }
            }));

            match interrupt_attempt {
                Ok(Ok(true)) => {
                    // Mark that we successfully sent an interrupt signal.
                    if let Ok(mut manager) = control.job_manager.lock() {
                        if let Some(job) = manager.get_mut(job_id) {
                            job.cancel_sent = true;
                        }
                        manager.touch();
                    }
                    let _ = control
                        .executor_tx
                        .send(ExecutorEvent::Log(LogEvent::system(format!(
                            "Sent interrupt for job #{} (agent: {})",
                            job_id, agent_id
                        ))));
                }
                Ok(Ok(false)) => {
                    let _ = control
                        .executor_tx
                        .send(ExecutorEvent::Log(LogEvent::error(format!(
                            "Interrupt rejected for job #{} (agent: {})",
                            job_id, agent_id
                        ))));
                }
                Ok(Err(e)) => {
                    let _ = control
                        .executor_tx
                        .send(ExecutorEvent::Log(LogEvent::error(format!(
                            "Failed to interrupt job #{} (agent: {}): {}",
                            job_id, agent_id, e
                        ))));
                }
                Err(_) => {
                    let _ = control
                        .executor_tx
                        .send(ExecutorEvent::Log(LogEvent::error(format!(
                            "Bridge interrupt panicked (job #{})",
                            job_id
                        ))));
                }
            }
        } else if status == JobStatus::Running {
            let _ = control.executor_tx.send(ExecutorEvent::Log(LogEvent::system(
                format!("Stop requested for job #{} (waiting for session start)", job_id),
            )));
        }
        let _ = control.executor_tx.send(ExecutorEvent::Log(LogEvent::system(
            format!("Abort requested for job #{}", job_id),
        )));

        respond_json(
            request,
            200,
            serde_json::json!({ "status": "ok", "job_id": job_id }),
        );
        return;
    }

    respond_json(
        request,
        400,
        serde_json::json!({ "error": "not_abortable", "job_id": job_id, "status": status }),
    );
}
