//! Selection handling methods for KycoApp
//!
//! Contains IDE selection/batch request handling and popup task execution.

use super::app::KycoApp;
use super::app_types::ViewMode;
use super::http_server::{BatchRequest, SelectionRequest};
use super::jobs;
use super::selection::SelectionContext;
use super::selection::autocomplete::parse_input_multi;
use crate::LogEvent;
use eframe::egui;
use std::path::PathBuf;
use tracing::info;

impl KycoApp {
    /// Handle incoming selection from IDE extension
    pub(crate) fn on_selection_received(&mut self, req: SelectionRequest, ctx: &egui::Context) {
        info!(
            "[kyco:gui] Received selection: file={:?}, lines={:?}-{:?}, deps={:?}, tests={:?}, project_root={:?}, git_root={:?}, workspace={:?}",
            req.file_path,
            req.line_start,
            req.line_end,
            req.dependency_count,
            req.related_tests.as_ref().map(|t| t.len()),
            req.project_root,
            req.git_root,
            req.workspace
        );

        // Auto-register workspace from IDE request
        // Priority: project_root (includes git detection) > workspace > active workspace
        let effective_path = req
            .project_root
            .as_ref()
            .or(req.git_root.as_ref())
            .or(req.workspace.as_ref());

        let (workspace_id, workspace_path) = if let Some(ref ws_path) = effective_path {
            let ws_path_buf = PathBuf::from(ws_path);
            if let Ok(mut registry) = self.workspace_registry.lock() {
                let ws_id = registry.get_or_create(ws_path_buf.clone());
                // Switch to this workspace and update active
                registry.set_active(ws_id);
                self.active_workspace_id = Some(ws_id);
                // Save registry to persist the new workspace
                if let Err(e) = registry.save_default() {
                    self.logs.push(LogEvent::error(format!(
                        "Failed to save workspace registry: {}",
                        e
                    )));
                }
                (Some(ws_id), Some(ws_path_buf))
            } else {
                (None, Some(ws_path_buf))
            }
        } else {
            // Use currently active workspace if no workspace specified
            (
                self.active_workspace_id,
                self.active_workspace_id.and_then(|id| {
                    self.workspace_registry
                        .lock()
                        .ok()
                        .and_then(|r| r.get(id).map(|w| w.path.clone()))
                }),
            )
        };

        self.selection = SelectionContext {
            app_name: Some("IDE".to_string()),
            file_path: req.file_path,
            selected_text: req.selected_text,
            line_number: req.line_start,
            line_end: req.line_end,
            possible_files: Vec::new(),
            dependencies: req.dependencies,
            dependency_count: req.dependency_count,
            additional_dependency_count: req.additional_dependency_count,
            related_tests: req.related_tests,
            diagnostics: req.diagnostics,
            workspace_id,
            workspace_path,
        };

        // Show selection popup
        self.view_mode = ViewMode::SelectionPopup;
        self.popup_input.clear();
        self.popup_status = None;
        self.update_suggestions();

        // Bring window to front
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    /// Handle incoming batch request from IDE extension
    pub(crate) fn on_batch_received(&mut self, req: BatchRequest, ctx: &egui::Context) {
        info!("[kyco:gui] Received batch: {} files", req.files.len(),);

        if req.files.is_empty() {
            self.logs
                .push(LogEvent::error("Batch request has no files".to_string()));
            return;
        }

        // Store batch files and open popup for mode/agent/prompt selection
        self.batch_files = req.files;
        self.view_mode = ViewMode::BatchPopup;
        self.popup_input.clear();
        self.popup_status = None;
        self.update_suggestions();

        self.logs.push(LogEvent::system(format!(
            "Batch: {} files selected, waiting for mode/prompt",
            self.batch_files.len()
        )));

        // Bring window to front
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    /// Execute batch task from batch popup
    /// Creates jobs for all batch files with the selected mode/agents/prompt
    pub(crate) fn execute_batch_task(&mut self, force_worktree: bool) {
        // Parse input same as single selection
        let (agents, mode, prompt) = parse_input_multi(&self.popup_input);

        if mode.is_empty() {
            self.popup_status = Some((
                "Please enter a mode (e.g., 'refactor', 'fix')".to_string(),
                true,
            ));
            return;
        }

        if self.batch_files.is_empty() {
            self.popup_status = Some(("No files in batch".to_string(), true));
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

        self.logs.push(LogEvent::system(format!(
            "Starting batch: {} files with agents {:?}, mode '{}'",
            self.batch_files.len(),
            resolved_agents,
            mode
        )));

        let mut total_jobs = 0;
        let mut total_groups = 0;

        // Create jobs for each file
        for file in &self.batch_files {
            // Extract workspace from batch file
            // Priority: project_root (includes git detection) > git_root > workspace
            let effective_path = file
                .project_root
                .as_ref()
                .or(file.git_root.as_ref())
                .map(|s| s.as_str())
                .unwrap_or(&file.workspace);
            let ws_path_buf = PathBuf::from(effective_path);
            let (workspace_id, workspace_path) =
                if let Ok(mut registry) = self.workspace_registry.lock() {
                    let ws_id = registry.get_or_create(ws_path_buf.clone());
                    (Some(ws_id), Some(ws_path_buf))
                } else {
                    (None, Some(ws_path_buf))
                };

            // Create SelectionContext for this file
            let selection = SelectionContext {
                app_name: Some("IDE Batch".to_string()),
                file_path: Some(file.path.clone()),
                selected_text: None,
                line_number: file.line_start,
                line_end: file.line_end,
                possible_files: Vec::new(),
                dependencies: None,
                dependency_count: None,
                additional_dependency_count: None,
                related_tests: None,
                diagnostics: None, // Batch files don't have diagnostics
                workspace_id,
                workspace_path,
            };

            // Create job(s) for this file
            if let Some(result) = jobs::create_jobs_from_selection_multi(
                &self.job_manager,
                &self.group_manager,
                &selection,
                &resolved_agents,
                &mode,
                &prompt,
                &mut self.logs,
                force_worktree,
            ) {
                total_jobs += result.job_ids.len();
                if result.group_id.is_some() {
                    total_groups += 1;
                }
            }
        }

        self.popup_status = Some((
            format!(
                "Batch complete: {} jobs created{}",
                total_jobs,
                if total_groups > 0 {
                    format!(" in {} groups", total_groups)
                } else {
                    String::new()
                }
            ),
            false,
        ));

        // Clear batch files and return to job list
        self.batch_files.clear();
        self.refresh_jobs();
        self.view_mode = ViewMode::JobList;
    }

    /// Update autocomplete suggestions based on input
    pub(crate) fn update_suggestions(&mut self) {
        let Ok(config) = self.config.read() else {
            return; // Skip autocomplete if lock poisoned
        };
        self.autocomplete
            .update_suggestions(&self.popup_input, &config);
    }

    /// Apply selected suggestion
    pub(crate) fn apply_suggestion(&mut self) {
        if let Some(new_input) = self.autocomplete.apply_suggestion(&self.popup_input) {
            self.popup_input = new_input;
        }
    }

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
