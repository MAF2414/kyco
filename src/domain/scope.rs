use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The granularity of a job's scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// A single line
    Line,
    /// A code block (struct, enum, const, etc.)
    Block,
    /// A single function
    #[default]
    Function,
    /// An impl block
    Impl,
    /// An entire file
    File,
    /// A Rust module (mod.rs + submodules)
    Module,
    /// A directory and its contents
    Dir,
    /// The entire project
    Project,
}

impl Scope {
    /// Parse a scope from a string (supports short aliases)
    /// - line: l, ln, line
    /// - block: b, blk, block
    /// - function: f, fn, func, function
    /// - impl: i, impl
    /// - file: fi, file
    /// - module: m, mod, module
    /// - dir: d, dir, directory
    /// - project: p, proj, project, repo
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "l" | "ln" | "line" => Some(Scope::Line),
            "b" | "blk" | "block" => Some(Scope::Block),
            "f" | "fn" | "func" | "function" => Some(Scope::Function),
            "i" | "impl" => Some(Scope::Impl),
            "fi" | "file" => Some(Scope::File),
            "m" | "mod" | "module" => Some(Scope::Module),
            "d" | "dir" | "directory" => Some(Scope::Dir),
            "p" | "proj" | "project" | "repo" => Some(Scope::Project),
            _ => None,
        }
    }

    /// Get the canonical string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Scope::Line => "line",
            Scope::Block => "block",
            Scope::Function => "fn",
            Scope::Impl => "impl",
            Scope::File => "file",
            Scope::Module => "module",
            Scope::Dir => "dir",
            Scope::Project => "project",
        }
    }

    /// Get the short alias
    pub fn short(&self) -> &'static str {
        match self {
            Scope::Line => "l",
            Scope::Block => "b",
            Scope::Function => "f",
            Scope::Impl => "i",
            Scope::File => "fi",
            Scope::Module => "m",
            Scope::Dir => "d",
            Scope::Project => "p",
        }
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Detailed definition of what a scope encompasses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeDefinition {
    /// The scope type
    pub scope: Scope,

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
    pub fn function(
        file_path: PathBuf,
        function_name: String,
        line_range: (usize, usize),
    ) -> Self {
        Self {
            scope: Scope::Function,
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
            scope: Scope::File,
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
            scope: Scope::Dir,
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
            scope: Scope::Project,
            file_path: PathBuf::new(),
            function_name: None,
            byte_range: None,
            line_range: None,
            dir_path: None,
        }
    }
}
