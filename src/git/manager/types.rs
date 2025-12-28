//! Git types and parsing helpers

use crate::Job;

/// Result of creating a worktree
pub struct WorktreeInfo {
    /// Path to the created worktree
    pub path: std::path::PathBuf,
    /// The base branch from which the worktree was created
    pub base_branch: String,
}

/// Suggested git commit message (subject + optional body).
#[derive(Debug, Clone)]
pub struct CommitMessage {
    pub subject: String,
    pub body: Option<String>,
}

impl CommitMessage {
    pub fn from_job(job: &Job) -> Self {
        let subject = job
            .result
            .as_ref()
            .and_then(|r| {
                r.commit_subject
                    .as_deref()
                    .or(r.title.as_deref())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
            })
            .map(sanitize_commit_subject)
            .unwrap_or_else(|| sanitize_commit_subject(&format!("{}: {}", job.mode, job.target)));

        let body = job
            .result
            .as_ref()
            .and_then(|r| {
                if let Some(body) = r
                    .commit_body
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    return Some(body.to_string());
                }

                let mut paragraphs = Vec::new();
                if let Some(details) = r
                    .details
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    paragraphs.push(details.to_string());
                }
                if let Some(summary) = r
                    .summary
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    paragraphs.push(summary.to_string());
                }

                if paragraphs.is_empty() {
                    None
                } else {
                    Some(paragraphs.join("\n\n"))
                }
            })
            .map(|mut body| {
                // Add lightweight traceability without spamming the subject.
                body.push_str(&format!("\n\nKYCO-Job: #{}", job.id));
                body
            });

        Self { subject, body }
    }
}

fn sanitize_commit_subject(raw: &str) -> String {
    // Keep the subject single-line and reasonably short.
    let first_line = raw.lines().next().unwrap_or("").trim();
    let mut out: String = first_line
        .chars()
        .filter(|c| *c != '\r' && *c != '\n')
        .collect();
    if out.is_empty() {
        out = "kyco: update".to_string();
    }

    const MAX_LEN: usize = 72;
    if out.chars().count() > MAX_LEN {
        out = out.chars().take(MAX_LEN).collect();
    }

    out
}

/// Status of a file in a diff
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed { from: String },
    Copied { from: String },
    Untracked,
}

/// Diff information for a single file
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub status: FileStatus,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub is_binary: bool,
    pub patch: Option<String>,
}

/// Aggregated diff report
#[derive(Debug, Clone)]
pub struct DiffReport {
    pub files: Vec<FileDiff>,
    pub total_added: usize,
    pub total_removed: usize,
    pub files_changed: usize,
}

/// Options for diff generation
#[derive(Debug, Clone, Default)]
pub struct DiffSettings {
    pub ignore_whitespace: bool,
    pub context_lines: u32,
    pub include_untracked: bool,
}

/// Parse NUL-delimited output from git commands
pub(super) fn parse_null_delimited(output: &[u8]) -> Vec<String> {
    output
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .filter_map(|s| std::str::from_utf8(s).ok())
        .map(|s| s.to_string())
        .collect()
}

/// Parse git diff --numstat -z output
/// Returns tuples of (path, lines_added, lines_removed, is_binary)
pub(super) fn parse_numstat_output(output: &[u8]) -> Vec<(String, usize, usize, bool)> {
    let text = String::from_utf8_lossy(output);
    let mut results = Vec::new();

    for line in text.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }

        let (added, removed, is_binary) = if parts[0] == "-" && parts[1] == "-" {
            // Binary file
            (0, 0, true)
        } else {
            let added = parts[0].parse().unwrap_or(0);
            let removed = parts[1].parse().unwrap_or(0);
            (added, removed, false)
        };

        // Handle renames: "old_path\tnew_path" or just "path"
        let path = if parts.len() > 3 {
            // Rename format with NUL: the path after rename info
            parts[2].to_string()
        } else {
            parts[2].to_string()
        };

        // Skip empty paths
        if !path.is_empty() {
            results.push((path, added, removed, is_binary));
        }
    }

    results
}
