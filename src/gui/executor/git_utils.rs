//! Git utility functions for the executor

use std::path::Path;
use tokio::process::Command;

fn parse_numstat(output: &str) -> (usize, usize) {
    let mut lines_added = 0usize;
    let mut lines_removed = 0usize;

    for line in output.lines() {
        let mut parts = line.split('\t');
        let Some(added) = parts.next() else { continue };
        let Some(removed) = parts.next() else {
            continue;
        };

        if added != "-" {
            lines_added = lines_added.saturating_add(added.parse::<usize>().unwrap_or(0));
        }
        if removed != "-" {
            lines_removed = lines_removed.saturating_add(removed.parse::<usize>().unwrap_or(0));
        }
    }

    (lines_added, lines_removed)
}

async fn run_git_numstat_async(worktree: &Path, args: &[&str]) -> Option<(usize, usize)> {
    let output = Command::new("git")
        .args(args)
        .current_dir(worktree)
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(parse_numstat(&String::from_utf8_lossy(&output.stdout)))
}

/// Calculate lines added/removed using git numstat (async version)
pub async fn calculate_git_numstat_async(
    worktree: &Path,
    base_branch: Option<&str>,
) -> (usize, usize) {
    let mut total = (0usize, 0usize);

    // Run both git commands concurrently when we have a base branch
    if let Some(base_branch) = base_branch {
        let range = format!("{}...HEAD", base_branch);

        // Store args in variables to extend their lifetime beyond the join!
        let committed_args = ["diff", "--numstat", "--no-color", &range];
        let uncommitted_args = ["diff", "--numstat", "--no-color"];

        // Spawn both git operations concurrently
        let (committed_result, uncommitted_result) = tokio::join!(
            run_git_numstat_async(worktree, &committed_args),
            run_git_numstat_async(worktree, &uncommitted_args)
        );

        if let Some((added, removed)) = committed_result {
            total.0 = total.0.saturating_add(added);
            total.1 = total.1.saturating_add(removed);
        }
        if let Some((added, removed)) = uncommitted_result {
            total.0 = total.0.saturating_add(added);
            total.1 = total.1.saturating_add(removed);
        }
    } else {
        // Only uncommitted changes
        if let Some((added, removed)) =
            run_git_numstat_async(worktree, &["diff", "--numstat", "--no-color"]).await
        {
            total.0 = total.0.saturating_add(added);
            total.1 = total.1.saturating_add(removed);
        }
    }

    total
}

/// Calculate lines added/removed using git numstat (sync version for compatibility)
/// Prefer `calculate_git_numstat_async` in async contexts.
#[allow(dead_code)]
pub fn calculate_git_numstat(worktree: &Path, base_branch: Option<&str>) -> (usize, usize) {
    use std::process::Command as StdCommand;

    fn run_git_numstat_sync(worktree: &Path, args: &[&str]) -> Option<(usize, usize)> {
        let output = StdCommand::new("git")
            .args(args)
            .current_dir(worktree)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        Some(parse_numstat(&String::from_utf8_lossy(&output.stdout)))
    }

    let mut total = (0usize, 0usize);

    // Count committed changes on the worktree branch.
    if let Some(base_branch) = base_branch {
        let range = format!("{}...HEAD", base_branch);
        if let Some((added, removed)) =
            run_git_numstat_sync(worktree, &["diff", "--numstat", "--no-color", &range])
        {
            total.0 = total.0.saturating_add(added);
            total.1 = total.1.saturating_add(removed);
        }
    }

    // Count uncommitted changes.
    if let Some((added, removed)) =
        run_git_numstat_sync(worktree, &["diff", "--numstat", "--no-color"])
    {
        total.0 = total.0.saturating_add(added);
        total.1 = total.1.saturating_add(removed);
    }

    total
}
