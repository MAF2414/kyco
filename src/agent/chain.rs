//! Chain execution for sequential mode runs
//!
//! Chains allow executing multiple modes sequentially, where each mode
//! receives the output/summary of the previous mode as context.

use std::path::Path;
use tokio::sync::mpsc;

use crate::config::{ChainStep, Config, ModeChain};
use crate::{Job, JobResult, LogEvent};

use super::AgentRegistry;

/// Result of a single step in a chain
#[derive(Debug, Clone)]
pub struct ChainStepResult {
    /// The mode that was executed
    pub mode: String,
    /// The step index in the chain
    pub step_index: usize,
    /// Whether this step was skipped (due to trigger conditions)
    pub skipped: bool,
    /// The job result from this step (if not skipped)
    pub job_result: Option<JobResult>,
    /// The agent result (if not skipped)
    pub agent_result: Option<AgentResultSummary>,
}

/// Summarized agent result (without large data)
#[derive(Debug, Clone)]
pub struct AgentResultSummary {
    pub success: bool,
    pub error: Option<String>,
    pub files_changed: usize,
}

/// Result of running a complete chain
#[derive(Debug)]
pub struct ChainResult {
    /// Name of the chain
    pub chain_name: String,
    /// Results from each step
    pub step_results: Vec<ChainStepResult>,
    /// Whether the chain completed successfully
    pub success: bool,
    /// Final state from the last executed step
    pub final_state: Option<String>,
    /// Accumulated context from all steps
    pub accumulated_summaries: Vec<String>,
}

/// Executes mode chains
pub struct ChainRunner<'a> {
    config: &'a Config,
    agent_registry: &'a AgentRegistry,
    work_dir: &'a Path,
}

impl<'a> ChainRunner<'a> {
    pub fn new(
        config: &'a Config,
        agent_registry: &'a AgentRegistry,
        work_dir: &'a Path,
    ) -> Self {
        Self {
            config,
            agent_registry,
            work_dir,
        }
    }

