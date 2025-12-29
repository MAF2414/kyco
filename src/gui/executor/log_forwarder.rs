//! Log forwarding utilities for the executor

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use crate::agent::bridge::BridgeClient;
use crate::job::JobManager;
use crate::{JobStatus, LogEvent, LogEventKind};

use super::ExecutorEvent;

/// Spawn a log forwarder task that processes log events and permission requests.
///
/// Returns a JoinHandle for the spawned task.
pub fn spawn_log_forwarder(
    mut log_rx: tokio::sync::mpsc::Receiver<LogEvent>,
    event_tx: Sender<ExecutorEvent>,
    job_manager: Arc<Mutex<JobManager>>,
    job_id: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(log) = log_rx.recv().await {
            if let Some(args) = log.tool_args.as_ref() {
                if let Some(session_id) = args.get("session_id").and_then(|v| v.as_str()) {
                    let mut maybe_interrupt: Option<(String, String)> = None;
                    if let Ok(mut manager) = job_manager.lock() {
                        if let Some(job) = manager.get_mut(job_id) {
                            job.bridge_session_id = Some(session_id.to_string());
                            if job.status == JobStatus::Running
                                && job.cancel_requested
                                && !job.cancel_sent
                            {
                                // Mark as in-flight to avoid spamming interrupts from repeated session_id logs.
                                job.cancel_sent = true;
                                maybe_interrupt =
                                    Some((job.agent_id.clone(), session_id.to_string()));
                            }
                        }
                        manager.touch();
                    }

                    if let Some((agent_id, session_id_owned)) = maybe_interrupt {
                        let event_tx = event_tx.clone();
                        let job_manager = Arc::clone(&job_manager);
                        tokio::task::spawn_blocking(move || {
                            let client = BridgeClient::new();
                            let agent_id_lower = agent_id.to_ascii_lowercase();
                            let likely_codex =
                                agent_id_lower == "codex" || agent_id_lower.contains("codex");

                            let interrupt_result = if likely_codex {
                                client
                                    .interrupt_codex(&session_id_owned)
                                    .or_else(|_| client.interrupt_claude(&session_id_owned))
                            } else {
                                client
                                    .interrupt_claude(&session_id_owned)
                                    .or_else(|_| client.interrupt_codex(&session_id_owned))
                            };

                            let (success, message) = match interrupt_result {
                                Ok(true) => (
                                    true,
                                    format!(
                                        "Sent interrupt for job #{} (agent: {})",
                                        job_id, agent_id
                                    ),
                                ),
                                Ok(false) => (
                                    false,
                                    format!(
                                        "Interrupt rejected for job #{} (agent: {})",
                                        job_id, agent_id
                                    ),
                                ),
                                Err(e) => (
                                    false,
                                    format!(
                                        "Failed to interrupt job #{} (agent: {}): {}",
                                        job_id, agent_id, e
                                    ),
                                ),
                            };

                            if !success {
                                if let Ok(mut manager) = job_manager.lock() {
                                    if let Some(job) = manager.get_mut(job_id) {
                                        job.cancel_sent = false;
                                    }
                                    manager.touch();
                                }
                                let _ = event_tx
                                    .send(ExecutorEvent::Log(LogEvent::error(message).for_job(job_id)));
                            } else {
                                let _ = event_tx
                                    .send(ExecutorEvent::Log(LogEvent::system(message).for_job(job_id)));
                            }
                        });
                    }
                }
            }

            if log.kind == LogEventKind::Permission {
                tracing::info!("⚠️ Log forwarder received Permission event for job {}", job_id);
                let args = match log.tool_args {
                    Some(a) => a,
                    None => {
                        tracing::warn!("⚠️ Permission event has no tool_args, skipping!");
                        continue;
                    }
                };

                let Some(request_id) = args
                    .get("request_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                else {
                    tracing::warn!("⚠️ Permission event missing request_id, skipping! args={:?}", args);
                    continue;
                };
                tracing::info!("⚠️ Forwarding PermissionNeeded event: request_id={}", request_id);

                let session_id = args
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let tool_name = args
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let tool_input = args
                    .get("tool_input")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect::<std::collections::HashMap<String, serde_json::Value>>()
                    })
                    .unwrap_or_default();

                let _ = event_tx.send(ExecutorEvent::PermissionNeeded {
                    job_id,
                    request_id,
                    session_id,
                    tool_name,
                    tool_input,
                });

                continue;
            }

            let _ = event_tx.send(ExecutorEvent::Log(log));
        }
    })
}
