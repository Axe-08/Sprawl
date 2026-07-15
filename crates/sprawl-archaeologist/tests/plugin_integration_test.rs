/// Plugin integration tests require pre-built WASM detector plugins.
/// Run `./build_plugins.sh` first to compile them.
/// In CI, these tests are gated by the presence of the WASM files —
/// they are skipped (not failed) when the plugins are not found.
use sprawl_archaeologist::Archaeologist;
use sprawl_plugin_host::{PluginHost, PluginRegistry};
use std::fs;
use std::path::PathBuf;

fn plugin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/target/wasm32-wasip1/release")
}

fn plugins_available() -> bool {
    let dir = plugin_dir();
    ["rust_detector.wasm", "node_detector.wasm"]
        .iter()
        .any(|f| dir.join(f).exists())
}

async fn setup_archaeologist() -> Archaeologist {
    let host = PluginHost::new(true, None).expect("Failed to initialize plugin host");
    let mut registry = PluginRegistry::new();

    let dir = plugin_dir();

    for (file, name) in &[
        ("rust_detector.wasm", "rust-detector"),
        ("node_detector.wasm", "node-detector"),
        ("python_detector.wasm", "python-detector"),
        ("go_detector.wasm", "go-detector"),
    ] {
        let path = dir.join(file);
        if path.exists() {
            let plugin = host
                .load_plugin(&path, name, None)
                .expect("Failed to load plugin");
            registry.register(plugin);
        }
    }

    Archaeologist::new(host, registry)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_rust_detector_integration() {
    if !plugins_available() {
        eprintln!(
            "SKIP test_rust_detector_integration: WASM plugins not built. Run ./build_plugins.sh"
        );
        return;
    }

    let arch = setup_archaeologist().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let project_root = temp_dir.path();

    let cargo_toml = r#"
    [package]
    name = "test-pkg"
    version = "0.1.0"
    
    [dependencies]
    serde = "1.0"
    "#;

    fs::write(project_root.join("Cargo.toml"), cargo_toml).unwrap();
    fs::write(project_root.join("Cargo.lock"), "").unwrap(); // Mark as reproducible
    fs::create_dir_all(project_root.join("src")).unwrap();
    fs::write(project_root.join("src/main.rs"), "fn main() {}").unwrap();

    let (primary, _matches) = arch.detect_stack(project_root).await.unwrap();

    assert!(primary.is_some(), "Should detect rust stack");
    let info = primary.unwrap();
    assert_eq!(info.ecosystem, "rust");
    assert!(info.entry_points.contains(&"src/main.rs".to_string()));
    assert_eq!(info.dependencies.len(), 1);
    assert_eq!(info.dependencies[0].name, "serde");
    assert!(info.reproducibility.is_reproducible);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_node_detector_integration() {
    if !plugins_available() {
        eprintln!(
            "SKIP test_node_detector_integration: WASM plugins not built. Run ./build_plugins.sh"
        );
        return;
    }

    let arch = setup_archaeologist().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let project_root = temp_dir.path();

    let package_json = r#"{
        "name": "test-node",
        "main": "index.js",
        "dependencies": {
            "express": "^4.17.1"
        }
    }"#;

    fs::write(project_root.join("package.json"), package_json).unwrap();
    fs::write(project_root.join("package-lock.json"), "").unwrap(); // Mark as reproducible

    let (primary, _matches) = arch.detect_stack(project_root).await.unwrap();

    assert!(primary.is_some(), "Should detect node stack");
    let info = primary.unwrap();
    assert_eq!(info.ecosystem, "node");
    assert!(info.entry_points.contains(&"index.js".to_string()));
    assert_eq!(info.dependencies.len(), 1);
    assert_eq!(info.dependencies[0].name, "express");
    assert!(info.reproducibility.is_reproducible);
}
