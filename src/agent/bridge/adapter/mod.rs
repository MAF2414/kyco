//! Bridge-based agent adapters.
//!
//! These adapters use the SDK Bridge for full SDK control instead of CLI invocation.

mod claude;
mod codex;
mod util;

pub use claude::ClaudeBridgeAdapter;
pub use codex::CodexBridgeAdapter;
