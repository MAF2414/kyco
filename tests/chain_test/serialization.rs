//! Serialization format tests for chain configuration

use kyco::config::{ChainStep, Config, ModeChain};

#[test]
fn test_chain_yaml_serialization() {
    let chain = ModeChain {
        version: 0,
        description: Some("YAML test".to_string()),
        steps: vec![ChainStep {
            mode: "review".to_string(),
            trigger_on: Some(vec!["ready".to_string()]),
            skip_on: None,
            agent: None,
            inject_context: None,
        }],
        stop_on_failure: true,
        states: vec![],
        pass_full_response: true,
        use_worktree: None,
    };

    let yaml_str = serde_yaml::to_string(&chain).expect("Failed to serialize to YAML");
    let deserialized: ModeChain =
        serde_yaml::from_str(&yaml_str).expect("Failed to deserialize from YAML");

    assert_eq!(deserialized.description, chain.description);
    assert_eq!(deserialized.steps.len(), 1);
}

#[test]
fn test_full_config_with_chain_toml_roundtrip() {
    let mut config = Config::with_defaults();

    config.chain.insert(
        "test_chain".to_string(),
        ModeChain {
            version: 0,
            description: Some("Test".to_string()),
            steps: vec![ChainStep {
                mode: "review".to_string(),
                trigger_on: None,
                skip_on: None,
                agent: Some("claude".to_string()),
                inject_context: None,
            }],
            stop_on_failure: false,
            states: vec![],
            pass_full_response: true,
            use_worktree: None,
        },
    );

    let toml_str = toml::to_string(&config).expect("Failed to serialize config to TOML");
    let deserialized: Config =
        toml::from_str(&toml_str).expect("Failed to deserialize config from TOML");

    assert!(deserialized.get_chain("test_chain").is_some());
    let chain = deserialized.get_chain("test_chain").unwrap();
    assert_eq!(chain.description, Some("Test".to_string()));
}
