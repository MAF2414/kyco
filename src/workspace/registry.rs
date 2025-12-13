//! Workspace registry for managing multiple repositories
//!
//! Enables KYCO to handle multiple workspaces simultaneously, each with
//! its own configuration, jobs, and git context.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::init::ensure_config_exists;

/// Unique identifier for a workspace
/// Uses a simple incrementing ID for fast comparison
pub type WorkspaceId = u64;

/// Represents a single workspace (repository/project)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique identifier
    pub id: WorkspaceId,

    /// Root path of the workspace
    pub path: PathBuf,

    /// Human-readable name (derived from folder name or git remote)
    pub name: String,

    /// Whether this workspace is currently active in the UI
    #[serde(default)]
    pub is_active: bool,

    /// Last time this workspace was accessed
    #[serde(default)]
    pub last_accessed: Option<u64>,
}

impl Workspace {
    /// Create a new workspace from a path
    pub fn new(id: WorkspaceId, path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("workspace-{}", id));

        Self {
            id,
            path,
            name,
            is_active: false,
            last_accessed: None,
        }
    }

    /// Update the last accessed timestamp
    pub fn touch(&mut self) {
        self.last_accessed = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
    }
}

/// Registry for managing multiple workspaces
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceRegistry {
    /// All registered workspaces, keyed by ID
    workspaces: HashMap<WorkspaceId, Workspace>,

    /// Currently active workspace ID
    active_workspace: Option<WorkspaceId>,

    /// Next workspace ID to assign
    next_id: WorkspaceId,

    /// Path to workspace index by path (for fast lookup)
    #[serde(skip)]
    path_index: HashMap<PathBuf, WorkspaceId>,
}

