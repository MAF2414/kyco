//! Project memory model for tracking sources, sinks, dataflow paths, and context

use serde::{Deserialize, Serialize};

/// Type of memory entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// User input entry point (request.body, argv, query params, etc.)
    Source,
    /// Dangerous operation (SQL execute, shell exec, file write, etc.)
    Sink,
    /// Taint path from source to sink
    Dataflow,
    /// General observation or context note
    Note,
    /// Architecture/design knowledge
    Context,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Source => "source",
            MemoryType::Sink => "sink",
            MemoryType::Dataflow => "dataflow",
            MemoryType::Note => "note",
            MemoryType::Context => "context",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "source" => Some(MemoryType::Source),
            "sink" => Some(MemoryType::Sink),
            "dataflow" => Some(MemoryType::Dataflow),
            "note" => Some(MemoryType::Note),
            "context" => Some(MemoryType::Context),
            _ => None,
        }
    }
}

/// Source of the memory entry (how it was created)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySourceKind {
    /// Manually added by user
    Manual,
    /// Automatically extracted from agent output
    Agent,
    /// Imported from Semgrep analysis
    Semgrep,
    /// Imported from CodeQL analysis
    Codeql,
    /// From TypeScript language server (future)
    Tsserver,
    /// From Rust Analyzer (future)
    RustAnalyzer,
}

impl MemorySourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemorySourceKind::Manual => "manual",
            MemorySourceKind::Agent => "agent",
            MemorySourceKind::Semgrep => "semgrep",
            MemorySourceKind::Codeql => "codeql",
            MemorySourceKind::Tsserver => "tsserver",
            MemorySourceKind::RustAnalyzer => "rust_analyzer",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "manual" => Some(MemorySourceKind::Manual),
            "agent" => Some(MemorySourceKind::Agent),
            "semgrep" => Some(MemorySourceKind::Semgrep),
            "codeql" => Some(MemorySourceKind::Codeql),
            "tsserver" => Some(MemorySourceKind::Tsserver),
            "rust_analyzer" | "rustanalyzer" => Some(MemorySourceKind::RustAnalyzer),
            _ => None,
        }
    }
}

/// Confidence level for memory entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryConfidence {
    High,
    Medium,
    Low,
}

impl MemoryConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryConfidence::High => "high",
            MemoryConfidence::Medium => "medium",
            MemoryConfidence::Low => "low",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "high" => Some(MemoryConfidence::High),
            "medium" | "med" => Some(MemoryConfidence::Medium),
            "low" => Some(MemoryConfidence::Low),
            _ => None,
        }
    }
}

/// Code location for memory entries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryLocation {
    pub file: String,
    pub line: Option<u32>,
}

impl MemoryLocation {
    pub fn new(file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line: None,
        }
    }

    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    pub fn format(&self) -> String {
        match self.line {
            Some(l) => format!("{}:{}", self.file, l),
            None => self.file.clone(),
        }
    }
}

/// A project memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMemory {
    /// Database ID (None for new entries)
    pub id: Option<i64>,
    /// Project this memory belongs to
    pub project_id: String,
    /// Type of memory (source, sink, dataflow, note, context)
    pub memory_type: MemoryType,
    /// How this memory was created
    pub source_kind: MemorySourceKind,
    /// Short title/description
    pub title: String,
    /// Detailed content/explanation
    pub content: Option<String>,

    // Location (for source, sink, note)
    /// File path
    pub file_path: Option<String>,
    /// Start line number
    pub line_start: Option<u32>,
    /// End line number (for ranges)
    pub line_end: Option<u32>,
    /// Symbol name (function, variable, etc.)
    pub symbol: Option<String>,

    // Dataflow edges (for type=dataflow)
    /// Source location for dataflow
    pub from_location: Option<MemoryLocation>,
    /// Sink location for dataflow
    pub to_location: Option<MemoryLocation>,

    // Metadata
    /// Confidence level
    pub confidence: Option<MemoryConfidence>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Job that created this memory entry
    pub source_job_id: Option<String>,

    /// Created timestamp (ms since epoch)
    pub created_at: i64,
    /// Whether this entry is active
    pub is_active: bool,
}

