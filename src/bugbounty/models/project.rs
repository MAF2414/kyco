//! Project model for BugBounty programs

use serde::{Deserialize, Serialize};

/// A BugBounty program/project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique identifier, e.g., "hackerone-nextcloud"
    pub id: String,
    /// Root path relative to workspace, e.g., "BugBounty/programs/hackerone-nextcloud/"
    pub root_path: String,
    /// Platform: hackerone, intigriti, bugcrowd, etc.
    pub platform: Option<String>,
    /// Target name: nextcloud, miro, etc.
    pub target_name: Option<String>,
    /// Parsed scope as JSON
    pub scope: Option<ProjectScope>,
    /// Tool policy restrictions
    pub tool_policy: Option<ToolPolicy>,
    /// Additional metadata (stack, auth notes, etc.)
    pub metadata: Option<ProjectMetadata>,
    /// Created timestamp (ms since epoch)
    pub created_at: i64,
    /// Updated timestamp (ms since epoch)
    pub updated_at: i64,
}

/// Scope definition parsed from scope.md
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectScope {
    /// In-scope domains/assets
    pub in_scope: Vec<String>,
    /// Out-of-scope domains/assets
    pub out_of_scope: Vec<String>,
    /// Rate limits (requests per second)
    pub rate_limit: Option<u32>,
    /// Additional scope notes
    pub notes: Option<String>,
}

/// Tool usage policy
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolPolicy {
    /// Allowed commands (whitelist)
    pub allowed_commands: Vec<String>,
    /// Blocked commands (blacklist)
    pub blocked_commands: Vec<String>,
    /// Required wrapper script for network requests
    pub network_wrapper: Option<String>,
    /// Protected paths that agents should not read
    pub protected_paths: Vec<String>,
}

/// Additional project metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMetadata {
    /// Technology stack
    pub stack: Vec<String>,
    /// Auth requirements/notes
    pub auth_notes: Option<String>,
    /// Important endpoints
    pub endpoints: Vec<String>,
    /// Links (to program page, docs, etc.)
    pub links: Vec<String>,
}

impl Project {
    /// Create a new project with minimal required fields
    pub fn new(id: impl Into<String>, root_path: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: id.into(),
            root_path: root_path.into(),
            platform: None,
            target_name: None,
            scope: None,
            tool_policy: None,
            metadata: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the platform (builder pattern)
    pub fn with_platform(mut self, platform: impl Into<String>) -> Self {
        self.platform = Some(platform.into());
        self
    }

    /// Set the target name (builder pattern)
    pub fn with_target_name(mut self, name: impl Into<String>) -> Self {
        self.target_name = Some(name.into());
        self
    }

    /// Derive platform and target from project ID
    /// e.g., "hackerone-nextcloud" -> platform="hackerone", target="nextcloud"
    pub fn derive_from_id(mut self) -> Self {
        if let Some((platform, target)) = self.id.split_once('-') {
            if self.platform.is_none() {
                self.platform = Some(platform.to_string());
            }
            if self.target_name.is_none() {
                self.target_name = Some(target.to_string());
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_from_id() {
        let project = Project::new("hackerone-nextcloud", "/path/to/project").derive_from_id();
        assert_eq!(project.platform, Some("hackerone".to_string()));
        assert_eq!(project.target_name, Some("nextcloud".to_string()));
    }

    #[test]
    fn test_builder_pattern() {
        let project = Project::new("test-project", "/path")
            .with_platform("intigriti")
            .with_target_name("myapp");

        assert_eq!(project.platform, Some("intigriti".to_string()));
        assert_eq!(project.target_name, Some("myapp".to_string()));
    }
}
