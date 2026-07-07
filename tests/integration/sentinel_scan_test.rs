use fixtures::create_secrets_fixture;
use sprawl_sentinel::classify::{classify_string, SecretClassification};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_sentinel_end_to_end_classification() {
    let dir = tempdir().unwrap();
    let project_path = dir.path().join("secrets_project");
    fs::create_dir_all(&project_path).unwrap();

    create_secrets_fixture(&project_path);

    // 1. Known Provider (sk_live_)
    let env_content = fs::read_to_string(project_path.join(".env")).unwrap();
    let env_val = env_content.split('=').nth(1).unwrap().trim();
    let result = classify_string(env_val);
    assert!(matches!(
        result.status,
        SecretClassification::KnownProvider(_)
    ));

    // 2. Negative Filter (JWT)
    let jwt_content = fs::read_to_string(project_path.join(".env.local")).unwrap();
    let jwt_val = jwt_content.split('=').nth(1).unwrap().trim();
    let result2 = classify_string(jwt_val);
    assert!(matches!(
        result2.status,
        SecretClassification::FilteredNoise(_)
    ));

    // 3. Ambiguous High Entropy
    let config_content = fs::read_to_string(project_path.join("config.yaml")).unwrap();
    let config_val = config_content
        .split(": ")
        .nth(1)
        .unwrap()
        .trim()
        .trim_matches('"');
    let result3 = classify_string(config_val);
    assert!(matches!(result3.status, SecretClassification::Ambiguous));
}
