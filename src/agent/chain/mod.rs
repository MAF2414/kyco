//! Chain execution for sequential mode runs.
//!
//! This module provides the [`ChainRunner`] which orchestrates multi-step workflows
//! by executing a sequence of modes where each step can pass context to the next.
//! Chains enable complex agent pipelines like "review → fix → test" with conditional
//! branching based on previous step outcomes.

mod prompt;
mod state;
mod types;

use std::path::Path;
use tokio::sync::mpsc;

use crate::config::{Config, ModeChain};
use crate::{Job, LogEvent};

use super::AgentRegistry;

pub use types::{AgentResultSummary, ChainProgressEvent, ChainResult, ChainStepResult};

/// Executes mode chains by orchestrating sequential agent runs.
///
/// `ChainRunner` is the core executor for multi-step workflows. It iterates
/// through chain steps, evaluates trigger conditions, builds prompts with
/// accumulated context, and dispatches jobs to the appropriate agents.
pub struct ChainRunner<'a> {
    config: &'a Config,
    agent_registry: &'a AgentRegistry,
    work_dir: &'a Path,
}

impl<'a> ChainRunner<'a> {
    /// Creates a new chain runner with the given configuration.
    pub fn new(config: &'a Config, agent_registry: &'a AgentRegistry, work_dir: &'a Path) -> Self {
        Self {
            config,
            agent_registry,
            work_dir,
        }
    }

    /// Executes a complete mode chain.
    ///
    /// Iterates through each step in the chain, evaluating trigger conditions
    /// and executing the appropriate agent. Context (summaries) from each step
    /// is accumulated and passed to subsequent steps.
    pub async fn run_chain(
        &self,
        chain_name: &str,
        chain: &ModeChain,
        initial_job: &Job,
        event_tx: mpsc::Sender<LogEvent>,
        progress_tx: Option<std::sync::mpsc::Sender<ChainProgressEvent>>,
    ) -> ChainResult {
        let mut step_results = Vec::new();
        let mut last_state: Option<String> = None;
        let mut last_output: Option<String> = None;
        let mut last_summary: Option<String> = None;
        let mut accumulated_summaries = Vec::new();
        let mut chain_success = true;
        let mut last_mode: Option<String> = None;

        let _ = event_tx
            .send(LogEvent::system(format!(
                "Starting chain '{}' with {} steps",
                chain_name,
                chain.steps.len()
            )))
            .await;

        for (step_index, step) in chain.steps.iter().enumerate() {
            // Detect states from previous output
            let detected_states = if !chain.states.is_empty() {
                state::detect_states(&chain.states, &last_output)
            } else if let Some(ref prev_mode) = last_mode {
                state::detect_states_from_mode(self.config, prev_mode, &last_output)
            } else {
                Vec::new()
            };

            // Check if this step should run based on trigger conditions
            if !state::should_step_run(step, &detected_states) {
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
                    full_response: None,
                });
                continue;
            }

            if !detected_states.is_empty() {
                let _ = event_tx
                    .send(LogEvent::system(format!(
                        "Detected states from previous step: {:?}",
                        detected_states
                    )))
                    .await;
            }

            let _ = event_tx
                .send(LogEvent::system(format!(
                    "Executing chain step {} of {}: mode '{}'",
                    step_index + 1,
                    chain.steps.len(),
                    step.mode
                )))
                .await;

            if let Some(ref tx) = progress_tx {
                let _ = tx.send(ChainProgressEvent {
                    step_index,
                    total_steps: chain.steps.len(),
                    mode: step.mode.clone(),
                    is_starting: true,
                    step_result: None,
                });
            }

            // Build the prompt with previous context
            let previous_context = if chain.pass_full_response {
                last_output.clone()
            } else {
                last_summary.clone()
            };

            let chained_prompt = prompt::build_chained_prompt(
                self.config,
                initial_job,
                step,
                &previous_context,
                &accumulated_summaries,
            );

            let step_job = prompt::create_step_job(self.config, initial_job, step, &chained_prompt);

            // Get the agent config and adapter
            let default_agent = self.config.get_agent_for_mode(&step.mode);
            let agent_id = step.agent.as_ref().unwrap_or(&default_agent);
            let agent_config = self
                .config
                .get_agent_for_job(agent_id, &step.mode)
                .unwrap_or_default();

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
                        full_response: None,
                    });

                    if chain.stop_on_failure {
                        chain_success = false;
                        break;
                    }
                    continue;
                }
            };

            let result = adapter
                .run(&step_job, self.work_dir, &agent_config, event_tx.clone())
                .await;

            match result {
                Ok(agent_result) => {
                    last_output = agent_result.output_text.clone();

                    let job_result = agent_result
                        .output_text
                        .as_ref()
                        .and_then(|text| crate::JobResult::parse(text));

                    if let Some(ref jr) = job_result {
                        last_state = jr.state.clone();
                        if let Some(ref summary) = jr.summary {
                            last_summary = Some(summary.clone());
                            accumulated_summaries.push(format!("[{}] {}", step.mode, summary));
                        } else if let Some(ref details) = jr.details {
                            last_summary = Some(details.clone());
                            accumulated_summaries.push(format!("[{}] {}", step.mode, details));
                        }
                    }

                    let step_result = ChainStepResult {
                        mode: step.mode.clone(),
                        step_index,
                        skipped: false,
                        job_result,
                        agent_result: Some(AgentResultSummary {
                            success: agent_result.success,
                            error: agent_result.error.clone(),
                            files_changed: agent_result.changed_files.len(),
                        }),
                        full_response: last_output.clone(),
                    };

                    if let Some(ref tx) = progress_tx {
                        let _ = tx.send(ChainProgressEvent {
                            step_index,
                            total_steps: chain.steps.len(),
                            mode: step.mode.clone(),
                            is_starting: false,
                            step_result: Some(step_result.clone()),
                        });
                    }

                    step_results.push(step_result);
                    last_mode = Some(step.mode.clone());

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
                        full_response: None,
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
}