    /// Run a mode chain
    pub async fn run_chain(
        &self,
        chain_name: &str,
        chain: &ModeChain,
        initial_job: &Job,
        event_tx: mpsc::Sender<LogEvent>,
    ) -> ChainResult {
        let mut step_results = Vec::new();
        let mut last_state: Option<String> = None;
        let mut last_summary: Option<String> = None;
        let mut accumulated_summaries = Vec::new();
        let mut chain_success = true;

        let _ = event_tx
            .send(LogEvent::system(format!(
                "Starting chain '{}' with {} steps",
                chain_name,
                chain.steps.len()
            )))
            .await;

        for (step_index, step) in chain.steps.iter().enumerate() {
            // Check if this step should run based on trigger conditions
            let should_run = self.should_step_run(step, &last_state);

            if !should_run {
                let _ = event_tx
                    .send(LogEvent::system(format!(
                        "Skipping step {} ({}) - trigger condition not met",
                        step_index + 1,
                        step.mode
                    )))
                    .await;

                step_results.push(ChainStepResult {
                    mode: step.mode.clone(),
                    step_index,
                    skipped: true,
                    job_result: None,
                    agent_result: None,
                });
                continue;
            }

            let _ = event_tx
                .send(LogEvent::system(format!(
                    "Executing chain step {} of {}: mode '{}'",
                    step_index + 1,
                    chain.steps.len(),
                    step.mode
                )))
                .await;

            // Build the prompt with previous context
            let chained_prompt = self.build_chained_prompt(
                initial_job,
                step,
                &last_summary,
                &accumulated_summaries,
            );

            // Create a modified job for this step
            let step_job = self.create_step_job(initial_job, step, &chained_prompt);

            // Get the agent config with mode-specific tool overrides
            let default_agent = self.config.get_agent_for_mode(&step.mode);
            let agent_id = step.agent.as_ref().unwrap_or(&default_agent);
            let agent_config = self
                .config
                .get_agent_for_job(agent_id, &step.mode)
                .unwrap_or_default();

            // Get the adapter
            let adapter = match self.agent_registry.get_for_config(&agent_config) {
                Some(a) => a,
                None => {
                    let _ = event_tx
                        .send(LogEvent::error(format!(
                            "No adapter found for agent '{}'",
                            agent_id
                        )))
                        .await;

                    step_results.push(ChainStepResult {
                        mode: step.mode.clone(),
                        step_index,
                        skipped: false,
                        job_result: None,
                        agent_result: Some(AgentResultSummary {
                            success: false,
                            error: Some(format!("No adapter for agent '{}'", agent_id)),
                            files_changed: 0,
                        }),
                    });

                    if chain.stop_on_failure {
                        chain_success = false;
                        break;
                    }
                    continue;
                }
            };

            // Run the step
            let result = adapter
                .run(&step_job, self.work_dir, &agent_config, event_tx.clone())
                .await;

            match result {
                Ok(agent_result) => {
                    // Parse the job result
                    let job_result = agent_result
                        .output_text
                        .as_ref()
                        .and_then(|text| JobResult::parse(text));

                    // Update state and summary for next step
                    if let Some(ref jr) = job_result {
                        last_state = jr.state.clone();
                        if let Some(ref summary) = jr.summary {
                            last_summary = Some(summary.clone());
                            accumulated_summaries.push(format!(
                                "[{}] {}",
                                step.mode,
                                summary
                            ));
                        } else if let Some(ref details) = jr.details {
                            // Fall back to details if no summary
                            last_summary = Some(details.clone());
                            accumulated_summaries.push(format!(
                                "[{}] {}",
                                step.mode,
                                details
                            ));
                        }
                    }

                    step_results.push(ChainStepResult {
                        mode: step.mode.clone(),
                        step_index,
                        skipped: false,
                        job_result,
                        agent_result: Some(AgentResultSummary {
                            success: agent_result.success,
                            error: agent_result.error,
                            files_changed: agent_result.changed_files.len(),
                        }),
                    });

                    if !agent_result.success && chain.stop_on_failure {
                        chain_success = false;
                        let _ = event_tx
                            .send(LogEvent::error(format!(
                                "Chain stopped: step {} ({}) failed",
                                step_index + 1,
                                step.mode
                            )))
                            .await;
                        break;
                    }
                }
                Err(e) => {
                    let _ = event_tx
                        .send(LogEvent::error(format!(
                            "Step {} ({}) error: {}",
                            step_index + 1,
                            step.mode,
                            e
                        )))
                        .await;

                    step_results.push(ChainStepResult {
                        mode: step.mode.clone(),
                        step_index,
                        skipped: false,
                        job_result: None,
                        agent_result: Some(AgentResultSummary {
                            success: false,
                            error: Some(e.to_string()),
                            files_changed: 0,
                        }),
                    });

                    if chain.stop_on_failure {
                        chain_success = false;
                        break;
                    }
                }
            }
        }

        let _ = event_tx
            .send(LogEvent::system(format!(
                "Chain '{}' completed: {} steps executed, success: {}",
                chain_name,
                step_results.iter().filter(|r| !r.skipped).count(),
                chain_success
            )))
            .await;

        ChainResult {
            chain_name: chain_name.to_string(),
            step_results,
            success: chain_success,
            final_state: last_state,
            accumulated_summaries,
        }
    }

    /// Check if a step should run based on its trigger conditions
    fn should_step_run(&self, step: &ChainStep, last_state: &Option<String>) -> bool {
        // Check skip_on first - if matched, don't run
        if let Some(skip_states) = &step.skip_on {
            if let Some(state) = last_state {
                if skip_states.contains(state) {
                    return false;
                }
            }
        }

        // Check trigger_on - if specified, must match
        if let Some(trigger_states) = &step.trigger_on {
            match last_state {
                Some(state) => trigger_states.contains(state),
                None => false, // No previous state, can't trigger
            }
        } else {
            // No trigger condition = always run
            true
        }
    }

    /// Build a prompt that includes context from previous chain steps
    fn build_chained_prompt(
        &self,
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

        let base_prompt = self.config.build_prompt(
            &step.mode,
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
            for summary in accumulated_summaries.iter().take(accumulated_summaries.len() - 1) {
                prompt.push_str("- ");
                prompt.push_str(summary);
                prompt.push('\n');
            }
        }

        prompt
    }

    /// Create a job for a specific chain step
    fn create_step_job(&self, initial_job: &Job, step: &ChainStep, prompt: &str) -> Job {
        let mut step_job = initial_job.clone();
        step_job.mode = step.mode.clone();
        step_job.description = Some(prompt.to_string());
        if let Some(agent) = &step.agent {
            step_job.agent_id = agent.clone();
        } else {
            step_job.agent_id = self.config.get_agent_for_mode(&step.mode);
        }
        step_job
    }
}
