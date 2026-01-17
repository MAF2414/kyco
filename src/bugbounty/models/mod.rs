//! Data models for BugBounty tracking

mod artifact;
mod finding;
mod flow_edge;
mod job;
mod project;

pub use artifact::{Artifact, ArtifactType};
pub use finding::{Confidence, Finding, FindingStatus, Reachability, Severity};
pub use flow_edge::{CodeLocation, FlowEdge, FlowKind, FlowTrace};
pub use job::BugBountyJob;
pub use project::{Project, ProjectMetadata, ProjectScope, ToolPolicy};
