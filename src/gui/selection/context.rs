//! Selection context - information about the current selection from IDE

/// Information about the current selection context (received from IDE extensions)
#[derive(Debug, Clone, Default)]
pub struct SelectionContext {
    /// Name of the focused application (e.g., "Visual Studio Code", "IntelliJ IDEA")
    pub app_name: Option<String>,
    /// Path to the current file (if detectable)
    pub file_path: Option<String>,
    /// The selected text
    pub selected_text: Option<String>,
    /// Start line number (if available)
    pub line_number: Option<usize>,
    /// End line number (if available)
    pub line_end: Option<usize>,
    /// Multiple file matches (when file_path couldn't be determined uniquely)
    pub possible_files: Vec<String>,
}

impl SelectionContext {
    /// Check if we have any useful selection data
    pub fn has_selection(&self) -> bool {
        self.selected_text.as_ref().map_or(false, |s| !s.is_empty())
    }
}
