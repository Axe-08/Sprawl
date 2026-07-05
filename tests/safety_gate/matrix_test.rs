use tempfile::tempdir;
use sprawl_sweeper::safety_gate::{SafetyGate, NukeEligibility, Ecosystem, ReproducibilityVerdict, nuke_eligible};
use fixtures::*;

// We use the fixtures exported from the `fixtures` lib via `tests/Cargo.toml`
fn mock_verdict(eligible: bool) -> ReproducibilityVerdict {
    ReproducibilityVerdict {
        is_reproducible: eligible,
        details: String::new(),
    }
}

#[test]
fn row_1_lockfile_present_no_patches_plugin_true() {
    let dir = tempdir().unwrap();
    create_node_with_lockfile(dir.path());
    let plugin = mock_verdict(true);
    let core = SafetyGate::new().verify(dir.path(), &Ecosystem::Node);
    let result = nuke_eligible(Some(&plugin), &core);
    assert!(matches!(result, NukeEligibility::Eligible));
}

#[test]
fn row_2_no_lockfile_plugin_true() {
    let dir = tempdir().unwrap();
    create_node_no_lockfile(dir.path());
    let plugin = mock_verdict(true);
    let core = SafetyGate::new().verify(dir.path(), &Ecosystem::Node);
    let result = nuke_eligible(Some(&plugin), &core);
    assert!(matches!(result, NukeEligibility::Locked { .. }));
}

#[test]
fn row_3_lockfile_with_patches_plugin_true() {
    let dir = tempdir().unwrap();
    create_node_with_patches(dir.path());
    let plugin = mock_verdict(true);
    let core = SafetyGate::new().verify(dir.path(), &Ecosystem::Node);
    let result = nuke_eligible(Some(&plugin), &core);
    assert!(matches!(result, NukeEligibility::Locked { .. }));
}

#[test]
fn row_4_lockfile_with_local_dep_plugin_true() {
    let dir = tempdir().unwrap();
    create_node_with_local_dep(dir.path());
    let plugin = mock_verdict(true);
    let core = SafetyGate::new().verify(dir.path(), &Ecosystem::Node);
    let result = nuke_eligible(Some(&plugin), &core);
    assert!(matches!(result, NukeEligibility::Locked { .. }));
}

#[test]
fn row_5_plugin_false_core_clean() {
    let dir = tempdir().unwrap();
    create_node_with_lockfile(dir.path());
    let plugin = mock_verdict(false);
    let core = SafetyGate::new().verify(dir.path(), &Ecosystem::Node);
    let result = nuke_eligible(Some(&plugin), &core);
    assert!(matches!(result, NukeEligibility::Locked { .. }));
}

#[test]
fn row_6_no_plugin() {
    let dir = tempdir().unwrap();
    create_node_with_lockfile(dir.path());
    let core = SafetyGate::new().verify(dir.path(), &Ecosystem::Node);
    let result = nuke_eligible(None, &core);
    assert!(matches!(result, NukeEligibility::Locked { .. }));
}

#[test]
fn row_7_predicate_says_nuke_gate_still_applies() {
    let dir = tempdir().unwrap();
    create_node_no_lockfile(dir.path());
    let plugin = mock_verdict(true);
    let core = SafetyGate::new().verify(dir.path(), &Ecosystem::Node);
    let result = nuke_eligible(Some(&plugin), &core);
    assert!(matches!(result, NukeEligibility::Locked { .. }));
}
