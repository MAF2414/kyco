//! CLI commands for managing project memory (sources, sinks, dataflow, notes)

use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::bugbounty::{
    import_semgrep_memory, BugBountyManager, MemorySourceKind, MemoryType,
};

/// List memory entries
pub fn list(
    project: Option<String>,
    memory_type: Option<String>,
    source: Option<String>,
    json: bool,
) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    // Require project
    let project_id = project.ok_or_else(|| anyhow::anyhow!("Project ID is required"))?;

    let entries = if let Some(type_str) = memory_type {
        let mem_type = MemoryType::from_str(&type_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid memory type: {}", type_str))?;
        manager.memory().list_by_type(&project_id, mem_type)?
    } else if let Some(source_str) = source {
        let source_kind = MemorySourceKind::from_str(&source_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid source kind: {}", source_str))?;
        manager.memory().list_by_source_kind(&project_id, source_kind)?
    } else {
        manager.memory().list_by_project(&project_id)?
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        if entries.is_empty() {
            println!("No memory entries found for project '{}'.", project_id);
            return Ok(());
        }

        // Print table header
        println!(
            "{:<6} {:<10} {:<10} {:<40} {:<30}",
            "ID", "TYPE", "SOURCE", "TITLE", "LOCATION"
        );
        println!("{}", "-".repeat(96));

        for e in &entries {
            let id = e.id.map(|i| i.to_string()).unwrap_or_else(|| "-".to_string());
            let loc = e.location_string().unwrap_or_else(|| "-".to_string());
            println!(
                "{:<6} {:<10} {:<10} {:<40} {:<30}",
                id,
                e.memory_type.as_str(),
                e.source_kind.as_str(),
                truncate(&e.title, 38),
                truncate(&loc, 28),
            );
        }

        println!("\nTotal: {} entries", entries.len());
    }

    Ok(())
}

/// Import memory from external tools (semgrep, codeql)
pub fn import(tool: &str, file: &str, project: &str) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;
    let path = Path::new(file);

    if !path.exists() {
        bail!("File not found: {}", file);
    }

    // Verify project exists
    let _project = manager
        .get_project(project)?
        .ok_or_else(|| anyhow::anyhow!("Project not found: {}", project))?;

    match tool.to_lowercase().as_str() {
        "semgrep" => {
            let result = import_semgrep_memory(path, project)?;

            // Store memory entries with deduplication
            let mut created = 0;
            let mut skipped = 0;

            for mem in &result.memory {
                if !manager.memory().exists_duplicate(mem)? {
                    manager.memory().create(mem)?;
                    created += 1;
                } else {
                    skipped += 1;
                }
            }

            // Print warnings
            for warning in &result.warnings {
                eprintln!("Warning: {}", warning);
            }

            println!(
                "Imported {} memory entries from Semgrep ({} skipped as duplicates)",
                created, skipped
            );

            // Print summary by type
            let sources = result
                .memory
                .iter()
                .filter(|m| m.memory_type == MemoryType::Source)
                .count();
            let sinks = result
                .memory
                .iter()
                .filter(|m| m.memory_type == MemoryType::Sink)
                .count();
            let dataflows = result
                .memory
                .iter()
                .filter(|m| m.memory_type == MemoryType::Dataflow)
                .count();

            println!("  - {} sources", sources);
            println!("  - {} sinks", sinks);
            println!("  - {} dataflow paths", dataflows);
        }
        "codeql" => {
            // TODO: Implement CodeQL SARIF memory import
            bail!("CodeQL memory import not yet implemented. Use 'kyco import codeql' for findings.");
        }
        _ => {
            bail!(
                "Unknown tool: {}. Supported: semgrep, codeql",
                tool
            );
        }
    }

    Ok(())
}

/// Delete a memory entry by ID
pub fn delete(id: i64) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    // Verify entry exists
    let entry = manager
        .memory()
        .get(id)?
        .ok_or_else(|| anyhow::anyhow!("Memory entry not found: {}", id))?;

    manager.memory().delete(id)?;

    println!(
        "Deleted memory entry {} ({}: {})",
        id,
        entry.memory_type.as_str(),
        entry.title
    );

    Ok(())
}

/// Clear memory entries for a project
pub fn clear(project: &str, source: Option<String>) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    // Verify project exists
    let _project = manager
        .get_project(project)?
        .ok_or_else(|| anyhow::anyhow!("Project not found: {}", project))?;

    let _count = if let Some(source_str) = source {
        let source_kind = MemorySourceKind::from_str(&source_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid source kind: {}", source_str))?;
        let count = manager.memory().clear_by_source_kind(project, source_kind)?;
        println!(
            "Cleared {} {} memory entries from project '{}'",
            count,
            source_kind.as_str(),
            project
        );
        count
    } else {
        let count = manager.memory().clear_all(project)?;
        println!(
            "Cleared {} memory entries from project '{}'",
            count, project
        );
        count
    };

    Ok(())
}

/// Show memory summary for a project
pub fn summary(project: &str) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    // Verify project exists
    let _proj = manager
        .get_project(project)?
        .ok_or_else(|| anyhow::anyhow!("Project not found: {}", project))?;

    let entries = manager.memory().list_by_project(project)?;

    if entries.is_empty() {
        println!("No memory entries for project '{}'.", project);
        return Ok(());
    }

    // Count by type
    let sources = entries
        .iter()
        .filter(|m| m.memory_type == MemoryType::Source)
        .count();
    let sinks = entries
        .iter()
        .filter(|m| m.memory_type == MemoryType::Sink)
        .count();
    let dataflows = entries
        .iter()
        .filter(|m| m.memory_type == MemoryType::Dataflow)
        .count();
    let notes = entries
        .iter()
        .filter(|m| m.memory_type == MemoryType::Note || m.memory_type == MemoryType::Context)
        .count();

    // Count by source
    let agent = entries
        .iter()
        .filter(|m| m.source_kind == MemorySourceKind::Agent)
        .count();
    let semgrep = entries
        .iter()
        .filter(|m| m.source_kind == MemorySourceKind::Semgrep)
        .count();
    let codeql = entries
        .iter()
        .filter(|m| m.source_kind == MemorySourceKind::Codeql)
        .count();
    let manual = entries
        .iter()
        .filter(|m| m.source_kind == MemorySourceKind::Manual)
        .count();

    println!("Project Memory Summary: {}", project);
    println!("{}", "=".repeat(50));
    println!("\nBy Type:");
    println!("  Sources:   {:>5}", sources);
    println!("  Sinks:     {:>5}", sinks);
    println!("  Dataflows: {:>5}", dataflows);
    println!("  Notes:     {:>5}", notes);
    println!("  Total:     {:>5}", entries.len());

    println!("\nBy Source:");
    if agent > 0 {
        println!("  Agent:     {:>5}", agent);
    }
    if semgrep > 0 {
        println!("  Semgrep:   {:>5}", semgrep);
    }
    if codeql > 0 {
        println!("  CodeQL:    {:>5}", codeql);
    }
    if manual > 0 {
        println!("  Manual:    {:>5}", manual);
    }

    Ok(())
}

/// Truncate a string to max length
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
