//! State management for chain editing UI

use std::path::Path;

use crate::config::{ChainStep, Config};
use crate::gui::app::ViewMode;

/// State for chain editing UI
pub struct ChainEditorState<'a> {
    pub selected_chain: &'a mut Option<String>,
    pub chain_edit_name: &'a mut String,
    pub chain_edit_description: &'a mut String,
    pub chain_edit_steps: &'a mut Vec<ChainStepEdit>,
    pub chain_edit_stop_on_failure: &'a mut bool,
    pub chain_edit_status: &'a mut Option<(String, bool)>,
    pub view_mode: &'a mut ViewMode,
    pub config: &'a mut Config,
    pub work_dir: &'a Path,
}

/// Editable chain step
#[derive(Clone, Default)]
pub struct ChainStepEdit {
    pub mode: String,
    pub trigger_on: String,  // Comma-separated
    pub skip_on: String,     // Comma-separated
    pub agent: String,       // Optional override
    pub inject_context: String,
}

impl From<&ChainStep> for ChainStepEdit {
    fn from(step: &ChainStep) -> Self {
        Self {
            mode: step.mode.clone(),
            trigger_on: step.trigger_on.as_ref()
                .map(|v| v.join(", "))
                .unwrap_or_default(),
            skip_on: step.skip_on.as_ref()
                .map(|v| v.join(", "))
                .unwrap_or_default(),
            agent: step.agent.clone().unwrap_or_default(),
            inject_context: step.inject_context.clone().unwrap_or_default(),
        }
    }
}

impl ChainStepEdit {
    pub fn to_chain_step(&self) -> ChainStep {
        ChainStep {
            mode: self.mode.clone(),
            trigger_on: if self.trigger_on.trim().is_empty() {
                None
            } else {
                Some(self.trigger_on.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
            },
            skip_on: if self.skip_on.trim().is_empty() {
                None
            } else {
                Some(self.skip_on.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
            },
            agent: if self.agent.trim().is_empty() { None } else { Some(self.agent.clone()) },
            inject_context: if self.inject_context.trim().is_empty() { None } else { Some(self.inject_context.clone()) },
        }
    }
}
