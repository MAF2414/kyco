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
