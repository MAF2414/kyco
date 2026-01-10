//! Configuration view rendering methods for KycoApp
//!
//! Contains methods for rendering settings, modes, agents, and chains views.

use crate::gui::app::KycoApp;
use crate::LogEvent;
use eframe::egui;

impl KycoApp {
    /// Render settings/extensions view
    pub(crate) fn render_settings(&mut self, ctx: &egui::Context) {
        use crate::gui::settings;

        let Ok(mut config) = self.config.write() else {
            // Lock poisoned - show error and return
            self.logs.push(LogEvent::error(
                "Config lock poisoned, cannot render settings",
            ));
            return;
        };
        settings::render_settings(
            ctx,
            &mut settings::SettingsState {
                // General settings
                settings_max_concurrent: &mut self.settings_max_concurrent,
                settings_auto_run: &mut self.settings_auto_run,
                settings_use_worktree: &mut self.settings_use_worktree,
                settings_output_schema: &mut self.settings_output_schema,
                settings_structured_output_schema: &mut self.settings_structured_output_schema,
                settings_status: &mut self.settings_status,
                // Voice settings
                voice_settings_mode: &mut self.voice_settings_mode,
                voice_settings_keywords: &mut self.voice_settings_keywords,
                voice_settings_model: &mut self.voice_settings_model,
                voice_settings_language: &mut self.voice_settings_language,
                voice_settings_silence_threshold: &mut self.voice_settings_silence_threshold,
                voice_settings_silence_duration: &mut self.voice_settings_silence_duration,
                voice_settings_max_duration: &mut self.voice_settings_max_duration,
                voice_settings_global_hotkey: &mut self.voice_settings_global_hotkey,
                voice_settings_popup_hotkey: &mut self.voice_settings_popup_hotkey,
                voice_install_status: &mut self.voice_install_status,
                voice_install_in_progress: &mut self.voice_install_in_progress,
                voice_install_handle: &mut self.voice_install_handle,
                // Voice test state
                voice_test_status: &mut self.voice_test_status,
                voice_test_result: &mut self.voice_test_result,
                // VAD settings
                vad_enabled: &mut self.vad_enabled,
                vad_speech_threshold: &mut self.vad_speech_threshold,
                vad_silence_duration_ms: &mut self.vad_silence_duration_ms,
                // Voice action registry (from voice manager config)
                voice_action_registry: &self.voice_manager.config.action_registry,
                // Extension status
                extension_status: &mut self.extension_status,
                // Navigation and config
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
                // Voice config change tracking
                voice_config_changed: &mut self.voice_config_changed,
                // Shared max concurrent jobs (for runtime updates to executor)
                max_concurrent_jobs_shared: &self.max_concurrent_jobs,
                orchestrator_cli_agent: &mut self.orchestrator_cli_agent,
                orchestrator_cli_command: &mut self.orchestrator_cli_command,
                orchestrator_system_prompt: &mut self.orchestrator_system_prompt,
            },
        );
    }

    /// Render skills configuration view
    pub(crate) fn render_modes(&mut self, ctx: &egui::Context) {
        use crate::gui::skills;

        let Ok(config) = self.config.read() else {
            self.logs
                .push(LogEvent::error("Config lock poisoned, cannot render skills"));
            return;
        };
        skills::render_skills(
            ctx,
            &mut skills::SkillEditorState {
                selected_skill: &mut self.selected_mode, // Reuse existing field
                skill_edit_content: &mut self.skill_edit_content,
                skill_edit_status: &mut self.mode_edit_status, // Reuse existing field
                skill_folder_info: &mut self.skill_folder_info,
                skill_edit_name: &mut self.mode_edit_name, // Reuse for new skill name
                view_mode: &mut self.view_mode,
                config: &*config,
                work_dir: &self.work_dir,
                skills_tab: &mut self.skills_tab,
                registry_search_query: &mut self.registry_search_query,
                registry_search_results: &mut self.registry_search_results,
                registry: &mut self.skill_registry,
                registry_install_status: &mut self.registry_install_status,
                registry_install_location: &mut self.registry_install_location,
            },
        );
    }

