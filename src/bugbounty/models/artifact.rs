//! Artifact model - evidence and proof files

use serde::{Deserialize, Serialize};

/// Type of artifact
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    /// HTTP request (curl command or raw request)
    HttpRequest,
    /// HTTP response
    HttpResponse,
    /// Screenshot image
    Screenshot,
    /// Log file or output
    Log,
    /// Proof of concept file
    PocFile,
    /// Video recording
    Video,
    /// Other/generic
    Other,
}

impl ArtifactType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactType::HttpRequest => "http_request",
            ArtifactType::HttpResponse => "http_response",
            ArtifactType::Screenshot => "screenshot",
            ArtifactType::Log => "log",
            ArtifactType::PocFile => "poc_file",
            ArtifactType::Video => "video",
            ArtifactType::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "http_request" | "request" => Some(ArtifactType::HttpRequest),
            "http_response" | "response" => Some(ArtifactType::HttpResponse),
            "screenshot" | "image" | "png" | "jpg" => Some(ArtifactType::Screenshot),
            "log" | "logs" | "output" => Some(ArtifactType::Log),
            "poc_file" | "poc" | "exploit" => Some(ArtifactType::PocFile),
            "video" | "recording" => Some(ArtifactType::Video),
            "other" => Some(ArtifactType::Other),
            _ => None,
        }
    }

    /// Guess artifact type from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" => ArtifactType::Screenshot,
            "mp4" | "webm" | "mov" => ArtifactType::Video,
            "log" | "txt" => ArtifactType::Log,
            "http" | "req" | "request" => ArtifactType::HttpRequest,
            "json" | "xml" => ArtifactType::HttpResponse,
            "py" | "js" | "sh" | "html" | "rs" => ArtifactType::PocFile,
            _ => ArtifactType::Other,
        }
    }
}

/// An artifact (evidence file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Unique ID (UUID)
    pub id: String,
    /// Associated finding (optional)
    pub finding_id: Option<String>,
    /// Associated job (optional)
    pub job_id: Option<String>,
    /// Type of artifact
    pub artifact_type: ArtifactType,
    /// Path relative to project root
    pub path: String,
    /// Description of the artifact
    pub description: Option<String>,
    /// SHA256 hash for deduplication
    pub hash: Option<String>,
    /// Created timestamp (ms since epoch)
    pub created_at: i64,
}

impl Artifact {
    /// Create a new artifact
    pub fn new(path: impl Into<String>, artifact_type: ArtifactType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            finding_id: None,
            job_id: None,
            artifact_type,
            path: path.into(),
            description: None,
            hash: None,
            created_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Create artifact from file path, inferring type from extension
    pub fn from_path(path: impl Into<String>) -> Self {
        let path_str = path.into();
        let ext = std::path::Path::new(&path_str)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let artifact_type = ArtifactType::from_extension(ext);
        Self::new(path_str, artifact_type)
    }

    // Builder methods
    pub fn with_finding(mut self, finding_id: impl Into<String>) -> Self {
        self.finding_id = Some(finding_id.into());
        self
    }

    pub fn with_job(mut self, job_id: impl Into<String>) -> Self {
        self.job_id = Some(job_id.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_hash(mut self, hash: impl Into<String>) -> Self {
        self.hash = Some(hash.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_from_path() {
        let artifact = Artifact::from_path("evidence/screenshot.png");
        assert_eq!(artifact.artifact_type, ArtifactType::Screenshot);

        let artifact = Artifact::from_path("poc/exploit.py");
        assert_eq!(artifact.artifact_type, ArtifactType::PocFile);

        let artifact = Artifact::from_path("logs/output.log");
        assert_eq!(artifact.artifact_type, ArtifactType::Log);
    }

    #[test]
    fn test_artifact_builder() {
        let artifact = Artifact::from_path("request.http")
            .with_finding("VULN-001")
            .with_description("Initial IDOR request");

        assert_eq!(artifact.finding_id, Some("VULN-001".to_string()));
        assert!(artifact.description.is_some());
    }
}
