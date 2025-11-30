//! Status command implementation

use anyhow::Result;
use std::path::Path;

use kyco::git::GitManager;
use kyco::job::JobManager;
use kyco::JobStatus;

/// Show the status of all jobs
pub async fn status_command(work_dir: &Path, filter: Option<String>) -> Result<()> {
    let mut manager = JobManager::load(work_dir)?;

    // Clean up worktrees for rejected jobs
    cleanup_rejected_worktrees(&mut manager, work_dir);

    let jobs = manager.jobs();

    let filtered_jobs: Vec<_> = if let Some(status_filter) = filter {
        let target_status = match status_filter.to_lowercase().as_str() {
            "pending" => Some(JobStatus::Pending),
            "queued" => Some(JobStatus::Queued),
            "running" => Some(JobStatus::Running),
            "done" => Some(JobStatus::Done),
            "failed" => Some(JobStatus::Failed),
            "rejected" => Some(JobStatus::Rejected),
            _ => {
                eprintln!("Unknown status: {}", status_filter);
                return Ok(());
            }
        };

        jobs.iter()
            .filter(|j| Some(j.status) == target_status)
            .collect()
    } else {
        jobs.iter().collect()
    };

    if filtered_jobs.is_empty() {
        println!("No jobs found.");
        return Ok(());
    }

    println!("Jobs ({}):\n", filtered_jobs.len());

    for job in filtered_jobs {
        println!(
            "  #{} [{}] {} {} - {}",
            job.id,
            job.status,
            job.mode,
            job.scope.scope,
            job.target
        );

        if let Some(desc) = &job.description {
            println!("    {}", desc);
        }

        if !job.changed_files.is_empty() {
            println!("    Changed files: {}", job.changed_files.len());
        }

        if let Some(err) = &job.error_message {
            println!("    Error: {}", err);
        }

        println!();
    }

    Ok(())
}

/// Clean up worktrees for all rejected jobs
fn cleanup_rejected_worktrees(manager: &mut JobManager, work_dir: &Path) {
    // Try to create a GitManager - if it fails, we skip cleanup
    let git = match GitManager::new(work_dir) {
        Ok(g) => g,
        Err(_) => return,
    };

    // Collect rejected job IDs that have worktrees
    let rejected_with_worktrees: Vec<_> = manager
        .jobs()
        .iter()
        .filter(|j| j.status == JobStatus::Rejected && j.git_worktree_path.is_some())
        .map(|j| j.id)
        .collect();

    // Clean up each worktree
    for job_id in rejected_with_worktrees {
        if let Err(e) = git.remove_worktree(job_id) {
            eprintln!("Warning: Failed to remove worktree for job #{}: {}", job_id, e);
        } else {
            // Clear the worktree path on the job
            if let Some(job) = manager.get_mut(job_id) {
                job.git_worktree_path = None;
                job.branch_name = None;
            }
            println!("Cleaned up worktree for rejected job #{}", job_id);
        }
    }
}
