//! Selection handling methods for KycoApp
//!
//! Contains IDE selection/batch request handling and popup task execution.

mod batch;
mod popup_task;

use super::app::KycoApp;
use super::app_types::ViewMode;
use super::http_server::SelectionRequest;
use super::selection::SelectionContext;
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

        // Determine workspace path for SDK cwd resolution
        // Priority: project_root > git_root > workspace > file's directory
        let workspace_path = req
            .project_root
            .as_ref()
            .or(req.git_root.as_ref())
            .or(req.workspace.as_ref())
            .map(PathBuf::from)
            .or_else(|| req.file_path.as_ref().and_then(|f| {
                PathBuf::from(f).parent().map(PathBuf::from)
            }));

        self.selection = SelectionContext {
            app_name: Some("IDE".to_string()),
            file_path: req.file_path,
            selected_text: req.selected_text,
            line_number: req.line_start,
            line_end: req.line_end,
            possible_files: Vec::new(),
            context_files: Vec::new(),
            dependencies: req.dependencies,
            dependency_count: req.dependency_count,
            additional_dependency_count: req.additional_dependency_count,
            related_tests: req.related_tests,
            diagnostics: req.diagnostics,
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
