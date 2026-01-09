//! Process registry for running CLI agent jobs.
//!
//! The SDK bridge adapters can interrupt sessions via HTTP. For direct CLI execution,
//! we need a way to interrupt/kill the underlying process by job id.

use anyhow::Result;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RunningProcess {
    pub pid: u32,
    pub agent_id: String,
}

static RUNNING: Lazy<Mutex<HashMap<u64, RunningProcess>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn register(job_id: u64, pid: u32, agent_id: impl Into<String>) {
    let mut guard = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(
        job_id,
        RunningProcess {
            pid,
            agent_id: agent_id.into(),
        },
    );
}

pub fn unregister(job_id: u64) {
    let mut guard = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    guard.remove(&job_id);
}

pub fn get(job_id: u64) -> Option<RunningProcess> {
    let guard = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    guard.get(&job_id).cloned()
}

/// Send SIGINT to the registered process for the given job id.
pub fn interrupt(job_id: u64) -> Result<bool> {
    let Some(proc) = get(job_id) else {
        return Ok(false);
    };

    #[cfg(unix)]
    unsafe {
        let rc = libc::kill(proc.pid as i32, libc::SIGINT);
        return Ok(rc == 0);
    }

    #[cfg(not(unix))]
    {
        let _ = proc;
        Ok(false)
    }
}

/// Send SIGKILL to the registered process for the given job id.
pub fn kill(job_id: u64) -> Result<bool> {
    let Some(proc) = get(job_id) else {
        return Ok(false);
    };

    #[cfg(unix)]
    unsafe {
        let rc = libc::kill(proc.pid as i32, libc::SIGKILL);
        return Ok(rc == 0);
    }

    #[cfg(not(unix))]
    {
        let _ = proc;
        Ok(false)
    }
}
