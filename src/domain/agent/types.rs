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

/// Agent execution mode - determines if conversation is continued or one-shot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionMode {
    /// One-shot execution - no session persistence
    #[default]
    Oneshot,
    /// Session mode - conversation can be resumed/continued
    Session,
    /// Legacy: Print mode (equivalent to Oneshot)
    Print,
    /// Legacy: REPL mode (equivalent to Session)
    Repl,
}

impl SessionMode {
    /// Returns true if this is a session-based mode (Session or Repl)
    pub fn is_session(&self) -> bool {
        matches!(self, SessionMode::Session | SessionMode::Repl)
    }

    /// Returns true if this is a one-shot mode (Oneshot or Print)
    pub fn is_oneshot(&self) -> bool {
        matches!(self, SessionMode::Oneshot | SessionMode::Print)
    }
}

// Keep AgentMode as alias for backwards compatibility
pub type AgentMode = SessionMode;