    /// Render agents configuration view
    pub(crate) fn render_agents(&mut self, ctx: &egui::Context) {
        use crate::gui::agents;

        let Ok(mut config) = self.config.write() else {
            self.logs.push(LogEvent::error(
                "Config lock poisoned, cannot render agents",
            ));
            return;
        };
        agents::render_agents(
            ctx,
            &mut agents::AgentEditorState {
                selected_agent: &mut self.selected_agent,
                agent_edit_name: &mut self.agent_edit_name,
                agent_edit_aliases: &mut self.agent_edit_aliases,
                agent_edit_cli_type: &mut self.agent_edit_cli_type,
                agent_edit_model: &mut self.agent_edit_model,
                agent_edit_permission_mode: &mut self.agent_edit_permission_mode,
                agent_edit_sandbox: &mut self.agent_edit_sandbox,
                agent_edit_ask_for_approval: &mut self.agent_edit_ask_for_approval,
                agent_edit_mode: &mut self.agent_edit_mode,
                agent_edit_system_prompt_mode: &mut self.agent_edit_system_prompt_mode,
                agent_edit_disallowed_tools: &mut self.agent_edit_disallowed_tools,
                agent_edit_allowed_tools: &mut self.agent_edit_allowed_tools,
                agent_edit_status: &mut self.agent_edit_status,
                agent_edit_price_input: &mut self.agent_edit_price_input,
                agent_edit_price_cached_input: &mut self.agent_edit_price_cached_input,
                agent_edit_price_output: &mut self.agent_edit_price_output,
                agent_edit_allow_dangerous_bypass: &mut self.agent_edit_allow_dangerous_bypass,
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Render chains configuration view
    pub(crate) fn render_chains(&mut self, ctx: &egui::Context) {
        use crate::gui::chains;

        let Ok(mut config) = self.config.write() else {
            self.logs.push(LogEvent::error(
                "Config lock poisoned, cannot render chains",
            ));
            return;
        };
        chains::render_chains(
            ctx,
            &mut chains::ChainEditorState {
                selected_chain: &mut self.selected_chain,
                chain_edit_name: &mut self.chain_edit_name,
                chain_edit_description: &mut self.chain_edit_description,
                chain_edit_states: &mut self.chain_edit_states,
                chain_edit_steps: &mut self.chain_edit_steps,
                chain_edit_stop_on_failure: &mut self.chain_edit_stop_on_failure,
                chain_edit_pass_full_response: &mut self.chain_edit_pass_full_response,
                chain_edit_max_loops: &mut self.chain_edit_max_loops,
                chain_edit_use_worktree: &mut self.chain_edit_use_worktree,
                chain_edit_status: &mut self.chain_edit_status,
                pending_confirmation: &mut self.chain_pending_confirmation,
                view_mode: &mut self.view_mode,
                config: &mut *config,
                work_dir: &self.work_dir,
            },
        );
    }

    /// Render file search and batch selection view
    pub(crate) fn render_files(&mut self, ctx: &egui::Context) {
        use crate::gui::files;
        use crate::gui::http_server::BatchFile;
        use crate::gui::app_types::ViewMode;
        use crate::gui::theme::{BG_PRIMARY, BG_SECONDARY, TEXT_PRIMARY, TEXT_MUTED, ACCENT_CYAN, ACCENT_GREEN};

        // Track actions to perform after UI rendering
        let mut create_batch_jobs = false;
        let mut set_as_context = false;

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG_PRIMARY).inner_margin(16.0))
            .show(ctx, |ui| {
                ui.heading(egui::RichText::new("📁 File Search").color(TEXT_PRIMARY));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Search for files and create batch jobs").color(TEXT_MUTED));
                ui.add_space(16.0);

                // Search input row
                let mut do_search = false;
                ui.horizontal(|ui| {
                    // Mode selector
                    egui::ComboBox::from_id_salt("search_mode")
                        .selected_text(self.file_search.search_mode.label())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.file_search.search_mode,
                                files::SearchMode::Glob,
                                "Glob (file patterns)",
                            );
                            ui.selectable_value(
                                &mut self.file_search.search_mode,
                                files::SearchMode::Grep,
                                "Grep (content search)",
                            );
                        });

                    // Search input (full width)
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.file_search.search_query)
                            .hint_text(self.file_search.search_mode.hint())
                            .desired_width(ui.available_width() - 100.0),
                    );

                    // Search button or Enter key
                    if ui.button("🔍 Search").clicked() {
                        do_search = true;
                    }
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        do_search = true;
                    }
                });

                // Options row
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.file_search.respect_gitignore, "Respect .gitignore");
                });

                // Perform search (synchronous for now - fast for most cases)
                if do_search && !self.file_search.search_query.is_empty() {
                    self.file_search.is_searching = true;
                    let results = files::perform_search(
                        &self.file_search.search_query,
                        self.file_search.search_mode,
                        self.work_dir.clone(),
                        self.file_search.respect_gitignore,
                    );
                    self.file_search.search_results = results;
                    self.file_search.selected_files.clear();
                    self.file_search.is_searching = false;
                }

                ui.add_space(16.0);

                // Selection controls
                if !self.file_search.search_results.is_empty() {
                    ui.horizontal(|ui| {
                        if ui.button("☑ Select All").clicked() {
                            self.file_search.select_all();
                        }
                        if ui.button("☐ Deselect All").clicked() {
                            self.file_search.deselect_all();
                        }
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "{} / {} selected",
                                self.file_search.selected_count(),
                                self.file_search.search_results.len()
                            ))
                            .color(ACCENT_CYAN),
                        );
                    });
                    ui.add_space(8.0);
                }

                // Action buttons - always visible at top
                let selected_count = self.file_search.selected_count();
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(selected_count > 0, |ui| {
                        if ui.button("🚀 Create Batch Jobs").clicked() {
                            create_batch_jobs = true;
                        }
                        if ui.button("📌 Set as Context").clicked() {
                            set_as_context = true;
                        }
                    });
                    if selected_count > 0 {
                        ui.label(
                            egui::RichText::new(format!("({} files selected)", selected_count))
                                .color(ACCENT_GREEN),
                        );
                    }
                });

                ui.add_space(8.0);

                // Results area with scroll
                // Collect toggles to apply after iteration (borrow checker)
                let mut toggles: Vec<usize> = Vec::new();

                let available_width = ui.available_width();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(available_width);
                        egui::Frame::NONE
                            .fill(BG_SECONDARY)
                            .corner_radius(4.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.set_min_width(available_width - 16.0); // Account for inner margin

                                if self.file_search.is_searching {
                                    ui.label(egui::RichText::new("Searching...").color(TEXT_MUTED));
                                } else if self.file_search.search_results.is_empty() {
                                    ui.label(
                                        egui::RichText::new("No results. Enter a search pattern above.")
                                            .color(TEXT_MUTED),
                                    );
                                } else {
                                    // Render file list with checkboxes
                                    for (idx, file_match) in self.file_search.search_results.iter().enumerate() {
                                        let is_selected = self.file_search.selected_files.contains(&idx);
                                        let mut should_toggle = false;

                                        ui.horizontal(|ui| {
                                            let mut selected = is_selected;
                                            if ui.checkbox(&mut selected, "").changed() {
                                                should_toggle = true;
                                            }

                                            // File path (clickable for preview) - use full width
                                            let text = egui::RichText::new(&file_match.relative_path)
                                                .monospace()
                                                .color(if is_selected { ACCENT_GREEN } else { TEXT_PRIMARY });

                                            if ui.link(text).clicked() {
                                                // Toggle selection on click
                                                should_toggle = true;
                                            }

                                            // Show match preview for grep results
                                            if let Some(line) = file_match.match_line {
                                                ui.label(
                                                    egui::RichText::new(format!(":{}", line))
                                                        .small()
                                                        .color(TEXT_MUTED),
                                                );
                                            }
                                        });

                                        if should_toggle {
                                            toggles.push(idx);
                                        }

                                        // Show match preview
                                        if let Some(preview) = &file_match.match_preview {
                                            ui.indent(format!("preview_{}", idx), |ui| {
                                                ui.label(
                                                    egui::RichText::new(preview)
                                                        .small()
                                                        .color(TEXT_MUTED),
                                                );
                                            });
                                        }
                                    }
                                }
                            });
                    });

                // Apply toggles after iteration
                for idx in toggles {
                    self.file_search.toggle_file(idx);
                }
            });

        // Handle actions after UI rendering (outside the closure)
        if create_batch_jobs {
            // Convert selected files to BatchFile format
            let batch_files: Vec<BatchFile> = self
                .file_search
                .selected_files
                .iter()
                .filter_map(|&idx| self.file_search.search_results.get(idx))
                .map(|file_match| BatchFile {
                    path: file_match.path.display().to_string(),
                    workspace: self.work_dir.display().to_string(),
                    git_root: None,
                    project_root: Some(self.work_dir.display().to_string()),
                    line_start: file_match.match_line,
                    line_end: file_match.match_line,
                })
                .collect();

            if !batch_files.is_empty() {
                self.batch_files = batch_files;
                self.view_mode = ViewMode::BatchPopup;
                self.popup_input.clear();
                self.popup_status = None;
                self.update_suggestions();
            }
        }

        if set_as_context {
            // Store selected files as context for the next selection popup
            let selected_paths: Vec<String> = self
                .file_search
                .selected_files
                .iter()
                .filter_map(|&idx| self.file_search.search_results.get(idx))
                .map(|file_match| file_match.relative_path.clone())
                .collect();

            if !selected_paths.is_empty() {
                // Set context files (will be shown in next selection popup)
                self.selection.context_files = selected_paths;
                // Return to job list
                self.view_mode = ViewMode::JobList;
                self.logs.push(crate::LogEvent::system(format!(
                    "Set {} files as context for next job",
                    self.file_search.selected_count()
                )));
            }
        }
    }
}
