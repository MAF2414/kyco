//! eframe::App implementation for KycoApp
//!
//! Contains the main update loop that runs every frame.

use super::app::KycoApp;
use crate::LogEvent;
use eframe::egui;
use global_hotkey::GlobalHotKeyEvent;

impl eframe::App for KycoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Refresh jobs periodically (every frame for now, could optimize)
        self.refresh_jobs();

        // Periodically truncate logs (every 60 seconds)
        if self.last_log_cleanup.elapsed().as_secs() >= 60 {
            self.truncate_logs();
            self.last_log_cleanup = std::time::Instant::now();
        }

        // Poll async voice installation progress (non-blocking)
        self.poll_voice_install_progress();

        // Load inline diff when job selection changes
        if self.selected_job_id != self.prev_selected_job_id {
            self.prev_selected_job_id = self.selected_job_id;
            self.load_inline_diff_for_selected();
        }

        // Check for HTTP selection events from IDE extensions
        while let Ok(req) = self.http_rx.try_recv() {
            self.on_selection_received(req, ctx);
        }

        // Check for batch processing requests from IDE extensions
        while let Ok(req) = self.batch_rx.try_recv() {
            self.on_batch_received(req, ctx);
        }

        // Auto-run: Queue pending jobs automatically when auto_run is enabled
        self.auto_queue_pending_jobs();

        // Check for executor events (job status updates, logs)
        self.handle_executor_events(ctx);
        // Fallback: poll pending tool approvals in case we missed the stream event
        self.poll_pending_tool_approvals();

        // Process global voice hotkey events (Cmd+Shift+V / Ctrl+Shift+V)
        if self.global_hotkey_manager.is_some() {
            if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                // Any hotkey event triggers voice toggle (we only have one registered)
                if event.state == global_hotkey::HotKeyState::Pressed {
                    self.handle_global_voice_hotkey();
                }
            }
        }

        // Process voice events
        self.handle_voice_events(ctx);

        // Handle keyboard shortcuts
        let input_action = ctx.input(|i| self.process_keyboard_input(i));

        // Execute any pending action from keyboard input
        match input_action {
            super::app_input::InputAction::ExecutePopup { force_worktree } => {
                self.execute_popup_task(force_worktree);
            }
            super::app_input::InputAction::ExecuteBatch { force_worktree } => {
                self.execute_batch_task(force_worktree);
            }
            _ => {}
        }

        // Apply theme and show init banner if needed
        self.apply_theme(ctx);
        self.render_init_banner(ctx);

        // Poll update checker and handle install (needed for status bar)
        let update_info = self.poll_update_checker();
        self.handle_update_install(update_info.as_ref());

        // Poll apply/merge result if an operation is running
        self.poll_apply_result();

        // Ensure player stats are loaded for status bar
        if self.player_stats.is_none() {
            if let Some(manager) = &self.stats_manager {
                self.player_stats = manager.achievements().get_player_stats().ok();
            }
        }

        // Bottom status bar - MUST be rendered before SidePanel/CentralPanel
        // so that those panels can properly account for the status bar's height
        super::status_bar::render_status_bar(
            ctx,
            &mut super::status_bar::StatusBarState {
                auto_run: &mut self.auto_run,
                auto_allow: &mut self.auto_allow,
                view_mode: &mut self.view_mode,
                selected_mode: &mut self.selected_mode,
                mode_edit_status: &mut self.mode_edit_status,
                selected_agent: &mut self.selected_agent,
                agent_edit_status: &mut self.agent_edit_status,
                selected_chain: &mut self.selected_chain,
                chain_edit_status: &mut self.chain_edit_status,
                update_info: update_info.as_ref(),
                install_status: &mut self.update_install_status,
                orchestrator_requested: &mut self.orchestrator_requested,
                player_stats: self.player_stats.as_ref(),
            },
        );

        if self.orchestrator_requested {
            self.orchestrator_requested = false;
            if let Err(e) = self.launch_orchestrator() {
                self.logs.push(LogEvent::error(format!(
                    "Failed to start orchestrator: {}",
                    e
                )));
            }
        }

        // Render based on view mode
        self.render_view_mode(ctx);

        // Render permission popup on top of everything if visible
        self.render_permission_popup_modal(ctx);

        // Render gamification toast notifications (achievements, level-ups, etc.)
        self.render_toast(ctx);

        // Render global voice overlay (small indicator when recording via hotkey)
        if self.show_voice_overlay {
            self.render_voice_overlay(ctx);
        }

        // Request continuous updates
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
