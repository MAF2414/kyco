//! Default mode templates for agent configuration.

use std::collections::HashMap;

use super::{ModeTemplate, SessionMode};

/// Build the default mode templates
pub fn default_mode_templates() -> HashMap<String, ModeTemplate> {
    let mut templates = HashMap::new();

    templates.insert(
        "refactor".to_string(),
        ModeTemplate {
            prompt_template: "Refactor the {scope_type} `{target}` in `{file}`. {description}"
                .to_string(),
            system_prompt: Some(
                "You are running in KYCo 'refactor' mode. You may read the entire repo. \
                 Make code changes only within the marked scope. Write idiomatic code. \
                 Do not change function signatures unless explicitly requested."
                    .to_string(),
            ),
            default_agent: None,
            session_mode: SessionMode::Oneshot,
            disallowed_tools: vec![],
            allowed_tools: vec![],
            output_states: vec![],
            state_prompt: None,
        },
    );

    templates.insert(
        "fix".to_string(),
        ModeTemplate {
            prompt_template: "Fix the issue in {scope_type} `{target}` in `{file}`. {description}"
                .to_string(),
            system_prompt: Some(
                "You are running in KYCo 'fix' mode. You may read the entire repo. \
                 Analyze the code and fix the described issue. Make minimal changes necessary."
                    .to_string(),
            ),
            default_agent: None,
            session_mode: SessionMode::Oneshot,
            disallowed_tools: vec![],
            allowed_tools: vec![],
            output_states: vec![],
            state_prompt: None,
        },
    );

    templates.insert(
        "tests".to_string(),
        ModeTemplate {
            prompt_template:
                "Write unit tests for {scope_type} `{target}` in `{file}`. {description}"
                    .to_string(),
            system_prompt: Some(
                "You are running in KYCo 'tests' mode. You may read the entire repo. \
                 Write comprehensive unit tests. Use the existing test framework and patterns \
                 found in the codebase."
                    .to_string(),
            ),
            default_agent: None,
            session_mode: SessionMode::Oneshot,
            disallowed_tools: vec![],
            allowed_tools: vec![],
            output_states: vec![],
            state_prompt: None,
        },
    );

    templates.insert(
        "docs".to_string(),
        ModeTemplate {
            prompt_template:
                "Write documentation for {scope_type} `{target}` in `{file}`. {description}"
                    .to_string(),
            system_prompt: Some(
                "You are running in KYCo 'docs' mode. You may read the entire repo. \
                 Write clear, concise documentation. Follow existing documentation patterns \
                 in the codebase."
                    .to_string(),
            ),
            default_agent: None,
            session_mode: SessionMode::Oneshot,
            disallowed_tools: vec![],
            allowed_tools: vec![],
            output_states: vec![],
            state_prompt: None,
        },
    );

    templates.insert(
        "review".to_string(),
        ModeTemplate {
            prompt_template: "Review {scope_type} `{target}` in `{file}`. {description}"
                .to_string(),
            system_prompt: Some(
                "You are running in KYCo 'review' mode. You may read the entire repo. \
                 Analyze the code for bugs, performance issues, and code quality. \
                 Suggest improvements but do not make changes unless explicitly asked."
                    .to_string(),
            ),
            default_agent: None,
            session_mode: SessionMode::Oneshot,
            disallowed_tools: vec![],
            allowed_tools: vec![],
            output_states: vec![],
            state_prompt: None,
        },
    );

    templates.insert(
        "chat".to_string(),
        ModeTemplate {
            prompt_template: "{description}".to_string(),
            system_prompt: Some(
                "You are running in KYCo 'chat' mode. This is a conversational session. \
                 You can continue the conversation and remember previous context."
                    .to_string(),
            ),
            default_agent: None,
            session_mode: SessionMode::Session, // Chat mode uses sessions by default
            disallowed_tools: vec![],
            allowed_tools: vec![],
            output_states: vec![],
            state_prompt: None,
        },
    );

    templates
}
