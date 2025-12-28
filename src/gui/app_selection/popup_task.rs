//! Popup task execution for KycoApp

use crate::gui::app::KycoApp;
use crate::gui::app_types::ViewMode;
use crate::gui::jobs;
use crate::gui::selection::autocomplete::parse_input_multi;
use crate::LogEvent;

impl KycoApp {
    /// Execute the task from selection popup
    /// If force_worktree is true, the job will run in a git worktree regardless of global settings
    pub(crate) fn execute_popup_task(&mut self, force_worktree: bool) {
        // Use the multi-agent parser to support "claude+codex:mode" syntax
        let (agents, mode, prompt) = parse_input_multi(&self.popup_input);

        if mode.is_empty() {
            self.popup_status = Some((
                "Please enter a mode (e.g., 'refactor', 'fix')".to_string(),
                true,
            ));
            return;
        }

        // Resolve agent aliases
        let resolved_agents: Vec<String> = {
            let Ok(config) = self.config.read() else {
                self.logs.push(LogEvent::error("Config lock poisoned"));
                return;
            };
            agents
                .iter()
                .map(|a| {
                    config
                        .agent
                        .iter()
                        .find(|(name, cfg)| {
                            name.eq_ignore_ascii_case(a)
                                || cfg
                                    .aliases
                                    .iter()
                                    .any(|alias| alias.eq_ignore_ascii_case(a))
                        })
                        .map(|(name, _)| name.clone())
                        .unwrap_or_else(|| a.clone())
                })
                .collect()
        };

        // Remove duplicates and map legacy agents.
        let mut seen = std::collections::HashSet::new();
        let resolved_agents: Vec<String> = resolved_agents
            .into_iter()
            .map(|a| match a.as_str() {
                "g" | "gm" | "gemini" | "custom" => "claude".to_string(),
                _ => a,
            })
            .filter(|a| seen.insert(a.clone()))
            .collect();

        let resolved_agents = if resolved_agents.is_empty() {
            vec!["claude".to_string()]
        } else {
            resolved_agents
        };

        // Create job(s) - uses multi-agent creation for parallel execution
        if let Some(result) = jobs::create_jobs_from_selection_multi(
            &self.job_manager,
            &self.group_manager,
            &self.selection,
            &resolved_agents,
            &mode,
            &prompt,
            &mut self.logs,
            force_worktree,
        ) {
            let selection_info = self
                .selection
                .selected_text
                .as_ref()
                .map(|s| format!("{} chars", s.len()))
                .unwrap_or_else(|| "no selection".to_string());

            if result.job_ids.len() == 1 {
                // Single agent
                let job_id = result.job_ids[0];
                self.popup_status = Some((
                    format!(
                        "Job #{} created: {}:{} ({})",
                        job_id, resolved_agents[0], mode, selection_info
                    ),
                    false,
                ));
                self.selected_job_id = Some(job_id);
            } else {
                // Multi-agent - show group info
                let agent_list = resolved_agents.join("+");
                self.popup_status = Some((
                    format!(
                        "Group #{} created: {} jobs ({}) for {}:{} ({})",
                        result.group_id.unwrap_or(0),
                        result.job_ids.len(),
                        agent_list,
                        agent_list,
                        mode,
                        selection_info
                    ),
                    false,
                ));
                // Select first job
                self.selected_job_id = result.job_ids.first().copied();
            }

            // Refresh job list
            self.refresh_jobs();

            // Return to job list view after a moment
            self.view_mode = ViewMode::JobList;
        } else {
            self.popup_status = Some(("Failed to create job".to_string(), true));
        }
    }
}
