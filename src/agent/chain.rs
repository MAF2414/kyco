//! Chain execution for sequential mode runs.
//!
//! This module provides the [`ChainRunner`] which orchestrates multi-step workflows
//! by executing a sequence of modes where each step can pass context to the next.
//! Chains enable complex agent pipelines like "review → fix → test" with conditional
//! branching based on previous step outcomes.
//!
//! # Architecture
//!
//! A chain consists of [`ChainStep`]s that are executed in order. Each step:
//! - Runs a specific mode (e.g., "review", "fix", "test")
//! - Can be conditionally triggered based on the previous step's state
//! - Receives accumulated context from prior steps via the `summary` field
//! - Produces a state identifier that subsequent steps can react to
//!
//! # State-Based Control Flow
//!
//! Steps can use `trigger_on` and `skip_on` to create conditional workflows:
//!
//! ```text
//! review ──┬── [issues_found] ──► fix ──► test
//!          │
//!          └── [no_issues] ──► (chain ends early)
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use kyco::agent::{ChainRunner, ChainResult};
//! use kyco::config::ModeChain;
//!
//! // Create a chain runner
//! let runner = ChainRunner::new(&config, &agent_registry, &work_dir);
//!
//! // Execute a chain (steps defined in config)
//! let result: ChainResult = runner.run_chain(
//!     "review-fix",
//!     &chain_config,
//!     &initial_job,
//!     event_tx,
//! ).await;
//!
//! // Check results
//! if result.success {
//!     println!("Chain completed: {:?}", result.final_state);
//! }
//! ```

use regex::Regex;
use std::path::Path;
use tokio::sync::mpsc;

use crate::config::{ChainStep, Config, ModeChain, StateDefinition};
use crate::{Job, JobResult, LogEvent};

use super::AgentRegistry;

/// Result of a single step in a chain.
///
/// Captures the outcome of executing one step, including whether it was skipped
/// due to trigger conditions not being met. When a step runs, both `job_result`
/// (parsed YAML output) and `agent_result` (execution metadata) are populated.
#[derive(Debug, Clone)]
pub struct ChainStepResult {
    /// The mode that was executed (e.g., "review", "fix").
    pub mode: String,
    /// Zero-based index of this step in the chain.
    pub step_index: usize,
    /// `true` if the step was skipped due to `trigger_on`/`skip_on` conditions.
    pub skipped: bool,
    /// Parsed job result from the agent's YAML output block, if the step ran.
    pub job_result: Option<JobResult>,
    /// Summary of agent execution (success, errors, file changes), if the step ran.
    pub agent_result: Option<AgentResultSummary>,
    /// Full response text from the agent (for UI display).
    pub full_response: Option<String>,
}

/// Summarized agent result for chain step tracking.
///
/// A lightweight view of [`super::AgentResult`] that omits large fields like
/// full output text and file diffs, suitable for storing in chain history.
#[derive(Debug, Clone)]
pub struct AgentResultSummary {
    /// Whether the agent completed without errors.
    pub success: bool,
    /// Error message if the agent failed.
    pub error: Option<String>,
    /// Number of files modified by this step.
    pub files_changed: usize,
}

/// Result of running a complete chain.
///
/// Contains the full execution history of the chain, including results from
/// each step (whether skipped or executed) and accumulated context summaries.
///
/// # Fields
///
/// - `success`: `true` only if all executed steps succeeded. A chain with
///   skipped steps can still be successful if no executed step failed.
/// - `final_state`: The state identifier from the last **executed** step,
///   used for downstream decision-making or reporting.
/// - `accumulated_summaries`: Ordered list of `"[mode] summary"` strings
///   from all executed steps, useful for debugging or generating reports.
#[derive(Debug)]
pub struct ChainResult {
    /// Name of the chain that was executed.
    pub chain_name: String,
    /// Results from each step in order, including skipped steps.
    pub step_results: Vec<ChainStepResult>,
    /// `true` if no executed step failed.
    pub success: bool,
    /// State identifier from the last executed step (e.g., "issues_found").
    pub final_state: Option<String>,
    /// Accumulated `"[mode] summary"` entries from all executed steps.
    pub accumulated_summaries: Vec<String>,
}

/// Progress event sent during chain execution for real-time UI updates.
#[derive(Debug, Clone)]
pub struct ChainProgressEvent {
    /// Current step index (0-based)
    pub step_index: usize,
    /// Total number of steps
    pub total_steps: usize,
    /// Mode being executed
    pub mode: String,
    /// Whether the step is starting (true) or completed (false)
    pub is_starting: bool,
    /// Step result (only present when is_starting is false)
    pub step_result: Option<ChainStepResult>,
}

