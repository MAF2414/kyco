//! Agent type definitions.

use serde::{Deserialize, Serialize};

/// The type of SDK being used
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SdkType {
    /// Claude Agent SDK (Anthropic)
    #[default]
    Claude,
    /// Codex SDK (OpenAI)
    Codex,
    /// Legacy: Gemini (not supported, will use Claude)
    Gemini,
    /// Legacy: Custom (not supported, will use Claude)
    Custom,
}

impl SdkType {
    /// Returns the default agent name for this SDK type.
    pub fn default_name(&self) -> &'static str {
        match self {
            SdkType::Claude => "claude",
            SdkType::Codex => "codex",
            // Legacy: map to Claude
            SdkType::Gemini | SdkType::Custom => "claude",
        }
    }

    /// Returns the default binary for this SDK type.
    pub fn default_binary(&self) -> &'static str {
        self.default_name()
    }

    /// Returns the permission mode options for this SDK type.
    pub fn permission_modes(&self) -> &'static [&'static str] {
        match self {
            SdkType::Claude | SdkType::Gemini | SdkType::Custom => {
                &["default", "acceptEdits", "bypassPermissions", "plan"]
            }
            SdkType::Codex => &["suggest", "auto-edit", "full-auto"],
        }
    }

    /// Returns the default permission mode for this SDK type.
    pub fn default_permission_mode(&self) -> &'static str {
        match self {
            SdkType::Claude | SdkType::Gemini | SdkType::Custom => "default",
            SdkType::Codex => "suggest",
        }
    }
}

// Keep CliType as an alias for backwards compatibility during transition
pub type CliType = SdkType;

/// How to handle the system prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemPromptMode {
    /// Append to the default system prompt
    #[default]
    Append,
    /// Replace the default system prompt entirely
    Replace,
    /// Legacy: Use config override (for CLI-based agents)
    #[serde(rename = "configoverride")]
    ConfigOverride,
}

// SessionMode removed - all agents now use persistent sessions by default.
// This enables automatic retry/resume when connections drop.
