//! Data models for BugBounty tracking

mod artifact;
mod finding;
mod flow_edge;
mod job;
mod memory;
mod project;

pub use artifact::{Artifact, ArtifactType};
pub use finding::{Confidence, Finding, FindingStatus, Reachability, Severity};
pub use flow_edge::{CodeLocation, FlowEdge, FlowKind, FlowTrace};
pub use job::BugBountyJob;
pub use memory::{MemoryConfidence, MemoryLocation, MemorySourceKind, MemoryType, ProjectMemory};
pub use project::{Project, ProjectMetadata, ProjectScope, ToolPolicy};
