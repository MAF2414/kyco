//! Helpers for mapping job file paths into a git worktree.

use crate::Job;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WorktreeRemapOutcome {
    pub remapped: bool,
    pub copied_source_file: bool,
}

/// Remap `job.source_file`, `job.scope`, and `job.target` to point into `worktree_root`.
///
/// This is needed because jobs are typically created with file paths in the *original* workspace.
/// When running in an isolated worktree, CLI agents may refuse to access paths outside the
/// current working directory unless explicitly added with `--add-dir`.
///
/// If the mapped source file does not exist in the worktree (e.g. untracked input files),
/// this function tries to copy it into the worktree to preserve isolation.
pub(super) fn remap_job_paths_to_worktree(
    job: &mut Job,
    workspace_root: &Path,
    worktree_root: &Path,
) -> WorktreeRemapOutcome {
    let source_is_prompt_only =
        job.source_file == workspace_root || job.source_file.to_string_lossy() == "prompt";
    if source_is_prompt_only {
        return WorktreeRemapOutcome {
            remapped: false,
            copied_source_file: false,
        };
    }

    let original_source = job.source_file.clone();
    let Ok(rel_source) = original_source.strip_prefix(workspace_root) else {
        return WorktreeRemapOutcome {
            remapped: false,
            copied_source_file: false,
        };
    };

    let mapped_source = worktree_root.join(rel_source);
    let mut copied_source_file = false;

    if !mapped_source.exists() && original_source.is_file() {
        if let Some(parent) = mapped_source.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::copy(&original_source, &mapped_source).is_ok() {
            copied_source_file = true;
        }
    }

    if !mapped_source.exists() {
        return WorktreeRemapOutcome {
            remapped: false,
            copied_source_file: false,
        };
    }

    job.source_file = mapped_source;

    // Remap scope file/dir paths (best effort).
    remap_scope_paths(job, workspace_root, worktree_root);

    // Remap target strings of the form "{path}:{line}" or "{path}:{start}-{end}" (best effort).
    if let Some(new_target) = remap_target_string(&job.target, workspace_root, worktree_root) {
        job.target = new_target;
    }

    WorktreeRemapOutcome {
        remapped: true,
        copied_source_file,
    }
}

fn remap_scope_paths(job: &mut Job, workspace_root: &Path, worktree_root: &Path) {
    if !job.scope.file_path.as_os_str().is_empty() {
        if let Ok(rel) = job.scope.file_path.strip_prefix(workspace_root) {
            job.scope.file_path = worktree_root.join(rel);
        }
    }

    if let Some(dir_path) = job.scope.dir_path.as_mut() {
        if let Ok(rel) = dir_path.strip_prefix(workspace_root) {
            *dir_path = worktree_root.join(rel);
        }
    }
}

fn remap_target_string(
    target: &str,
    workspace_root: &Path,
    worktree_root: &Path,
) -> Option<String> {
    let (path_part, suffix) = target.rsplit_once(':')?;

    // Validate suffix looks like a line number or a line range.
    if suffix.is_empty()
        || !suffix
            .chars()
            .all(|c| c.is_ascii_digit() || c == '-' || c == ' ')
    {
        return None;
    }

    let target_path = PathBuf::from(path_part);
    if !target_path.is_absolute() {
        return None;
    }

    let rel = target_path.strip_prefix(workspace_root).ok()?;
    let new_path = worktree_root.join(rel);
    Some(format!("{}:{}", new_path.display(), suffix.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScopeDefinition;

    #[test]
    fn remaps_paths_into_worktree() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let worktree = temp.path().join("worktree");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(worktree.join("src")).unwrap();

        let original = root.join("src/main.rs");
        let mapped = worktree.join("src/main.rs");
        std::fs::write(&original, "fn main() {}\n").unwrap();
        std::fs::write(&mapped, "fn main() {}\n").unwrap();

        let mut job = crate::Job::new(
            1,
            "refactor".to_string(),
            ScopeDefinition::file(original.clone()),
            format!("{}:42", original.display()),
            None,
            "claude".to_string(),
            original.clone(),
            42,
            None,
        );

        let outcome = remap_job_paths_to_worktree(&mut job, &root, &worktree);

        assert!(outcome.remapped);
        assert!(!outcome.copied_source_file);
        assert_eq!(job.source_file, mapped);
        assert_eq!(job.scope.file_path, mapped);
        assert_eq!(job.target, format!("{}:42", mapped.display()));
    }

    #[test]
    fn copies_untracked_source_file_into_worktree() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let worktree = temp.path().join("worktree");
        std::fs::create_dir_all(root.join("recon/chunks")).unwrap();
        std::fs::create_dir_all(&worktree).unwrap();

        let original = root.join("recon/chunks/file.js");
        let mapped = worktree.join("recon/chunks/file.js");
        std::fs::write(&original, "console.log('hi');\n").unwrap();
        assert!(!mapped.exists());

        let mut job = crate::Job::new(
            1,
            "review".to_string(),
            ScopeDefinition::file(original.clone()),
            format!("{}:10", original.display()),
            None,
            "claude".to_string(),
            original.clone(),
            10,
            None,
        );

        let outcome = remap_job_paths_to_worktree(&mut job, &root, &worktree);

        assert!(outcome.remapped);
        assert!(outcome.copied_source_file);
        assert!(mapped.exists());
        assert_eq!(job.source_file, mapped);
    }

    #[test]
    fn does_not_remap_prompt_only_jobs() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let worktree = temp.path().join("worktree");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&worktree).unwrap();

        let mut job = crate::Job::new(
            1,
            "review".to_string(),
            ScopeDefinition::project(),
            format!("{}:1", root.display()),
            None,
            "claude".to_string(),
            root.clone(),
            1,
            None,
        );

        let outcome = remap_job_paths_to_worktree(&mut job, &root, &worktree);
        assert!(!outcome.remapped);
        assert_eq!(job.source_file, root);
    }
}
