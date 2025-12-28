//! Job lifecycle operations: kill and permission mode management

use super::super::app::KycoApp;
use super::super::jobs;
use crate::agent::bridge::PermissionMode;
use crate::{JobId, LogEvent, SdkType};

impl KycoApp {
    /// Kill/stop a running job
    pub(crate) fn kill_job(&mut self, job_id: JobId) {
        // First, mark the job as failed in the manager (even before interrupting)
        // This ensures the job state is updated even if interrupt fails
        jobs::kill_job(&self.job_manager, job_id, &mut self.logs);

        let session_info = self.job_manager.lock().ok().and_then(|manager| {
            manager.get(job_id).map(|job| {
                (
                    job.agent_id.clone(),
                    job.mode.clone(),
                    job.bridge_session_id.clone(),
                )
            })
        });

        let Some((agent_id, job_mode, session_id)) = session_info else {
            self.refresh_jobs();
            return;
        };

        // Try to interrupt the bridge session (best effort, non-critical)
        if let Some(session_id) = session_id.as_deref() {
            let sdk_type = self
                .config
                .read()
                .ok()
                .and_then(|cfg| cfg.get_agent_for_job(&agent_id, &job_mode))
                .map(|a| a.sdk_type)
                .unwrap_or_else(|| {
                    if agent_id == "codex" {
                        SdkType::Codex
                    } else {
                        SdkType::Claude
                    }
                });

            // Clone what we need to avoid capturing self in catch_unwind
            let client = self.bridge_client.clone();
            let session_id_owned = session_id.to_string();

            let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                if sdk_type == SdkType::Codex {
                    client.interrupt_codex(&session_id_owned)
                } else {
                    client.interrupt_claude(&session_id_owned)
                }
            }));

            match interrupted {
                Ok(Ok(true)) => self.logs.push(LogEvent::system(format!(
                    "Sent interrupt for job #{}",
                    job_id
                ))),
                Ok(Ok(false)) => self.logs.push(LogEvent::error(format!(
                    "Interrupt was rejected (job #{})",
                    job_id
                ))),
                Ok(Err(e)) => self.logs.push(LogEvent::error(format!(
                    "Failed to interrupt job #{}: {}",
                    job_id, e
                ))),
                Err(_) => self.logs.push(LogEvent::error(format!(
                    "Bridge interrupt panicked (job #{})",
                    job_id
                ))),
            };
        }

        self.refresh_jobs();
    }

    /// Set permission mode for a job's Claude session
    pub(crate) fn set_job_permission_mode(&mut self, job_id: JobId, mode: PermissionMode) {
        let (agent_id, job_mode, session_id) = {
            let manager = match self.job_manager.lock() {
                Ok(m) => m,
                Err(_) => {
                    self.logs
                        .push(LogEvent::error("Failed to lock job manager"));
                    return;
                }
            };

            match manager.get(job_id) {
                Some(job) => (
                    job.agent_id.clone(),
                    job.mode.clone(),
                    job.bridge_session_id.clone(),
                ),
                None => {
                    self.logs
                        .push(LogEvent::error(format!("Job #{} not found", job_id)));
                    return;
                }
            }
        };

        let is_codex = {
            let Ok(config) = self.config.read() else {
                self.logs.push(LogEvent::error("Config lock poisoned"));
                return;
            };
            config
                .get_agent_for_job(&agent_id, &job_mode)
                .map(|a| a.sdk_type == SdkType::Codex)
                .unwrap_or(agent_id == "codex")
        };

        if is_codex {
            self.logs.push(LogEvent::error(format!(
                "Permission mode switching is only supported for Claude sessions (job #{})",
                job_id
            )));
            return;
        }

        let Some(session_id) = session_id else {
            self.logs.push(LogEvent::error(format!(
                "Job #{} has no active Claude session yet",
                job_id
            )));
            return;
        };

        match self
            .bridge_client
            .set_claude_permission_mode(&session_id, mode)
        {
            Ok(true) => {
                self.permission_mode_overrides.insert(job_id, mode);
                self.logs.push(LogEvent::system(format!(
                    "Set permission mode to {} for job #{}",
                    match mode {
                        PermissionMode::Default => "default",
                        PermissionMode::AcceptEdits => "acceptEdits",
                        PermissionMode::BypassPermissions => "bypassPermissions",
                        PermissionMode::Plan => "plan",
                    },
                    job_id
                )));
            }
            Ok(false) => {
                self.logs.push(LogEvent::error(format!(
                    "Failed to set permission mode for job #{} (bridge rejected request)",
                    job_id
                )));
            }
            Err(e) => {
                self.logs.push(LogEvent::error(format!(
                    "Failed to set permission mode for job #{}: {}",
                    job_id, e
                )));
            }
        }
    }
}
