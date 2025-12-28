//! Diff state management

/// State for the diff viewer
#[derive(Default)]
pub struct DiffState {
    /// Diff content to display
    pub content: Option<String>,
    /// File path being diffed (parsed from diff header)
    pub file_path: Option<String>,
    /// Diff scroll offset (reserved for future use)
    #[allow(dead_code)]
    pub scroll: f32,
}

impl DiffState {
    /// Create a new diff state
    pub fn new() -> Self {
        Self {
            content: None,
            file_path: None,
            scroll: 0.0,
        }
    }

    /// Set the diff content to display
    pub fn set_content(&mut self, content: String) {
        self.file_path = extract_file_path(&content);
        self.content = Some(content);
    }

    /// Clear the diff content
    pub fn clear(&mut self) {
        self.content = None;
        self.file_path = None;
        self.scroll = 0.0;
    }

    /// Check if there is content to display
    pub fn has_content(&self) -> bool {
        self.content.is_some()
    }
}

/// Extract file path from diff header (e.g., "diff --git a/foo.rs b/foo.rs")
pub(super) fn extract_file_path(diff: &str) -> Option<String> {
    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            return Some(rest.to_string());
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            return Some(rest.to_string());
        }
    }
    None
}
