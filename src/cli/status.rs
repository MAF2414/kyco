//! Status command implementation

use anyhow::Result;
use std::path::Path;

use kyco::job::JobManager;
use kyco::JobStatus;

/// Show the status of all jobs
pub async fn status_command(work_dir: &Path, filter: Option<String>) -> Result<()> {
    let manager = JobManager::load(work_dir)?;
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
