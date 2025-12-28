//! Git utility functions for the executor

use std::path::Path;
use std::process::Command;

/// Calculate lines added/removed using git numstat
pub fn calculate_git_numstat(worktree: &Path, base_branch: Option<&str>) -> (usize, usize) {
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

    fn run_git_numstat(worktree: &Path, args: &[&str]) -> Option<(usize, usize)> {
        let output = Command::new("git")
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
            run_git_numstat(worktree, &["diff", "--numstat", "--no-color", &range])
        {
            total.0 = total.0.saturating_add(added);
            total.1 = total.1.saturating_add(removed);
        }
    }

    // Count uncommitted changes.
    if let Some((added, removed)) = run_git_numstat(worktree, &["diff", "--numstat", "--no-color"])
    {
        total.0 = total.0.saturating_add(added);
        total.1 = total.1.saturating_add(removed);
    }

    total
}
