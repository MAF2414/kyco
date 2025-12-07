//! Prompt building utilities for the detail panel

use crate::config::Config;
use crate::Job;

/// Build prompt preview for a job (before it runs)
pub fn build_prompt_preview(job: &Job, config: &Config) -> String {
    // Get agent config to access mode templates
    let agent_config = config.get_agent(&job.agent_id).unwrap_or_default();
    let template = agent_config.get_mode_template(&job.mode);

    let file_path = job.source_file.display().to_string();
    let line = job.source_line;
    let description = job.description.as_deref().unwrap_or("");

    // Build the main prompt
    let ide_context = job.ide_context.as_deref().unwrap_or("");
    let prompt = template
        .prompt_template
        .replace("{file}", &file_path)
        .replace("{line}", &line.to_string())
        .replace("{target}", &job.target)
        .replace("{mode}", &job.mode)
        .replace("{description}", description)
        .replace("{scope_type}", "file")
        .replace("{ide_context}", ide_context);

    // Build system prompt if available
    let mut full_prompt = String::new();

    if let Some(system_prompt) = &template.system_prompt {
        full_prompt.push_str("=== SYSTEM PROMPT ===\n");
        full_prompt.push_str(system_prompt);
        full_prompt.push_str("\n\n");
    }

    full_prompt.push_str("=== USER PROMPT ===\n");
    full_prompt.push_str(&prompt);

    full_prompt
}
