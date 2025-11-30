//! Repository scanner for finding marker comments

use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

use crate::comment::CommentParser;
use crate::CommentTag;

/// Scans a repository for KYCo markers
pub struct Scanner {
    root: PathBuf,
    exclude_patterns: GlobSet,
    /// Marker prefix (e.g., "@" for @docs, "::" for ::docs)
    marker_prefix: String,
}

/// Default patterns to always exclude
const DEFAULT_EXCLUDES: &[&str] = &[
    "kyco.toml",
    ".kyco/**",
];

impl Scanner {
    /// Create a new scanner for the given directory (with default excludes and prefix)
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let default_excludes: Vec<String> = DEFAULT_EXCLUDES.iter().map(|s| s.to_string()).collect();
        Self::with_config(root, &default_excludes, "@@")
    }

    /// Create a new scanner with custom exclude patterns
    /// Note: Default excludes (kyco.toml, .kyco/) are always added
    pub fn with_excludes(root: impl Into<PathBuf>, excludes: &[String]) -> Self {
        Self::with_config(root, excludes, "@@")
    }

    /// Create a new scanner with custom exclude patterns and marker prefix
    pub fn with_config(root: impl Into<PathBuf>, excludes: &[String], marker_prefix: &str) -> Self {
        let mut builder = GlobSetBuilder::new();

        // Always add default excludes
        for pattern in DEFAULT_EXCLUDES {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }

        // Add user-provided excludes
        for pattern in excludes {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }

        Self {
            root: root.into(),
            exclude_patterns: builder.build().unwrap_or_else(|_| GlobSet::empty()),
            marker_prefix: marker_prefix.to_string(),
        }
    }

    /// Scan the repository and return all found comment tags
    pub async fn scan(&self) -> Result<Vec<CommentTag>> {
        let mut tags = Vec::new();
        let parser = CommentParser::with_prefix(&self.marker_prefix);

        let walker = WalkBuilder::new(&self.root)
            .hidden(true)  // Skip hidden files/directories (starting with '.')
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();

        for entry in walker.flatten() {
            let path = entry.path();

            // Skip directories and non-text files
            if !path.is_file() {
                continue;
            }

            // Skip excluded files
            if let Ok(relative) = path.strip_prefix(&self.root) {
                if self.exclude_patterns.is_match(relative) {
                    continue;
                }
                // Also check the file name alone for simple patterns like "kyco.toml"
                if let Some(file_name) = relative.file_name() {
                    if self.exclude_patterns.is_match(file_name) {
                        continue;
                    }
                }
            }

            // Skip files we can't parse (binary, etc.)
            if !Self::is_parseable(path) {
                continue;
            }

            // Read and parse the file
            if let Ok(content) = std::fs::read_to_string(path) {
                let file_tags = parser.parse_file(path, &content);
                tags.extend(file_tags);
            }
        }

        Ok(tags)
    }

    /// Check if a file is parseable for comments
    ///
    /// We scan all text files by default, only excluding known binary formats.
    fn is_parseable(path: &Path) -> bool {
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            // Files without extension - check if they look like text
            // Common cases: Makefile, Dockerfile, LICENSE, etc.
            return true;
        };

        let ext_lower = ext.to_lowercase();

        // Exclude known binary formats
        !matches!(
            ext_lower.as_str(),
            // Images
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "svg" | "webp" | "tiff" | "psd"
            // Audio/Video
            | "mp3" | "mp4" | "wav" | "avi" | "mkv" | "mov" | "flac" | "ogg" | "webm"
            // Archives
            | "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "dmg" | "iso"
            // Binaries
            | "exe" | "dll" | "so" | "dylib" | "bin" | "o" | "a" | "lib"
            // Documents (binary)
            | "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt"
            // Fonts
            | "ttf" | "otf" | "woff" | "woff2" | "eot"
            // Database
            | "db" | "sqlite" | "sqlite3"
            // Lock files (usually auto-generated, large)
            | "lock"
            // Other binary
            | "class" | "pyc" | "pyo" | "wasm" | "rlib"
        )
    }
}