impl WorkspaceRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            workspaces: HashMap::new(),
            active_workspace: None,
            next_id: 1,
            path_index: HashMap::new(),
        }
    }

    /// Rebuild the path index after deserialization
    pub fn rebuild_index(&mut self) {
        self.path_index.clear();
        for (id, ws) in &self.workspaces {
            self.path_index.insert(ws.path.clone(), *id);
        }
    }

    /// Add a new workspace for a given path
    /// Returns the workspace ID (new or existing)
    ///
    /// When a new workspace is created, this automatically initializes
    /// the configuration if it doesn't exist yet.
    pub fn add_workspace(&mut self, path: PathBuf) -> WorkspaceId {
        // Canonicalize path for consistent lookup
        let canonical_path = path.canonicalize().unwrap_or(path);

        // Check if workspace already exists
        if let Some(&id) = self.path_index.get(&canonical_path) {
            // Touch the existing workspace to update last_accessed
            if let Some(ws) = self.workspaces.get_mut(&id) {
                ws.touch();
            }
            return id;
        }

        // New workspace: ensure config exists (auto-init)
        ensure_config_exists(&canonical_path);

        // Create new workspace
        let id = self.next_id;
        self.next_id += 1;

        let mut workspace = Workspace::new(id, canonical_path.clone());
        workspace.touch();

        self.path_index.insert(canonical_path, id);
        self.workspaces.insert(id, workspace);

        id
    }

    /// Remove a workspace by ID
    pub fn remove_workspace(&mut self, id: WorkspaceId) -> Option<Workspace> {
        if let Some(ws) = self.workspaces.remove(&id) {
            self.path_index.remove(&ws.path);

            // Clear active if this was the active workspace
            if self.active_workspace == Some(id) {
                self.active_workspace = None;
            }

            Some(ws)
        } else {
            None
        }
    }

    /// Get a workspace by ID
    pub fn get(&self, id: WorkspaceId) -> Option<&Workspace> {
        self.workspaces.get(&id)
    }

    /// Get a mutable workspace by ID
    pub fn get_mut(&mut self, id: WorkspaceId) -> Option<&mut Workspace> {
        self.workspaces.get_mut(&id)
    }

    /// Get a workspace by path
    pub fn get_by_path(&self, path: &Path) -> Option<&Workspace> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.path_index
            .get(&canonical)
            .and_then(|id| self.workspaces.get(id))
    }

    /// Get workspace ID by path
    pub fn get_id_by_path(&self, path: &Path) -> Option<WorkspaceId> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.path_index.get(&canonical).copied()
    }

    /// Get or create a workspace for a path
    /// This is the primary entry point for IDE requests
    pub fn get_or_create(&mut self, path: PathBuf) -> WorkspaceId {
        self.add_workspace(path)
    }

    /// List all workspaces
    pub fn list(&self) -> Vec<&Workspace> {
        let mut workspaces: Vec<_> = self.workspaces.values().collect();
        // Sort by last accessed (most recent first)
        workspaces.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        workspaces
    }

    /// Set the active workspace
    pub fn set_active(&mut self, id: WorkspaceId) -> bool {
        if self.workspaces.contains_key(&id) {
            // Deactivate previous
            if let Some(prev_id) = self.active_workspace {
                if let Some(prev) = self.workspaces.get_mut(&prev_id) {
                    prev.is_active = false;
                }
            }

            // Activate new
            if let Some(ws) = self.workspaces.get_mut(&id) {
                ws.is_active = true;
                ws.touch();
            }
            self.active_workspace = Some(id);
            true
        } else {
            false
        }
    }

    /// Get the currently active workspace
    pub fn active(&self) -> Option<&Workspace> {
        self.active_workspace
            .and_then(|id| self.workspaces.get(&id))
    }

    /// Get the currently active workspace ID
    pub fn active_id(&self) -> Option<WorkspaceId> {
        self.active_workspace
    }

    /// Get the number of registered workspaces
    pub fn len(&self) -> usize {
        self.workspaces.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.workspaces.is_empty()
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Persistence
    // ═══════════════════════════════════════════════════════════════════════

    /// Get the default path for the workspace registry file
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".kyco")
            .join("workspaces.json")
    }

    /// Load the workspace registry from a file
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read workspace registry from {}", path.display()))?;

        let mut registry: WorkspaceRegistry = serde_json::from_str(&content)
            .with_context(|| "Failed to parse workspace registry JSON")?;

        // Rebuild the path index (not serialized)
        registry.rebuild_index();

        Ok(registry)
    }

    /// Save the workspace registry to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        let content = serde_json::to_string_pretty(self)
            .with_context(|| "Failed to serialize workspace registry")?;

        fs::write(path, content)
            .with_context(|| format!("Failed to write workspace registry to {}", path.display()))?;

        Ok(())
    }

    /// Load from default path or create new
    pub fn load_or_create() -> Self {
        let path = Self::default_path();
        Self::load(&path).unwrap_or_else(|_| Self::new())
    }

    /// Save to default path
    pub fn save_default(&self) -> Result<()> {
        self.save(&Self::default_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_add_workspace() {
        let mut registry = WorkspaceRegistry::new();
        let path = PathBuf::from("/tmp/test-workspace");

        let id1 = registry.add_workspace(path.clone());
        let id2 = registry.add_workspace(path.clone());

        // Same path should return same ID
        assert_eq!(id1, id2);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_get_or_create() {
        let mut registry = WorkspaceRegistry::new();
        let path1 = PathBuf::from("/tmp/workspace1");
        let path2 = PathBuf::from("/tmp/workspace2");

        let id1 = registry.get_or_create(path1.clone());
        let id2 = registry.get_or_create(path2.clone());
        let id1_again = registry.get_or_create(path1);

        assert_eq!(id1, id1_again);
        assert_ne!(id1, id2);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_active_workspace() {
        let mut registry = WorkspaceRegistry::new();
        let path = PathBuf::from("/tmp/test");

        let id = registry.add_workspace(path);
        assert!(registry.active().is_none());

        registry.set_active(id);
        assert_eq!(registry.active_id(), Some(id));
        assert!(registry.active().is_some());
    }

    #[test]
    fn test_remove_workspace() {
        let mut registry = WorkspaceRegistry::new();
        let path = PathBuf::from("/tmp/test");

        let id = registry.add_workspace(path.clone());
        registry.set_active(id);

        let removed = registry.remove_workspace(id);
        assert!(removed.is_some());
        assert!(registry.active().is_none());
        assert!(registry.get_by_path(&path).is_none());
    }
}
