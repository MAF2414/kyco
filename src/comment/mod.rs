//! Comment parsing for KYCo markers
//!
//! Supports both new syntax (@agent#mode.target.scope) and legacy (// cr: mode scope)

mod parser;

pub use parser::{AliasResolver, CommentParser, ModeDefaults};
