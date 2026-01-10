//! Prompt building for chain steps.
//!
//! This module handles constructing prompts with accumulated context
//! and creating step jobs for chain execution.

use crate::config::{ChainStep, Config};
use crate::Job;

/// Builds a prompt that includes context from previous chain steps.
///
/// Constructs the full prompt for a step by combining:
/// 1. The base mode prompt (from configuration)
/// 2. The summary from the immediately previous step
/// 3. Any custom `inject_context` from the step configuration
/// 4. Chain history (summaries from all prior steps, for later steps)
///
/// # Arguments
///
/// * `config` - Application configuration with mode definitions
/// * `initial_job` - The original job for scope/target information
/// * `step` - Current step configuration
/// * `last_summary` - Summary text from the previous step
/// * `accumulated_summaries` - All summaries from prior steps
pub fn build_chained_prompt(
    config: &Config,
    initial_job: &Job,
    step: &ChainStep,
    last_summary: &Option<String>,
    accumulated_summaries: &[String],
) -> String {
    // Determine scope type from ScopeDefinition
    let scope_type = if initial_job.scope.function_name.is_some() {
        "function"
    } else if initial_job.scope.dir_path.is_some() {
        "directory"
    } else if !initial_job.scope.file_path.as_os_str().is_empty() {
        "file"
    } else {
        "project"
    };

    let base_prompt = config.build_prompt(
        &step.skill,
        &initial_job.target,
        scope_type,
        initial_job.source_file.to_str().unwrap_or(""),
        initial_job.description.as_deref().unwrap_or(""),
    );

    let mut prompt = base_prompt;

    // Add previous step context
    if let Some(summary) = last_summary {
        prompt.push_str("\n\n## Context from previous step:\n");
        prompt.push_str(summary);
    }

    // Add injected context if specified
    if let Some(inject) = &step.inject_context {
        prompt.push_str("\n\n## Additional context:\n");
        prompt.push_str(inject);
    }

    // For later steps, optionally include accumulated history
    if accumulated_summaries.len() > 1 {
        prompt.push_str("\n\n## Chain history:\n");
        for summary in accumulated_summaries
            .iter()
            .take(accumulated_summaries.len() - 1)
        {
            prompt.push_str("- ");
            prompt.push_str(summary);
            prompt.push('\n');
        }
    }

    prompt
}

/// Creates a job for a specific chain step.
///
/// Clones the initial job and modifies it for this step by:
/// - Setting the mode to the step's mode
/// - Replacing the description with the chained prompt
/// - Overriding the agent if specified in the step configuration
///
/// # Arguments
///
/// * `config` - Application configuration for agent defaults
/// * `initial_job` - The original job to clone
/// * `step` - Step configuration with mode and optional agent override
/// * `prompt` - The full prompt built by [`build_chained_prompt`]
pub fn create_step_job(config: &Config, initial_job: &Job, step: &ChainStep, prompt: &str) -> Job {
    let mut step_job = initial_job.clone();
    step_job.skill = step.skill.clone();
    step_job.description = Some(prompt.to_string());
    if let Some(agent) = &step.agent {
        step_job.agent_id = agent.clone();
    } else {
        step_job.agent_id = config.get_agent_for_mode(&step.skill).into_owned();
    }
    step_job
}
