//! File search state management

use std::collections::HashSet;
use std::path::PathBuf;

/// Search mode for file discovery
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchMode {
    #[default]
    Glob, // Pattern matching on filenames (e.g., **/*.rs)
    Grep, // Content search (regex)
}

impl SearchMode {
    pub fn label(&self) -> &'static str {
        match self {
            SearchMode::Glob => "Glob",
            SearchMode::Grep => "Grep",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            SearchMode::Glob => "**/*.rs, src/**/*.ts",
            SearchMode::Grep => "fn main, TODO:",
        }
    }
}

/// A file match from search results
#[derive(Debug, Clone)]
pub struct FileMatch {
    /// Absolute path to the file
    pub path: PathBuf,
    /// Path relative to workspace root
    pub relative_path: String,
    /// For grep: matching line number (1-indexed)
    pub match_line: Option<usize>,
    /// For grep: preview of matching line content
    pub match_preview: Option<String>,
}

/// Selected file with optional line range
#[derive(Debug, Clone)]
pub struct FileSelection {
    /// Index in search_results
    pub file_index: usize,
    /// Selected line range (start, end) - 1-indexed, inclusive
    pub line_range: Option<(usize, usize)>,
}

/// State for file search and selection UI
#[derive(Debug)]
pub struct FileSearchState {
    /// Current search query
    pub search_query: String,
    /// Current search mode (Glob or Grep)
    pub search_mode: SearchMode,
    /// Whether to respect .gitignore files (default: true)
    pub respect_gitignore: bool,
    /// Search results
    pub search_results: Vec<FileMatch>,
    /// Indices of selected files in search_results
    pub selected_files: HashSet<usize>,
    /// Currently previewed file index
    pub preview_file: Option<usize>,
    /// Cached content of previewed file
    pub preview_content: Option<String>,
    /// Selected lines in preview (1-indexed)
    pub preview_selected_lines: HashSet<usize>,
    /// Whether a search is currently running
    pub is_searching: bool,
    /// Last search error
    pub search_error: Option<String>,
}

impl Default for FileSearchState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            search_mode: SearchMode::default(),
            respect_gitignore: true, // Default ON
            search_results: Vec::new(),
            selected_files: HashSet::new(),
            preview_file: None,
            preview_content: None,
            preview_selected_lines: HashSet::new(),
            is_searching: false,
            search_error: None,
        }
    }
}

impl FileSearchState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear search results and selections
    pub fn clear(&mut self) {
        self.search_results.clear();
        self.selected_files.clear();
        self.preview_file = None;
        self.preview_content = None;
        self.preview_selected_lines.clear();
        self.search_error = None;
    }

    /// Toggle selection of a file
    pub fn toggle_file(&mut self, index: usize) {
        if self.selected_files.contains(&index) {
            self.selected_files.remove(&index);
        } else {
            self.selected_files.insert(index);
        }
    }

    /// Select all files
    pub fn select_all(&mut self) {
        for i in 0..self.search_results.len() {
            self.selected_files.insert(i);
        }
    }

    /// Deselect all files
    pub fn deselect_all(&mut self) {
        self.selected_files.clear();
    }

    /// Toggle a line in preview
    pub fn toggle_preview_line(&mut self, line: usize) {
        if self.preview_selected_lines.contains(&line) {
            self.preview_selected_lines.remove(&line);
        } else {
            self.preview_selected_lines.insert(line);
        }
    }

    /// Select a range of lines in preview
    pub fn select_line_range(&mut self, start: usize, end: usize) {
        for line in start..=end {
            self.preview_selected_lines.insert(line);
        }
    }

    /// Get selected files with their line ranges
    pub fn get_selections(&self) -> Vec<FileSelection> {
        self.selected_files
            .iter()
            .map(|&idx| {
                let line_range = if self.preview_file == Some(idx) && !self.preview_selected_lines.is_empty() {
                    let min = *self.preview_selected_lines.iter().min().unwrap();
                    let max = *self.preview_selected_lines.iter().max().unwrap();
                    Some((min, max))
                } else {
                    None
                };
                FileSelection {
                    file_index: idx,
                    line_range,
                }
            })
            .collect()
    }

    /// Number of selected files
    pub fn selected_count(&self) -> usize {
        self.selected_files.len()
    }
}