impl ProjectMemory {
    /// Create a new memory entry with minimal required fields
    pub fn new(
        project_id: impl Into<String>,
        memory_type: MemoryType,
        source_kind: MemorySourceKind,
        title: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: None,
            project_id: project_id.into(),
            memory_type,
            source_kind,
            title: title.into(),
            content: None,
            file_path: None,
            line_start: None,
            line_end: None,
            symbol: None,
            from_location: None,
            to_location: None,
            confidence: None,
            tags: Vec::new(),
            source_job_id: None,
            created_at: now,
            is_active: true,
        }
    }

    /// Create a source memory entry
    pub fn source(
        project_id: impl Into<String>,
        source_kind: MemorySourceKind,
        title: impl Into<String>,
    ) -> Self {
        Self::new(project_id, MemoryType::Source, source_kind, title)
    }

    /// Create a sink memory entry
    pub fn sink(
        project_id: impl Into<String>,
        source_kind: MemorySourceKind,
        title: impl Into<String>,
    ) -> Self {
        Self::new(project_id, MemoryType::Sink, source_kind, title)
    }

    /// Create a dataflow memory entry
    pub fn dataflow(
        project_id: impl Into<String>,
        source_kind: MemorySourceKind,
        title: impl Into<String>,
    ) -> Self {
        Self::new(project_id, MemoryType::Dataflow, source_kind, title)
    }

    /// Create a note memory entry
    pub fn note(
        project_id: impl Into<String>,
        source_kind: MemorySourceKind,
        title: impl Into<String>,
    ) -> Self {
        Self::new(project_id, MemoryType::Note, source_kind, title)
    }

    // Builder methods
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn with_file(mut self, file_path: impl Into<String>) -> Self {
        self.file_path = Some(file_path.into());
        self
    }

    pub fn with_line(mut self, line: u32) -> Self {
        self.line_start = Some(line);
        self
    }

    pub fn with_line_range(mut self, start: u32, end: u32) -> Self {
        self.line_start = Some(start);
        self.line_end = Some(end);
        self
    }

    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn with_from_location(mut self, loc: MemoryLocation) -> Self {
        self.from_location = Some(loc);
        self
    }

    pub fn with_to_location(mut self, loc: MemoryLocation) -> Self {
        self.to_location = Some(loc);
        self
    }

    pub fn with_confidence(mut self, confidence: MemoryConfidence) -> Self {
        self.confidence = Some(confidence);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_source_job(mut self, job_id: impl Into<String>) -> Self {
        self.source_job_id = Some(job_id.into());
        self
    }

    /// Get a formatted location string
    pub fn location_string(&self) -> Option<String> {
        self.file_path.as_ref().map(|f| {
            match self.line_start {
                Some(line) => format!("{}:{}", f, line),
                None => f.clone(),
            }
        })
    }

    /// Check if this memory entry is a duplicate of another
    /// Duplicates have same type, file, and line
    pub fn is_duplicate_of(&self, other: &ProjectMemory) -> bool {
        if self.memory_type != other.memory_type {
            return false;
        }

        // For dataflow, check from/to locations
        if self.memory_type == MemoryType::Dataflow {
            let from_match = match (&self.from_location, &other.from_location) {
                (Some(a), Some(b)) => a.file == b.file && a.line == b.line,
                (None, None) => true,
                _ => false,
            };
            let to_match = match (&self.to_location, &other.to_location) {
                (Some(a), Some(b)) => a.file == b.file && a.line == b.line,
                (None, None) => true,
                _ => false,
            };
            return from_match && to_match;
        }

        // For source/sink/note, check file + line
        match (&self.file_path, &other.file_path) {
            (Some(a), Some(b)) if a == b => {
                self.line_start == other.line_start
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_type_conversion() {
        assert_eq!(MemoryType::from_str("source"), Some(MemoryType::Source));
        assert_eq!(MemoryType::from_str("SINK"), Some(MemoryType::Sink));
        assert_eq!(MemoryType::from_str("invalid"), None);
        assert_eq!(MemoryType::Source.as_str(), "source");
    }

    #[test]
    fn test_memory_source_kind_conversion() {
        assert_eq!(MemorySourceKind::from_str("agent"), Some(MemorySourceKind::Agent));
        assert_eq!(MemorySourceKind::from_str("SEMGREP"), Some(MemorySourceKind::Semgrep));
        assert_eq!(MemorySourceKind::Agent.as_str(), "agent");
    }

    #[test]
    fn test_memory_builder() {
        let mem = ProjectMemory::source("test-project", MemorySourceKind::Agent, "Request body input")
            .with_file("src/api/handler.rs")
            .with_line(42)
            .with_symbol("login_handler")
            .with_confidence(MemoryConfidence::High)
            .with_tag("http")
            .with_tag("auth");

        assert_eq!(mem.memory_type, MemoryType::Source);
        assert_eq!(mem.file_path, Some("src/api/handler.rs".to_string()));
        assert_eq!(mem.line_start, Some(42));
        assert_eq!(mem.tags, vec!["http", "auth"]);
    }

    #[test]
    fn test_duplicate_detection() {
        let mem1 = ProjectMemory::source("test", MemorySourceKind::Agent, "Input 1")
            .with_file("src/api.rs")
            .with_line(42);

        let mem2 = ProjectMemory::source("test", MemorySourceKind::Semgrep, "Input 2")
            .with_file("src/api.rs")
            .with_line(42);

        let mem3 = ProjectMemory::source("test", MemorySourceKind::Agent, "Input 3")
            .with_file("src/api.rs")
            .with_line(43);

        assert!(mem1.is_duplicate_of(&mem2));
        assert!(!mem1.is_duplicate_of(&mem3));
    }

    #[test]
    fn test_dataflow_duplicate_detection() {
        let mem1 = ProjectMemory::dataflow("test", MemorySourceKind::Agent, "Flow 1")
            .with_from_location(MemoryLocation::new("src/a.rs").with_line(10))
            .with_to_location(MemoryLocation::new("src/b.rs").with_line(20));

        let mem2 = ProjectMemory::dataflow("test", MemorySourceKind::Agent, "Flow 2")
            .with_from_location(MemoryLocation::new("src/a.rs").with_line(10))
            .with_to_location(MemoryLocation::new("src/b.rs").with_line(20));

        let mem3 = ProjectMemory::dataflow("test", MemorySourceKind::Agent, "Flow 3")
            .with_from_location(MemoryLocation::new("src/a.rs").with_line(10))
            .with_to_location(MemoryLocation::new("src/b.rs").with_line(30));

        assert!(mem1.is_duplicate_of(&mem2));
        assert!(!mem1.is_duplicate_of(&mem3));
    }
}
