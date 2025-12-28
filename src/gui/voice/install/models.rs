//! Whisper model definitions and utilities

/// Whisper model information with checksums for validation
#[derive(Debug, Clone)]
pub struct WhisperModel {
    /// Model name (tiny, base, small, medium, large)
    pub name: &'static str,
    /// Expected file size in bytes (approximate, for quick validation)
    pub expected_size: u64,
    /// SHA256 checksum for validation
    pub sha256: &'static str,
    /// Download URL
    pub url: &'static str,
}

/// Available Whisper models with their checksums
/// Checksums from: https://huggingface.co/ggerganov/whisper.cpp
pub const WHISPER_MODELS: &[WhisperModel] = &[
    WhisperModel {
        name: "tiny",
        expected_size: 77_691_713,
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
    },
    WhisperModel {
        name: "base",
        expected_size: 147_951_465,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
    },
    WhisperModel {
        name: "small",
        expected_size: 487_601_967,
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
    },
    WhisperModel {
        name: "medium",
        expected_size: 1_533_774_781,
        sha256: "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
    },
    WhisperModel {
        name: "large",
        expected_size: 3_094_623_691,
        sha256: "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
    },
];

/// Get model info by name
pub fn get_model_info(name: &str) -> Option<&'static WhisperModel> {
    WHISPER_MODELS.iter().find(|m| m.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_model_info() {
        assert!(get_model_info("base").is_some());
        assert!(get_model_info("tiny").is_some());
        assert!(get_model_info("invalid").is_none());
    }

    #[test]
    fn test_model_info_has_valid_urls() {
        for model in WHISPER_MODELS {
            assert!(model.url.starts_with("https://"));
            assert!(model.url.contains("huggingface.co"));
        }
    }
}
