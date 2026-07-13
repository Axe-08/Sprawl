use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn setup_test_dir() -> TempDir {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    // Create a mock .env file with high entropy string
    fs::write(
        dir.join(".env"),
        "API_KEY=v1_abc123def456ghi789jkl012mno345pqr678stu901vwx234yz567\n",
    )
    .unwrap();

    // Create a mock config file with another string
    let src = dir.join("src");
    fs::create_dir(&src).unwrap();
    fs::write(
        src.join("config.rs"),
        "pub const SECRET: &str = \"AKIAIOSFODNN7EXAMPLE\";\n",
    )
    .unwrap();

    temp
}

#[test]
fn test_scan_finds_entropy_hits() {
    let temp = setup_test_dir();

    let mut cmd = Command::cargo_bin("sprawl-cli").unwrap();
    cmd.arg("scan").arg(temp.path());

    cmd.assert()
        .code(4)
        .stdout(predicate::str::contains("[SCAN] .env:1"))
        .stdout(predicate::str::contains("[SCAN] src/config.rs:1"));
}

#[test]
fn test_scan_exits_0_on_clean_dir() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("main.rs"), "fn main() {}").unwrap();

    let mut cmd = Command::cargo_bin("sprawl-cli").unwrap();
    cmd.arg("scan").arg(temp.path());

    cmd.assert()
        .code(0)
        .stdout(predicate::str::contains("No ambiguous secrets found."));
}

#[test]
fn test_search_returns_mock_results() {
    let mut cmd = Command::cargo_bin("sprawl-cli").unwrap();
    cmd.arg("search").arg("config");

    cmd.assert()
        .code(0)
        .stdout(predicate::str::contains("fn main() { println!(\"Hello\"); }"))
        .stdout(predicate::str::contains("0.95"));
}

#[test]
fn test_search_json_valid_structure() {
    let mut cmd = Command::cargo_bin("sprawl-cli").unwrap();
    cmd.arg("search").arg("config").arg("--json");

    let assert = cmd.assert().code(0);
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(json.get("results").unwrap().is_array());
}

#[test]
fn test_triage_list_does_not_panic() {
    let mut cmd = Command::cargo_bin("sprawl-cli").unwrap();
    cmd.arg("triage").arg("list");

    cmd.assert()
        .code(0)
        .stdout(predicate::str::contains("PROJECT"))
        .stdout(predicate::str::contains("old-api/node_modules"));
}

#[test]
fn test_triage_nuke_blocked_by_safety_gate() {
    // Create a mock project that would fail the safety gate (no lockfile)
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();

    let mut cmd = Command::cargo_bin("sprawl-cli").unwrap();
    cmd.arg("triage").arg("nuke").arg(temp.path());

    cmd.assert()
        .code(2)
        .stdout(predicate::str::contains("Safety Gate"));
}

#[test]
fn test_status_reports_daemon_not_running() {
    let mut cmd = Command::cargo_bin("sprawl-cli").unwrap();
    cmd.arg("status");

    // In a pristine test environment, the daemon won't be running
    cmd.assert()
        .code(0)
        .stdout(predicate::str::contains("Daemon:           Not running"));
}

#[test]
fn test_status_json_structure_valid() {
    let mut cmd = Command::cargo_bin("sprawl-cli").unwrap();
    cmd.arg("status").arg("--json");

    let assert = cmd.assert().code(0);
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(json.get("daemon").is_some());
    assert!(json.get("archivist_backend").is_some());
}
