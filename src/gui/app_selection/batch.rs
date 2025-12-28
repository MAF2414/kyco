//! Batch request handling for KycoApp

use crate::gui::app::KycoApp;
use crate::gui::app_types::ViewMode;
use crate::gui::http_server::BatchRequest;
use crate::gui::jobs;
use crate::gui::selection::autocomplete::parse_input_multi;
use crate::gui::selection::SelectionContext;
use crate::LogEvent;
use eframe::egui;
use std::path::PathBuf;
use tracing::info;

impl KycoApp {
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
}
