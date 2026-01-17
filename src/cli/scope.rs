//! CLI commands for scope and tool policy management

use anyhow::{bail, Context, Result};
use crate::bugbounty::BugBountyManager;

/// Show scope for a project
pub fn show(project: Option<String>, json: bool) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let project_id = resolve_project_id(&manager, project)?;
    let project = manager
        .get_project(&project_id)?
        .ok_or_else(|| anyhow::anyhow!("Project not found: {}", project_id))?;

    if json {
        let output = serde_json::json!({
            "project_id": project.id,
            "scope": project.scope,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("Project: {}", project.id);
    println!("Root: {}", project.root_path);
    println!();

    match &project.scope {
        Some(scope) => {
            if !scope.in_scope.is_empty() {
                println!("In Scope:");
                for asset in &scope.in_scope {
                    println!("  ✓ {}", asset);
                }
                println!();
            }

            if !scope.out_of_scope.is_empty() {
                println!("Out of Scope:");
                for asset in &scope.out_of_scope {
                    println!("  ✗ {}", asset);
                }
                println!();
            }

            if let Some(rate_limit) = scope.rate_limit {
                println!("Rate Limit: {} req/s", rate_limit);
            }

            if let Some(ref notes) = scope.notes {
                println!("\nNotes:");
                for line in notes.lines().take(10) {
                    println!("  {}", line);
                }
                if notes.lines().count() > 10 {
                    println!("  ...(truncated)");
                }
            }

            if scope.in_scope.is_empty() && scope.out_of_scope.is_empty() {
                println!("No scope information parsed.");
                println!("Tip: Add a scope.md file to the project root with 'In Scope' and 'Out of Scope' sections.");
            }
        }
        None => {
            println!("No scope defined for this project.");
            println!("Tip: Add a scope.md file to the project root.");
        }
    }

    Ok(())
}

/// Check if a URL/asset is in scope
pub fn check(url: &str, project: Option<String>) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let project_id = resolve_project_id(&manager, project)?;
    let project = manager
        .get_project(&project_id)?
        .ok_or_else(|| anyhow::anyhow!("Project not found: {}", project_id))?;

    let scope = match &project.scope {
        Some(s) => s,
        None => {
            println!("⚠ No scope defined for project '{}'", project_id);
            println!("Cannot determine if '{}' is in scope.", url);
            return Ok(());
        }
    };

    // Extract domain/host from URL for matching
    let check_target = normalize_url_for_matching(url);

    // Check out-of-scope first (takes precedence)
    for oos in &scope.out_of_scope {
        if matches_scope_pattern(&check_target, oos) {
            println!("✗ OUT OF SCOPE: {}", url);
            println!("  Matches OOS pattern: {}", oos);
            return Ok(());
        }
    }

    // Check in-scope
    for is in &scope.in_scope {
        if matches_scope_pattern(&check_target, is) {
            println!("✓ IN SCOPE: {}", url);
            println!("  Matches pattern: {}", is);
            if let Some(rate_limit) = scope.rate_limit {
                println!("  Rate limit: {} req/s", rate_limit);
            }
            return Ok(());
        }
    }

    // No match found
    println!("? UNKNOWN: {}", url);
    println!("  Does not match any in-scope or out-of-scope patterns.");
    println!("\n  In-scope patterns:");
    for is in &scope.in_scope {
        println!("    - {}", is);
    }

    Ok(())
}

/// Show tool policy for a project
pub fn policy(project: Option<String>, json: bool) -> Result<()> {
    let manager = BugBountyManager::new().context("Failed to initialize BugBounty database")?;

    let project_id = resolve_project_id(&manager, project)?;
    let project = manager
        .get_project(&project_id)?
        .ok_or_else(|| anyhow::anyhow!("Project not found: {}", project_id))?;

    if json {
        let output = serde_json::json!({
            "project_id": project.id,
            "tool_policy": project.tool_policy,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("Project: {}", project.id);
    println!();

    match &project.tool_policy {
        Some(policy) => {
            if !policy.blocked_commands.is_empty() {
                println!("Blocked Commands:");
                for cmd in &policy.blocked_commands {
                    println!("  ✗ {}", cmd);
                }
                println!();
            }

            if !policy.allowed_commands.is_empty() {
                println!("Allowed Commands (whitelist):");
                for cmd in &policy.allowed_commands {
                    println!("  ✓ {}", cmd);
                }
                println!();
            }

            if let Some(ref wrapper) = policy.network_wrapper {
                println!("Network Wrapper: {}", wrapper);
                println!("  All network requests must go through this wrapper.");
                println!();
            }

            if !policy.protected_paths.is_empty() {
                println!("Protected Paths (agent cannot read):");
                for path in &policy.protected_paths {
                    println!("  🔒 {}", path);
                }
                println!();
            }

            if policy.blocked_commands.is_empty()
                && policy.allowed_commands.is_empty()
                && policy.network_wrapper.is_none()
                && policy.protected_paths.is_empty()
            {
                println!("No tool restrictions defined.");
            }
        }
        None => {
            println!("No tool policy defined for this project.");
            println!("Tip: Define tool_policy in project metadata or scope.md.");
        }
    }

    Ok(())
}

/// Resolve project ID from argument or active project
fn resolve_project_id(manager: &BugBountyManager, project: Option<String>) -> Result<String> {
    if let Some(id) = project {
        return Ok(id);
    }

    // Try to get active project from ~/.kyco/active_project
    if let Some(home) = dirs::home_dir() {
        let path = home.join(".kyco").join("active_project");
        if path.exists() {
            let id = std::fs::read_to_string(&path)?.trim().to_string();
            if !id.is_empty() {
                return Ok(id);
            }
        }
    }

    bail!("No project specified and no active project selected.\nUse --project <id> or run: kyco project select <id>")
}

/// Normalize URL for scope matching (extract domain/host)
fn normalize_url_for_matching(url: &str) -> String {
    let url = url.trim();

    // If it's a full URL, extract the host manually
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Extract host (before first / or :)
    let host = without_scheme
        .split('/')
        .next()
        .unwrap_or(without_scheme)
        .split(':')
        .next()
        .unwrap_or(without_scheme);

    host.to_lowercase()
}

/// Check if target matches a scope pattern (supports wildcards)
fn matches_scope_pattern(target: &str, pattern: &str) -> bool {
    let pattern = pattern.trim().to_lowercase();
    let target = target.to_lowercase();

    // Handle wildcard patterns like *.example.com
    if let Some(suffix) = pattern.strip_prefix("*.") {
        // Match the domain or any subdomain
        return target == suffix || target.ends_with(&format!(".{}", suffix));
    }

    // Handle URL patterns (strip protocol for matching)
    let pattern_clean = pattern
        .strip_prefix("https://")
        .or_else(|| pattern.strip_prefix("http://"))
        .unwrap_or(&pattern);

    let target_clean = target
        .strip_prefix("https://")
        .or_else(|| target.strip_prefix("http://"))
        .unwrap_or(&target);

    // Exact match or prefix match for paths
    target_clean == pattern_clean || target_clean.starts_with(&format!("{}/", pattern_clean))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_scope_pattern() {
        // Wildcard patterns
        assert!(matches_scope_pattern("api.example.com", "*.example.com"));
        assert!(matches_scope_pattern("sub.api.example.com", "*.example.com"));
        assert!(matches_scope_pattern("example.com", "*.example.com"));
        assert!(!matches_scope_pattern("example.org", "*.example.com"));

        // Exact patterns
        assert!(matches_scope_pattern("api.example.com", "api.example.com"));
        assert!(!matches_scope_pattern("other.example.com", "api.example.com"));

        // URL patterns
        assert!(matches_scope_pattern("https://api.example.com", "api.example.com"));
        assert!(matches_scope_pattern("api.example.com", "https://api.example.com"));
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(
            normalize_url_for_matching("https://API.Example.com/path"),
            "api.example.com"
        );
        assert_eq!(
            normalize_url_for_matching("api.example.com"),
            "api.example.com"
        );
    }
}