/// Executes mode chains by orchestrating sequential agent runs.
///
/// `ChainRunner` is the core executor for multi-step workflows. It iterates
/// through chain steps, evaluates trigger conditions, builds prompts with
/// accumulated context, and dispatches jobs to the appropriate agents.
///
/// # Lifetime
///
/// The `'a` lifetime ties the runner to its configuration and registry,
/// ensuring they remain valid throughout chain execution.
///
/// # Example
///
/// ```rust,ignore
/// let runner = ChainRunner::new(&config, &registry, &work_dir);
/// let result = runner.run_chain("my-chain", &chain, &job, event_tx).await;
/// ```
pub struct ChainRunner<'a> {
    /// Application configuration with mode and agent definitions.
    config: &'a Config,
    /// Registry providing access to agent adapters.
    agent_registry: &'a AgentRegistry,
    /// Working directory for agent execution.
    work_dir: &'a Path,
}

impl<'a> ChainRunner<'a> {
    /// Creates a new chain runner with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration containing mode definitions
    /// * `agent_registry` - Registry for looking up agent adapters
    /// * `work_dir` - Working directory where agents will execute
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
    ///
    /// # Arguments
    ///
    /// * `chain_name` - Identifier for this chain execution (for logging)
    /// * `chain` - The chain configuration defining steps and behavior
    /// * `initial_job` - The originating job that triggered this chain
    /// * `event_tx` - Channel for streaming log events during execution
    /// * `progress_tx` - Optional channel for real-time progress updates
    ///
    /// # Returns
    ///
    /// A [`ChainResult`] containing the outcome of each step and overall success status.
    /// If `chain.stop_on_failure` is `true`, execution halts on the first failure.
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
        let mut last_output: Option<String> = None; // Full response from previous step
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

        // Track the previous step's mode for auto-generating state patterns
        let mut last_mode: Option<String> = None;

