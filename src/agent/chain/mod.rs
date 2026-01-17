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
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::config::{Config, ModeChain};
use crate::bugbounty::BugBountyManager;
use crate::{AgentConfig, Job, LogEvent};

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

fn path_ends_with(full: &Path, suffix: &Path) -> bool {
    use std::path::Component;

    let full_parts: Vec<_> = full
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s),
            _ => None,
        })
        .collect();
    let suffix_parts: Vec<_> = suffix
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s),
            _ => None,
        })
        .collect();

    if suffix_parts.is_empty() {
        return true;
    }
    if full_parts.len() < suffix_parts.len() {
        return false;
    }

    let start = full_parts.len() - suffix_parts.len();
    full_parts[start..] == suffix_parts[..]
}

fn apply_bugbounty_tooling_policy(step_job: &Job, work_dir: &Path, agent_config: &mut AgentConfig) {
    let Some(project_id) = step_job.bugbounty_project_id.as_deref() else {
        return;
    };

    let Ok(bb) = BugBountyManager::new() else {
        return;
    };
    let Ok(Some(project)) = bb.get_project(project_id) else {
        return;
    };

    agent_config
        .env
        .insert("KYCO_BUGBOUNTY_ENFORCE".to_string(), "1".to_string());
    agent_config.env.insert(
        "KYCO_BUGBOUNTY_PROJECT_ID".to_string(),
        project.id.clone(),
    );

    let root_raw = std::path::PathBuf::from(&project.root_path);
    let root_abs = if root_raw.is_absolute() {
        root_raw
    } else {
        let candidate = work_dir.join(&root_raw);
        if candidate.exists() {
            candidate
        } else if path_ends_with(work_dir, &root_raw) {
            work_dir.to_path_buf()
        } else {
            candidate
        }
    };
    let root_abs = root_abs.canonicalize().unwrap_or(root_abs);
    agent_config.env.insert(
        "KYCO_BUGBOUNTY_PROJECT_ROOT".to_string(),
        root_abs.to_string_lossy().to_string(),
    );

    if let Some(ref scope) = project.scope {
        if let Ok(json) = serde_json::to_string(scope) {
            agent_config
                .env
                .insert("KYCO_BUGBOUNTY_SCOPE_JSON".to_string(), json);
        }
    }
    if let Some(ref policy) = project.tool_policy {
        if let Ok(json) = serde_json::to_string(policy) {
            agent_config
                .env
                .insert("KYCO_BUGBOUNTY_TOOL_POLICY_JSON".to_string(), json);
        }
    }

    // Convert ToolPolicy into tool-level blocks (best-effort).
    if let Some(policy) = project.tool_policy {
        let mut blocked = policy.blocked_commands.clone();
        if policy.network_wrapper.is_some() {
            for cmd in ["curl", "wget", "nc", "nmap"] {
                if !blocked.iter().any(|c| c.eq_ignore_ascii_case(cmd)) {
                    blocked.push(cmd.to_string());
                }
            }
        }

        for cmd in &blocked {
            let cmd = cmd.trim();
            if cmd.is_empty() {
                continue;
            }
            let pattern = format!("Bash({}:*)", cmd);
            if !agent_config.disallowed_tools.contains(&pattern) {
                agent_config.disallowed_tools.push(pattern);
            }
        }
    }
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
    ///
    /// Supports `loop_to` for restarting from a previous step (limited by `max_loops`).
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
        let mut last_skill: Option<String> = None;
        let mut loop_count: u32 = 0;

        let _ = event_tx
            .send(LogEvent::system(format!(
                "Starting chain '{}' with {} steps",
                chain_name,
                chain.steps.len()
            )))
            .await;

        let mut step_index: usize = 0;
        while step_index < chain.steps.len() {
            let step = &chain.steps[step_index];
            // Clone skill once per iteration into Arc<str> for cheap reuse
            let skill: Arc<str> = Arc::from(step.skill.as_str());

            // Detect states from previous output
            let detected_states = if !chain.states.is_empty() {
                state::detect_states(&chain.states, &last_output)
            } else if let Some(ref prev_skill) = last_skill {
                state::detect_states_from_skill(self.config, prev_skill, &last_output)
            } else {
                Vec::new()
            };

            // Check if this step should run based on trigger conditions
            let should_run = state::should_step_run(step, &detected_states);

            // Handle loop_to: if step has loop_to and would run, jump back instead
            if should_run {
                if let Some(ref loop_target) = step.loop_to {
                    if loop_count < chain.max_loops {
                        // Find the target step index by mode name
                        if let Some(target_idx) = chain.steps.iter().position(|s| &s.skill == loop_target) {
                            loop_count += 1;
                            let _ = event_tx
                                .send(LogEvent::system(format!(
                                    "Loop triggered: jumping back to '{}' (loop {}/{})",
                                    loop_target, loop_count, chain.max_loops
                                )))
                                .await;
                            step_index = target_idx;
                            continue;
                        } else {
                            let _ = event_tx
                                .send(LogEvent::error(format!(
                                    "loop_to target '{}' not found in chain",
                                    loop_target
                                )))
                                .await;
                        }
                    } else {
                        let _ = event_tx
                            .send(LogEvent::system(format!(
                                "Max loops ({}) reached, continuing without loop",
                                chain.max_loops
                            )))
                            .await;
                    }
                }
            }

            if !should_run {
                let _ = event_tx
                    .send(LogEvent::system(format!(
                        "Skipping step {} ({}) - trigger condition not met",
                        step_index + 1,
                        &skill
                    )))
                    .await;

                step_results.push(ChainStepResult {
                    skill: Arc::clone(&skill),
                    step_index,
                    skipped: true,
                    job_result: None,
                    agent_result: None,
                    full_response: None,
                });
                step_index += 1;
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
                    "Executing chain step {} of {}: mode '{}'{}",
                    step_index + 1,
                    chain.steps.len(),
                    &skill,
                    if loop_count > 0 { format!(" (loop {})", loop_count) } else { String::new() }
                )))
                .await;

            if let Some(ref tx) = progress_tx {
                let _ = tx.send(ChainProgressEvent {
                    step_index,
                    total_steps: chain.steps.len(),
                    skill: Arc::clone(&skill),
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
            let default_agent = self.config.get_agent_for_mode(&step.skill);
            let agent_id: &str = step.agent.as_deref().unwrap_or(&default_agent);
            let mut agent_config = self
                .config
                .get_agent_for_job(agent_id, &step.skill)
                .unwrap_or_default();

            apply_bugbounty_tooling_policy(&step_job, self.work_dir, &mut agent_config);

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
                        skill: Arc::clone(&skill),
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
                    step_index += 1;
                    continue;
                }
            };

            let result = adapter
                .run(&step_job, self.work_dir, &agent_config, event_tx.clone())
                .await;

            match result {
                Ok(agent_result) => {
                    // Extract Copy fields before moving owned fields
                    let agent_success = agent_result.success;
                    let files_changed = agent_result.changed_files.len();
                    // Move owned fields instead of cloning
                    let agent_error = agent_result.error;
                    last_output = agent_result.output_text;

                    let job_result = last_output
                        .as_ref()
                        .and_then(|text| crate::JobResult::parse(text));

                    if let Some(ref jr) = job_result {
                        // Clone from reference - unavoidable as jr is borrowed
                        last_state.clone_from(&jr.state);
                        if let Some(ref summary) = jr.summary {
                            last_summary = Some(summary.clone());
                            accumulated_summaries.push(format!("[{}] {}", &skill, summary));
                        } else if let Some(ref details) = jr.details {
                            last_summary = Some(details.clone());
                            accumulated_summaries.push(format!("[{}] {}", &skill, details));
                        }
                    }

                    let step_result = ChainStepResult {
                        skill: Arc::clone(&skill),
                        step_index,
                        skipped: false,
                        job_result,
                        agent_result: Some(AgentResultSummary {
                            success: agent_success,
                            error: agent_error,
                            files_changed,
                        }),
                        full_response: last_output.clone(),
                    };

                    if let Some(ref tx) = progress_tx {
                        let _ = tx.send(ChainProgressEvent {
                            step_index,
                            total_steps: chain.steps.len(),
                            skill: Arc::clone(&skill),
                            is_starting: false,
                            step_result: Some(step_result.clone()),
                        });
                    }

                    step_results.push(step_result);
                    last_skill = Some(skill.to_string());

                    if !agent_success && chain.stop_on_failure {
                        chain_success = false;
                        let _ = event_tx
                            .send(LogEvent::error(format!(
                                "Chain stopped: step {} ({}) failed",
                                step_index + 1,
                                &skill
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
                            &skill,
                            e
                        )))
                        .await;

                    step_results.push(ChainStepResult {
                        skill: Arc::clone(&skill),
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

            step_index += 1;
        }

        let _ = event_tx
            .send(LogEvent::system(format!(
                "Chain '{}' completed: {} steps executed, {} loops, success: {}",
                chain_name,
                step_results.iter().filter(|r| !r.skipped).count(),
                loop_count,
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
