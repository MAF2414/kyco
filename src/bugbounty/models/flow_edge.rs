//! FlowEdge model - for taint/data flow tracking

use serde::{Deserialize, Serialize};

/// Type of flow edge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlowKind {
    /// Taint propagation (user input flows)
    Taint,
    /// Authorization check flow
    Authz,
    /// General data flow
    Dataflow,
    /// Control flow
    Controlflow,
}

impl FlowKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlowKind::Taint => "taint",
            FlowKind::Authz => "authz",
            FlowKind::Dataflow => "dataflow",
            FlowKind::Controlflow => "controlflow",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "taint" => Some(FlowKind::Taint),
            "authz" | "authorization" | "auth" => Some(FlowKind::Authz),
            "dataflow" | "data" => Some(FlowKind::Dataflow),
            "controlflow" | "control" => Some(FlowKind::Controlflow),
            _ => None,
        }
    }
}

/// A location in code (file + line + column + symbol)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeLocation {
    /// File path (relative to project root)
    pub file: String,
    /// Line number (1-based)
    pub line: Option<u32>,
    /// Column number (1-based)
    pub column: Option<u32>,
    /// Symbol name (function, variable, etc.)
    pub symbol: Option<String>,
    /// Code snippet at this location
    pub snippet: Option<String>,
}

impl CodeLocation {
    pub fn new(file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line: None,
            column: None,
            symbol: None,
            snippet: None,
        }
    }

    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_column(mut self, column: u32) -> Self {
        self.column = Some(column);
        self
    }

    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }

    /// Format as "file:line" or "file:line:column" or "file:line:column:symbol"
    pub fn to_string(&self) -> String {
        let mut s = self.file.clone();
        if let Some(line) = self.line {
            s.push_str(&format!(":{}", line));
            if let Some(col) = self.column {
                s.push_str(&format!(":{}", col));
            }
            if let Some(ref symbol) = self.symbol {
                s.push_str(&format!(":{}", symbol));
            }
        }
        s
    }
}

/// A flow edge connecting two code locations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
    /// Database ID (auto-generated)
    pub id: Option<i64>,
    /// Associated finding
    pub finding_id: String,
    /// Source location
    pub from: CodeLocation,
    /// Destination location
    pub to: CodeLocation,
    /// Type of flow
    pub kind: FlowKind,
    /// Additional notes about this edge
    pub notes: Option<String>,
    /// Created timestamp (ms since epoch)
    pub created_at: i64,
}

impl FlowEdge {
    /// Create a new flow edge
    pub fn new(
        finding_id: impl Into<String>,
        from: CodeLocation,
        to: CodeLocation,
        kind: FlowKind,
    ) -> Self {
        Self {
            id: None,
            finding_id: finding_id.into(),
            from,
            to,
            kind,
            notes: None,
            created_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Create a taint flow edge
    pub fn taint(
        finding_id: impl Into<String>,
        from: CodeLocation,
        to: CodeLocation,
    ) -> Self {
        Self::new(finding_id, from, to, FlowKind::Taint)
    }

    /// Create an authz flow edge
    pub fn authz(
        finding_id: impl Into<String>,
        from: CodeLocation,
        to: CodeLocation,
    ) -> Self {
        Self::new(finding_id, from, to, FlowKind::Authz)
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Format as "from -> to"
    pub fn to_string(&self) -> String {
        format!("{} -> {}", self.from.to_string(), self.to.to_string())
    }
}

/// A complete flow trace (chain of edges)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowTrace {
    /// Associated finding
    pub finding_id: String,
    /// Ordered list of edges
    pub edges: Vec<FlowEdge>,
}

impl FlowTrace {
    pub fn new(finding_id: impl Into<String>) -> Self {
        Self {
            finding_id: finding_id.into(),
            edges: Vec::new(),
        }
    }

    pub fn add_edge(&mut self, edge: FlowEdge) {
        self.edges.push(edge);
    }

    /// Get the entry point (first edge's source)
    pub fn entry_point(&self) -> Option<&CodeLocation> {
        self.edges.first().map(|e| &e.from)
    }

    /// Get the sink (last edge's destination)
    pub fn sink(&self) -> Option<&CodeLocation> {
        self.edges.last().map(|e| &e.to)
    }

    /// Format as "entry -> ... -> sink"
    pub fn summary(&self) -> String {
        if self.edges.is_empty() {
            return "(empty trace)".to_string();
        }

        let entry = self.entry_point().map(|l| l.to_string()).unwrap_or_default();
        let sink = self.sink().map(|l| l.to_string()).unwrap_or_default();

        if self.edges.len() == 1 {
            format!("{} -> {}", entry, sink)
        } else {
            format!("{} -> ... ({} hops) -> {}", entry, self.edges.len(), sink)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_location() {
        let loc = CodeLocation::new("src/api/users.rs")
            .with_line(42)
            .with_symbol("get_user");

        assert_eq!(loc.to_string(), "src/api/users.rs:42:get_user");
    }

    #[test]
    fn test_flow_edge() {
        let from = CodeLocation::new("src/routes.rs").with_line(10);
        let to = CodeLocation::new("src/db.rs").with_line(50).with_symbol("query");

        let edge = FlowEdge::taint("VULN-001", from, to);
        assert_eq!(edge.kind, FlowKind::Taint);
        assert!(edge.to_string().contains("->"));
    }

    #[test]
    fn test_flow_trace() {
        let mut trace = FlowTrace::new("VULN-001");

        let loc1 = CodeLocation::new("src/handler.rs").with_line(10);
        let loc2 = CodeLocation::new("src/service.rs").with_line(20);
        let loc3 = CodeLocation::new("src/db.rs").with_line(30);

        trace.add_edge(FlowEdge::taint("VULN-001", loc1.clone(), loc2.clone()));
        trace.add_edge(FlowEdge::taint("VULN-001", loc2, loc3.clone()));

        assert_eq!(trace.entry_point().unwrap().file, "src/handler.rs");
        assert_eq!(trace.sink().unwrap().file, "src/db.rs");
        assert!(trace.summary().contains("2 hops"));
    }
}