        for (step_index, step) in chain.steps.iter().enumerate() {
            // Detect states from previous output
            // Priority: 1) chain's explicit state definitions, 2) auto-generate from mode's output_states
            let detected_states = if !chain.states.is_empty() {
                // Use chain's explicit state definitions
                self.detect_states(&chain.states, &last_output)
            } else if let Some(ref prev_mode) = last_mode {
                // Auto-generate states from previous mode's output_states
                self.detect_states_from_mode(prev_mode, &last_output)
            } else {
                Vec::new()
            };

            // Check if this step should run based on trigger conditions
            let should_run = self.should_step_run(step, &detected_states);

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
                    full_response: None,
                });
                continue;
            }

            // Log detected states for debugging
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

            // Send progress event: step starting
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(ChainProgressEvent {
                    step_index,
                    total_steps: chain.steps.len(),
                    mode: step.mode.clone(),
                    is_starting: true,
                    step_result: None,
                });
            }

            // Determine what context to pass: full response or just summary
            let previous_context = if chain.pass_full_response {
                last_output.clone()
            } else {
                last_summary.clone()
            };

            // Build the prompt with previous context
            let chained_prompt = self.build_chained_prompt(
                initial_job,
                step,
                &previous_context,
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
                        full_response: None,
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
                    // Store full output for state detection and context passing
                    last_output = agent_result.output_text.clone();

                    // Parse the job result
                    let job_result = agent_result
                        .output_text
                        .as_ref()
                        .and_then(|text| JobResult::parse(text));

                    // Update state and summary for next step
                    if let Some(ref jr) = job_result {
                        // Legacy: still use YAML state if defined (for backwards compatibility)
                        last_state = jr.state.clone();
                        if let Some(ref summary) = jr.summary {
                            last_summary = Some(summary.clone());
                            accumulated_summaries.push(format!("[{}] {}", step.mode, summary));
                        } else if let Some(ref details) = jr.details {
                            // Fall back to details if no summary
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

                    // Send progress event: step completed
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

                    // Track the mode for auto-generating state patterns in the next step
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

    /// Detects states from output text using the chain's state definitions.
    ///
    /// Iterates through each state definition and checks if any of its patterns
    /// match the output text. Returns all matching state IDs.
    ///
    /// # Arguments
    ///
    /// * `states` - State definitions from the chain configuration
    /// * `output` - Output text from the previous step
    ///
    /// # Returns
    ///
    /// Vector of state IDs that were detected in the output.
    fn detect_states(&self, states: &[StateDefinition], output: &Option<String>) -> Vec<String> {
        let Some(output_text) = output else {
            return Vec::new();
        };

        let mut detected = Vec::new();

        for state in states {
            let matched = state.patterns.iter().any(|pattern| {
                if state.is_regex {
                    // Regex matching
                    let regex_pattern = if state.case_insensitive {
                        format!("(?i){}", pattern)
                    } else {
                        pattern.clone()
                    };
                    match Regex::new(&regex_pattern) {
                        Ok(re) => re.is_match(output_text),
                        Err(_) => {
                            // Invalid regex - fall back to text search
                            if state.case_insensitive {
                                output_text.to_lowercase().contains(&pattern.to_lowercase())
                            } else {
                                output_text.contains(pattern)
                            }
                        }
                    }
                } else {
                    // Plain text search
                    if state.case_insensitive {
                        output_text.to_lowercase().contains(&pattern.to_lowercase())
                    } else {
                        output_text.contains(pattern)
                    }
                }
            });

            if matched {
                detected.push(state.id.clone());
            }
        }

        detected
    }

    /// Auto-detects states from output text using a mode's output_states.
    ///
    /// This enables chains to work without explicit state definitions by
    /// automatically generating patterns from the mode's output_states.
    /// For example, if a mode has `output_states = ["issues_found", "no_issues"]`,
    /// this will look for patterns like:
    /// - `state to "issues_found"` or `state: issues_found`
    /// - `issues_found` (the state name itself, case-insensitive)
    ///
    /// # Arguments
    ///
    /// * `mode_name` - The name of the mode to look up
    /// * `output` - The output text to search for state patterns
    ///
    /// # Returns
    ///
    /// A vector of detected state IDs from the mode's output_states.
    fn detect_states_from_mode(&self, mode_name: &str, output: &Option<String>) -> Vec<String> {
        let Some(output_text) = output else {
            return Vec::new();
        };

        // Look up the mode to get its output_states
        let Some(mode) = self.config.mode.get(mode_name) else {
            return Vec::new();
        };

        if mode.output_states.is_empty() {
            return Vec::new();
        }

        let output_lower = output_text.to_lowercase();
        let mut detected = Vec::new();

        for state_id in &mode.output_states {
            let state_lower = state_id.to_lowercase();

            // Check for various patterns that indicate this state
            let patterns = [
                format!("state to \"{}\"", state_lower),
                format!("state: {}", state_lower),
                format!("set state to {}", state_lower),
                format!("setting state to {}", state_lower),
                state_lower.clone(), // The state name itself
            ];

            let matched = patterns.iter().any(|p| output_lower.contains(p));

            if matched {
                detected.push(state_id.clone());
            }
        }

        detected
    }

    /// Evaluates whether a step should execute based on trigger conditions.
    ///
    /// The evaluation order is:
    /// 1. If `skip_on` contains any detected state → skip (return `false`)
    /// 2. If `trigger_on` is specified and no detected state matches → skip
    /// 3. Otherwise → run (return `true`)
    ///
    /// # Arguments
    ///
    /// * `step` - The step configuration with optional `trigger_on`/`skip_on`
    /// * `detected_states` - States detected from the previous step's output
    ///
    /// # Returns
    ///
    /// `true` if the step should execute, `false` if it should be skipped.
    fn should_step_run(&self, step: &ChainStep, detected_states: &[String]) -> bool {
        // Check skip_on first - if any detected state matches, don't run
        if let Some(skip_states) = &step.skip_on {
            for detected in detected_states {
                if skip_states.contains(detected) {
                    return false;
                }
            }
        }

        // Check trigger_on - if specified, at least one detected state must match
        if let Some(trigger_states) = &step.trigger_on {
            if detected_states.is_empty() {
                return false; // No states detected, can't trigger
            }
            // Check if any detected state is in trigger_on
            detected_states.iter().any(|d| trigger_states.contains(d))
        } else {
            // No trigger condition = always run
            true
        }
    }

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
    /// * `initial_job` - The original job for scope/target information
    /// * `step` - Current step configuration
    /// * `last_summary` - Summary text from the previous step
    /// * `accumulated_summaries` - All summaries from prior steps
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
    /// * `initial_job` - The original job to clone
    /// * `step` - Step configuration with mode and optional agent override
    /// * `prompt` - The full prompt built by [`Self::build_chained_prompt`]
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
