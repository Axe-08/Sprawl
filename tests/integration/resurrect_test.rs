use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_resurrect_command_creates_kit() {
    let temp_dir = tempdir().unwrap();
    let project_dir = temp_dir.path().join("my-dead-project");
    fs::create_dir_all(&project_dir).unwrap();

    let mut cmd = Command::cargo_bin("sprawl-cli").unwrap();
    cmd.arg("resurrect")
        .arg(project_dir.to_str().unwrap());

    // Should succeed
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("resurrected successfully"));

    // Check that the kit was generated
    let kit_path = project_dir.join("resurrection-kit.md");
    assert!(kit_path.exists());
    let content = fs::read_to_string(&kit_path).unwrap();
    assert!(content.contains("Resurrection Kit: my-dead-project"));
    assert!(content.contains("Next Steps"));
}
