//! File search implementation using glob patterns and content grep

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use super::state::FileMatch;

/// Maximum number of results to return
const MAX_RESULTS: usize = 500;

/// Maximum file size to search content (5MB)
const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024;

/// Gitignore pattern matcher
struct GitignorePatterns {
    patterns: Vec<(String, bool)>, // (pattern, is_negation)
}

impl GitignorePatterns {
    fn load(work_dir: &Path) -> Self {
        let mut patterns = Vec::new();

        // Try to load .gitignore from work_dir
        let gitignore_path = work_dir.join(".gitignore");
        if let Ok(content) = fs::read_to_string(&gitignore_path) {
            for line in content.lines() {
                let line = line.trim();
                // Skip empty lines and comments
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                let (pattern, is_negation) = if let Some(stripped) = line.strip_prefix('!') {
                    (stripped.to_string(), true)
                } else {
                    (line.to_string(), false)
                };

                patterns.push((pattern, is_negation));
            }
        }

        Self { patterns }
    }

    fn is_ignored(&self, relative_path: &str) -> bool {
        if self.patterns.is_empty() {
            return false;
        }

        let mut ignored = false;

        for (pattern, is_negation) in &self.patterns {
            if self.matches_pattern(relative_path, pattern) {
                ignored = !is_negation;
            }
        }

        ignored
    }

    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        // Handle directory patterns (ending with /)
        let pattern = pattern.trim_end_matches('/');

        // Simple glob matching
        if pattern.contains('*') {
            // Convert gitignore pattern to glob pattern
            let glob_pattern = if pattern.starts_with('/') {
                pattern[1..].to_string()
            } else if pattern.contains('/') {
                pattern.to_string()
            } else {
                // Match in any directory
                format!("**/{}", pattern)
            };

            if let Ok(glob) = glob::Pattern::new(&glob_pattern) {
                return glob.matches(path);
            }
        } else {
            // Literal match - check if path contains this segment
            let pattern_lower = pattern.to_lowercase();
            let path_lower = path.to_lowercase();

            // Check if it's a directory/file name match
            if pattern.starts_with('/') {
                // Anchored to root
                return path_lower.starts_with(&pattern_lower[1..]);
            } else if pattern.contains('/') {
                // Contains path separator - match as path
                return path_lower.contains(&pattern_lower);
            } else {
                // Match any path component
                return path_lower.split('/').any(|segment| segment == pattern_lower);
            }
        }

        false
    }
}

/// Search for files using a glob pattern
///
/// Supports patterns like:
/// - `*.rs` - all .rs files in current directory
/// - `**/*.rs` - all .rs files recursively
/// - `src/**/*.ts` - all .ts files under src/
pub fn search_glob(pattern: &str, work_dir: &Path, respect_gitignore: bool) -> Vec<FileMatch> {
    let mut results = Vec::new();

    // Load gitignore patterns if enabled
    let gitignore = if respect_gitignore {
        Some(GitignorePatterns::load(work_dir))
    } else {
        None
    };

    // Construct full pattern path
    let full_pattern = if pattern.starts_with('/') || pattern.starts_with("./") {
        pattern.to_string()
    } else {
        format!("{}/{}", work_dir.display(), pattern)
    };

    // Use glob to find matching files
    match glob::glob(&full_pattern) {
        Ok(paths) => {
            for entry in paths {
                if results.len() >= MAX_RESULTS {
                    break;
                }

                if let Ok(path) = entry {
                    // Skip directories
                    if path.is_dir() {
                        continue;
                    }

                    // Calculate relative path
                    let relative_path = path
                        .strip_prefix(work_dir)
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| path.display().to_string());

                    // Check gitignore
                    if let Some(ref gi) = gitignore {
                        if gi.is_ignored(&relative_path) {
                            continue;
                        }
                    }

                    results.push(FileMatch {
                        path,
                        relative_path,
                        match_line: None,
                        match_preview: None,
                    });
                }
            }
        }
        Err(e) => {
            tracing::warn!("Glob pattern error: {}", e);
        }
    }

    results
}

/// Search for files containing a pattern (grep-style)
///
/// Returns files with matching lines
pub fn search_grep(pattern: &str, work_dir: &Path, respect_gitignore: bool) -> Vec<FileMatch> {
    let mut results = Vec::new();
    let pattern_lower = pattern.to_lowercase();

    // Load gitignore patterns if enabled
    let gitignore = if respect_gitignore {
        Some(GitignorePatterns::load(work_dir))
    } else {
        None
    };

    // Walk directory recursively
    if let Err(e) = walk_and_search(work_dir, work_dir, &pattern_lower, &mut results, gitignore.as_ref()) {
        tracing::warn!("Error walking directory: {}", e);
    }

    results
}

fn walk_and_search(
    dir: &Path,
    work_dir: &Path,
    pattern: &str,
    results: &mut Vec<FileMatch>,
    gitignore: Option<&GitignorePatterns>,
) -> std::io::Result<()> {
    if results.len() >= MAX_RESULTS {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden files/directories
        if path
            .file_name()
            .map(|n| n.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }

        // Skip common non-text directories
        let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if matches!(
            dir_name,
            "node_modules" | "target" | "dist" | "build" | ".git" | "__pycache__" | "vendor"
        ) {
            continue;
        }

        // Check gitignore for directories too
        if let Some(gi) = gitignore {
            let relative_path = path
                .strip_prefix(work_dir)
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            if gi.is_ignored(&relative_path) {
                continue;
            }
        }

        if path.is_dir() {
            walk_and_search(&path, work_dir, pattern, results, gitignore)?;
        } else if path.is_file() {
            // Check file size
            if let Ok(metadata) = path.metadata() {
                if metadata.len() > MAX_FILE_SIZE {
                    continue;
                }
            }

            // Search file content
            if let Some(file_match) = search_file_content(&path, work_dir, pattern) {
                results.push(file_match);
                if results.len() >= MAX_RESULTS {
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

fn search_file_content(path: &Path, work_dir: &Path, pattern: &str) -> Option<FileMatch> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.ok()?;
        if line.to_lowercase().contains(pattern) {
            let relative_path = path
                .strip_prefix(work_dir)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| path.display().to_string());

            // Truncate preview if too long
            let preview = if line.len() > 100 {
                format!("{}...", &line[..100])
            } else {
                line
            };

            return Some(FileMatch {
                path: path.to_path_buf(),
                relative_path,
                match_line: Some(line_num + 1), // 1-indexed
                match_preview: Some(preview),
            });
        }
    }

    None
}

/// Perform search based on mode
pub fn perform_search(
    pattern: &str,
    mode: super::state::SearchMode,
    work_dir: PathBuf,
    respect_gitignore: bool,
) -> Vec<FileMatch> {
    match mode {
        super::state::SearchMode::Glob => search_glob(pattern, &work_dir, respect_gitignore),
        super::state::SearchMode::Grep => search_grep(pattern, &work_dir, respect_gitignore),
    }
}
