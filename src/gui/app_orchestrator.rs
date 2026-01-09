//! Orchestrator functionality for KycoApp
//!
//! Contains the orchestrator launch logic.

use super::app::KycoApp;
use crate::LogEvent;
use crate::agent::TerminalSession;
use crate::config::default_orchestrator_system_prompt;
use std::time::{SystemTime, UNIX_EPOCH};

impl KycoApp {
    /// Launch the orchestrator in a new Terminal.app window
    pub(crate) fn launch_orchestrator(&mut self) -> anyhow::Result<()> {
        #[cfg(not(target_os = "macos"))]
        {
            anyhow::bail!("Orchestrator launch is only supported on macOS right now.");
        }

        #[cfg(target_os = "macos")]
        {
            let kyco_dir = self.work_dir.join(".kyco");
            std::fs::create_dir_all(&kyco_dir)?;

            // Get orchestrator settings from config
            let (cli_agent, custom_cli, system_prompt) = self
                .config
                .read()
                .map(|cfg| {
                    let orch = &cfg.settings.gui.orchestrator;
                    let prompt = if orch.system_prompt.trim().is_empty() {
                        // Use default if config value is empty (common after config migration)
                        default_orchestrator_system_prompt()
                    } else {
                        orch.system_prompt.clone()
                    };
                    (
                        orch.cli_agent.trim().to_lowercase(),
                        orch.cli_command.trim().to_string(),
                        prompt,
                    )
                })
                .unwrap_or_else(|_| {
                    (
                        "claude".to_string(),
                        String::new(),
                        default_orchestrator_system_prompt(),
                    )
                });

            // Write the system prompt to file
            let prompt_file = kyco_dir.join("orchestrator_system_prompt.txt");
            std::fs::write(&prompt_file, &system_prompt)?;

            // Use custom CLI command or generate default based on cli_agent
            let (command, agent_name) = if !custom_cli.is_empty() {
                // Replace {prompt_file} placeholder with actual path
                let cmd = custom_cli.replace("{prompt_file}", ".kyco/orchestrator_system_prompt.txt");
                (cmd, "custom".to_string())
            } else {
                let agent = if cli_agent.is_empty() {
                    "claude"
                } else {
                    cli_agent.as_str()
                };
                let cmd = match agent {
                    "codex" => {
                        "codex \"$(cat .kyco/orchestrator_system_prompt.txt)\"".to_string()
                    }
                    _ => {
                        "claude --append-system-prompt \"$(cat .kyco/orchestrator_system_prompt.txt)\""
                            .to_string()
                    }
                };
                (cmd, agent.to_string())
            };

            let session_id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let args = vec!["-lc".to_string(), command.clone()];
            TerminalSession::spawn(session_id, "bash", &args, "", &self.work_dir)?;

            self.logs.push(LogEvent::system(format!(
                "Orchestrator started in Terminal.app ({})",
                agent_name
            )));
            Ok(())
        }
    }
}
