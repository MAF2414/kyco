//! Selection context - information about the current selection from IDE

use crate::gui::http_server::{Dependency, Diagnostic};
use crate::workspace::WorkspaceId;
use std::path::PathBuf;

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
    /// Dependencies found by IDE (files that reference the selected code)
    pub dependencies: Option<Vec<Dependency>>,
    /// Total count of dependencies
    pub dependency_count: Option<usize>,
    /// Count of additional dependencies not included in the list (when > 30)
    pub additional_dependency_count: Option<usize>,
    /// Related test files found by IDE
    pub related_tests: Option<Vec<String>>,
    /// Diagnostics (errors, warnings) from the IDE for this file
    pub diagnostics: Option<Vec<Diagnostic>>,
    /// Workspace ID this selection belongs to (for multi-workspace support)
    pub workspace_id: Option<WorkspaceId>,
    /// Workspace root path (for multi-workspace support)
    pub workspace_path: Option<PathBuf>,
}

impl SelectionContext {
    /// Check if we have any useful selection data
    pub fn has_selection(&self) -> bool {
        self.selected_text.as_ref().map_or(false, |s| !s.is_empty())
    }

    /// Format IDE context as markdown for prompt injection
    pub fn format_ide_context(&self) -> String {
        let mut ctx = String::new();

        ctx.push_str("## IDE Selection Context\n");

        if let Some(ref path) = self.file_path {
            ctx.push_str(&format!("- **File:** `{}`\n", path));
        }

        if let (Some(start), Some(end)) = (self.line_number, self.line_end) {
            ctx.push_str(&format!("- **Lines:** {}-{}\n", start, end));
        }

        // Dependencies
        if let Some(count) = self.dependency_count {
            if count > 0 {
                ctx.push_str(&format!("\n### Dependencies ({} total", count));
                if let Some(additional) = self.additional_dependency_count {
                    if additional > 0 {
                        ctx.push_str(&format!(", showing {}", count - additional));
                    }
                }
                ctx.push_str("):\n");

                if let Some(ref deps) = self.dependencies {
                    for dep in deps {
                        ctx.push_str(&format!("- `{}:{}`\n", dep.file_path, dep.line));
                    }
                }

                if let Some(additional) = self.additional_dependency_count {
                    if additional > 0 {
                        ctx.push_str(&format!("- *...and {} more*\n", additional));
                    }
                }
            }
        }

        // Related Tests
        if let Some(ref tests) = self.related_tests {
            if !tests.is_empty() {
                ctx.push_str("\n### Related Tests:\n");
                for test in tests {
                    ctx.push_str(&format!("- `{}`\n", test));
                }
            }
        }

        // Diagnostics (Errors/Warnings)
        if let Some(ref diagnostics) = self.diagnostics {
            if !diagnostics.is_empty() {
                let errors: Vec<_> = diagnostics
                    .iter()
                    .filter(|d| d.severity == "Error")
                    .collect();
                let warnings: Vec<_> = diagnostics
                    .iter()
                    .filter(|d| d.severity == "Warning")
                    .collect();

                ctx.push_str("\n### Diagnostics:\n");

                if !errors.is_empty() {
                    ctx.push_str(&format!("**Errors ({}):**\n", errors.len()));
                    for diag in errors {
                        ctx.push_str(&format!(
                            "- Line {}: {}{}\n",
                            diag.line,
                            diag.message,
                            diag.code
                                .as_ref()
                                .map(|c| format!(" [{}]", c))
                                .unwrap_or_default()
                        ));
                    }
                }

                if !warnings.is_empty() {
                    ctx.push_str(&format!("**Warnings ({}):**\n", warnings.len()));
                    for diag in warnings {
                        ctx.push_str(&format!(
                            "- Line {}: {}{}\n",
                            diag.line,
                            diag.message,
                            diag.code
                                .as_ref()
                                .map(|c| format!(" [{}]", c))
                                .unwrap_or_default()
                        ));
                    }
                }
            }
        }

        ctx
    }
}
