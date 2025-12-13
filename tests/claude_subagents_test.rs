use kyco::agent::bridge::ClaudeQueryRequest;
use kyco::config::Config;

#[test]
fn claude_subagents_flow_from_config_to_bridge_json() {
    let toml_config = r#"
[agent.claude]
aliases = ["c"]
sdk = "claude"
session_mode = "oneshot"
system_prompt_mode = "append"

[agent.claude.agents.code-reviewer]
description = "Reviews code for bugs and style issues"
prompt = "You are a strict code reviewer."
tools = ["Read", "Grep", "Glob"]
disallowed_tools = ["Bash"]
model = "sonnet"
"#;

    let config: Config = toml::from_str(toml_config).expect("Failed to parse config");
    let agent = config.get_agent("claude").expect("Missing claude agent");

    let subagent = agent
        .agents
        .get("code-reviewer")
        .expect("Missing code-reviewer subagent");
    assert_eq!(
        subagent.description,
        "Reviews code for bugs and style issues"
    );
    assert_eq!(subagent.prompt, "You are a strict code reviewer.");
    assert_eq!(
        subagent.tools.as_ref().expect("Missing tools"),
        &vec!["Read".to_string(), "Grep".to_string(), "Glob".to_string()]
    );
    assert_eq!(
        subagent
            .disallowed_tools
            .as_ref()
            .expect("Missing disallowed_tools"),
        &vec!["Bash".to_string()]
    );
    assert_eq!(subagent.model.as_deref(), Some("sonnet"));

    let request = ClaudeQueryRequest {
        prompt: "hello".to_string(),
        cwd: "/tmp".to_string(),
        session_id: None,
        fork_session: None,
        permission_mode: None,
        images: None,
        agents: Some(agent.agents.clone()),
        allowed_tools: None,
        disallowed_tools: None,
        env: None,
        mcp_servers: None,
        system_prompt: None,
        system_prompt_mode: None,
        setting_sources: None,
        plugins: None,
        max_turns: None,
        max_thinking_tokens: None,
        model: None,
        output_schema: None,
        kyco_callback_url: None,
        hooks: None,
    };

    let json = serde_json::to_value(&request).expect("Failed to serialize request");
    assert_eq!(
        json["agents"]["code-reviewer"]["disallowedTools"][0],
        "Bash"
    );
}
