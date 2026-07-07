use sprawl_core::config::LayeredConfig;

#[test]
fn test_global_defaults_load_without_persona() {
    let mut config = LayeredConfig::new();
    config.load_global_defaults().unwrap();

    assert!(!config.rules.is_empty(), "Rules should not be empty");
    let has_snooze = config.rules.iter().any(|r| r.name == "Global Snooze Baseline");
    assert!(has_snooze, "Should contain the Global Snooze Baseline rule");
}

#[test]
fn test_web_dev_persona_adds_node_modules_rule() {
    let mut config = LayeredConfig::new();
    config.load_global_defaults().unwrap();
    config.load_persona("web-dev").unwrap();

    let node_modules_rule = config.rules.iter().find(|r| r.name == "Nuke node_modules");
    assert!(node_modules_rule.is_some(), "web-dev persona must add node_modules rule");
    let rule = node_modules_rule.unwrap();
    assert_eq!(rule.action, "nuke_safe");
}

#[test]
fn test_persona_override_tracks_source_field() {
    let mut config = LayeredConfig::new();
    config.load_global_defaults().unwrap();
    config.load_persona("web-dev").unwrap();

    // In a full implementation, if a rule overrides another, it tracks the source.
    // For now, we just ensure the source is set to the persona name.
    let node_modules_rule = config.rules.iter().find(|r| r.name == "Nuke node_modules").unwrap();
    assert_eq!(node_modules_rule.source, "web-dev");
}

#[test]
fn test_unknown_persona_errors() {
    let mut config = LayeredConfig::new();
    let result = config.load_persona("unknown-hacker-persona");
    
    assert!(result.is_err(), "Loading an unknown persona should return an error");
    assert!(result.unwrap_err().to_string().contains("Unknown persona"));
}

#[test]
fn test_noise_patterns_loaded_from_defaults() {
    let mut config = LayeredConfig::new();
    config.load_global_defaults().unwrap();

    assert!(!config.noise_patterns.is_empty(), "Noise patterns should be loaded");
    
    // Test that common noise directories are included
    let has_mock = config.noise_patterns.iter().any(|n| n.pattern == "mock");
    
    assert!(has_mock, "Noise patterns should contain mock");
}
