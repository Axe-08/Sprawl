use std::fs;
use std::path::Path;

pub fn create_node_with_lockfile(root: &Path) {
    fs::write(
        root.join("package.json"),
        r#"{"dependencies":{"express":"^4.0.0"}}"#,
    )
    .unwrap();
    fs::write(root.join("package-lock.json"), "{}").unwrap();
    fs::create_dir_all(root.join("node_modules/express")).unwrap();
}

pub fn create_node_no_lockfile(root: &Path) {
    fs::write(
        root.join("package.json"),
        r#"{"dependencies":{"express":"^4.0.0"}}"#,
    )
    .unwrap();
    fs::create_dir_all(root.join("node_modules/express")).unwrap();
}

pub fn create_node_with_patches(root: &Path) {
    fs::write(
        root.join("package.json"),
        r#"{"dependencies":{"express":"^4.0.0"}}"#,
    )
    .unwrap();
    fs::write(root.join("package-lock.json"), "{}").unwrap();
    fs::create_dir_all(root.join("node_modules/express")).unwrap();
    fs::create_dir_all(root.join("patches")).unwrap();
    fs::write(root.join("patches/express.patch"), "dummy patch data").unwrap();
}

pub fn create_node_with_local_dep(root: &Path) {
    fs::write(
        root.join("package.json"),
        r#"{"dependencies":{"my-lib":"file:../my-lib"}}"#,
    )
    .unwrap();
    fs::write(root.join("package-lock.json"), "{}").unwrap();
    fs::create_dir_all(root.join("node_modules/my-lib")).unwrap();
}

pub fn create_secrets_fixture(root: &Path) {
    fs::write(
        root.join(".env"),
        format!(
            "AWS_SECRET_ACCESS_KEY=sk_live_{}\n",
            "1234567890abcdef1234567890"
        ),
    )
    .unwrap();
    fs::write(
        root.join(".env.local"),
        "JWT=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.ey.dummy\n",
    )
    .unwrap();
    fs::write(
        root.join("config.yaml"),
        "db_password: \"super_complex_high_entropy_password_123!!\"\n",
    )
    .unwrap();
}
