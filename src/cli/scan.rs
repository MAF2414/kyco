//! Scan command implementation

use anyhow::Result;
use std::path::Path;

use kyco::config::Config;
use kyco::scanner::Scanner;

/// Scan the repository for KYCo markers and display found tasks
pub async fn scan_command(work_dir: &Path, pending_only: bool) -> Result<()> {
    let config = Config::from_dir(work_dir).unwrap_or_else(|_| Config::with_defaults());
    let scanner = Scanner::with_config(
        work_dir,
        &config.settings.scan_exclude,
        &config.settings.marker_prefix,
    );
    let tags = scanner.scan().await?;

    let filtered_tags: Vec<_> = if pending_only {
        tags.into_iter().filter(|t| !t.is_linked()).collect()
    } else {
        tags
    };

    if filtered_tags.is_empty() {
        println!("No markers found.");
        return Ok(());
    }

    println!("Found {} task(s):\n", filtered_tags.len());

    for tag in &filtered_tags {
        let status = tag
            .status_marker
            .as_ref()
            .map(|m| format!(" [{}]", m))
            .unwrap_or_default();

        let agent_mode = if tag.agent == "claude" {
            tag.mode.clone()
        } else {
            format!("{}:{}", tag.agent, tag.mode)
        };

        println!(
            "  {} {}:{}{}",
            agent_mode,
            tag.file_path.display(),
            tag.line_number,
            status
        );

        if let Some(desc) = &tag.description {
            println!("    {}", desc);
        }
        println!();
    }

    Ok(())
}
