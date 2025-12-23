use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Detailed definition of what a scope encompasses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeDefinition {
    /// The file path this scope relates to
    pub file_path: PathBuf,

    /// For Function scope: the name of the function
    pub function_name: Option<String>,

    /// For Function scope: the byte range in the file (start, end)
    pub byte_range: Option<(usize, usize)>,

    /// For Function scope: the line range in the file (start, end, 1-indexed)
    pub line_range: Option<(usize, usize)>,

    /// For Dir scope: the directory path
    pub dir_path: Option<PathBuf>,
}

impl ScopeDefinition {
    /// Create a function scope definition
    pub fn function(file_path: PathBuf, function_name: String, line_range: (usize, usize)) -> Self {
        Self {
            file_path,
            function_name: Some(function_name),
            byte_range: None,
            line_range: Some(line_range),
            dir_path: None,
        }
    }

    /// Create a file scope definition
    pub fn file(file_path: PathBuf) -> Self {
        Self {
            file_path,
            function_name: None,
            byte_range: None,
            line_range: None,
            dir_path: None,
        }
    }

    /// Create a directory scope definition
    pub fn dir(dir_path: PathBuf) -> Self {
        Self {
            file_path: PathBuf::new(),
            function_name: None,
            byte_range: None,
            line_range: None,
            dir_path: Some(dir_path),
        }
    }

    /// Create a project scope definition
    pub fn project() -> Self {
        Self {
            file_path: PathBuf::new(),
            function_name: None,
            byte_range: None,
            line_range: None,
            dir_path: None,
        }
    }
}
