//! Selection handling methods for KycoApp
//!
//! Contains IDE selection/batch request handling and popup task execution.

mod batch;
mod popup_task;

use super::app::KycoApp;
use super::app_types::ViewMode;
use super::http_server::SelectionRequest;
use super::selection::SelectionContext;
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
}
