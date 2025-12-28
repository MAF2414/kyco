//! Voice Actions - Mapping wakewords to modes and actions
//!
//! This module defines how spoken keywords trigger specific modes or actions.
//! Users can configure custom wakewords that map to different modes.
//!
//! Example configurations:
//! - "refactor" -> triggers refactor mode
//! - "hey kyco fix" -> triggers fix mode
//! - "make tests" -> triggers tests mode

mod registry;
mod voice_action;

pub use registry::VoiceActionRegistry;
pub use voice_action::{VoiceAction, WakewordMatch};

#[cfg(test)]
mod tests;
