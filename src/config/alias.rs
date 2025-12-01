//! Alias configuration types

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Alias configuration for custom shortcuts
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AliasConfig {
    /// Agent aliases: short -> canonical
    #[serde(default)]
    pub agent: HashMap<String, String>,

    /// Mode aliases: short -> canonical
    #[serde(default)]
    pub mode: HashMap<String, String>,

    /// Scope aliases: short -> canonical
    #[serde(default)]
    pub scope: HashMap<String, String>,

    /// Target aliases: short -> canonical
    #[serde(default)]
    pub target: HashMap<String, String>,
}
