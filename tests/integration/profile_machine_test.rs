use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_profile_machine_creates_config() {
    let temp_dir = tempdir().unwrap();
    
    // Override HOME to use temp dir for config writing
    env::set_var("HOME", temp_dir.path());

    let mut cmd = Command::cargo_bin("sprawl-cli").unwrap();
    cmd.arg("profile-machine");

    // Should succeed
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Inferred Persona:"));

    // Verify patterns.toml was created
    let config_dir = temp_dir.path().join(".sprawl").join("config");
    let patterns_path = config_dir.join("patterns.toml");
    
    assert!(patterns_path.exists());
    
    let content = fs::read_to_string(&patterns_path).unwrap();
    assert!(content.contains("[ignore]"));
    assert!(content.contains("node_modules"));
}
