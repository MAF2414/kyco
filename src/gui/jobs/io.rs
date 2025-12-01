//! Job file I/O operations
//!
//! This module handles reading and writing job files to disk.

use super::super::selection::SelectionContext;
use std::path::PathBuf;

/// Write a job request file to the gui_jobs directory
#[allow(dead_code)]
pub fn write_job_request(
    work_dir: &PathBuf,
    selection: &SelectionContext,
    agent: &str,
    mode: &str,
    prompt: &str,
) -> std::io::Result<()> {
    use std::fs;
    use std::io::Write;

    let kyco_dir = work_dir.join(".kyco");
    let jobs_dir = kyco_dir.join("gui_jobs");
    fs::create_dir_all(&jobs_dir)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let job_file = jobs_dir.join(format!("job_{}.json", timestamp));

    let (line_start, line_end, line_count) = if let Some(text) = &selection.selected_text {
        let lines: Vec<&str> = text.lines().collect();
        let count = lines.len();
        let start = selection.line_number.unwrap_or(1);
        let end = start + count.saturating_sub(1);
        (Some(start), Some(end), Some(count))
    } else {
        (None, None, None)
    };

    let job = serde_json::json!({
        "agent": agent,
        "mode": mode,
        "prompt": if prompt.is_empty() { None } else { Some(prompt) },
        "source_app": selection.app_name,
        "source_file": selection.file_path,
        "selection": {
            "text": selection.selected_text,
            "line_start": line_start,
            "line_end": line_end,
            "line_count": line_count,
        },
        "created_at": timestamp,
    });

    let content = serde_json::to_string_pretty(&job)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut file = fs::File::create(&job_file)?;
    file.write_all(content.as_bytes())?;

    Ok(())
}
