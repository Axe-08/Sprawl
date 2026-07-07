use sprawl_archaeologist::Archaeologist;
use sprawl_plugin_host::{PluginHost, PluginRegistry};
use std::fs;
use std::path::{Path, PathBuf};

async fn setup_archaeologist() -> Archaeologist {
    let host = PluginHost::new().expect("Failed to initialize plugin host");
    let mut registry = PluginRegistry::new();

    let plugin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/target/wasm32-wasip1/release");

    // Load Rust detector
    let rust_wasm = plugin_dir.join("rust_detector.wasm");
    if rust_wasm.exists() {
        let plugin = host.load_plugin(&rust_wasm, "rust-detector").expect("Failed to load rust-detector");
        registry.register(plugin);
    }

    // Load Node detector
    let node_wasm = plugin_dir.join("node_detector.wasm");
    if node_wasm.exists() {
        let plugin = host.load_plugin(&node_wasm, "node-detector").expect("Failed to load node-detector");
        registry.register(plugin);
    }
    
    // Load Python detector
    let python_wasm = plugin_dir.join("python_detector.wasm");
    if python_wasm.exists() {
        let plugin = host.load_plugin(&python_wasm, "python-detector").expect("Failed to load python-detector");
        registry.register(plugin);
    }
    
    // Load Go detector
    let go_wasm = plugin_dir.join("go_detector.wasm");
    if go_wasm.exists() {
        let plugin = host.load_plugin(&go_wasm, "go-detector").expect("Failed to load go-detector");
        registry.register(plugin);
    }

    Archaeologist::new(host, registry)
}

#[tokio::test]
async fn test_rust_detector_integration() {
    let arch = setup_archaeologist().await;
    
    // Create a mock rust project
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
    
    let (primary, matches) = arch.detect_stack(project_root).await.unwrap();
    
    assert!(primary.is_some(), "Should detect stack");
    let info = primary.unwrap();
    assert_eq!(info.ecosystem, "rust");
    assert!(info.entry_points.contains(&"src/main.rs".to_string()));
    assert_eq!(info.dependencies.len(), 1);
    assert_eq!(info.dependencies[0].name, "serde");
    assert!(info.reproducibility.is_reproducible);
}

#[tokio::test]
async fn test_node_detector_integration() {
    let arch = setup_archaeologist().await;
    
    // Create a mock node project
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
    
    let (primary, matches) = arch.detect_stack(project_root).await.unwrap();
    
    assert!(primary.is_some(), "Should detect node stack");
    let info = primary.unwrap();
    assert_eq!(info.ecosystem, "node");
    assert!(info.entry_points.contains(&"index.js".to_string()));
    assert_eq!(info.dependencies.len(), 1);
    assert_eq!(info.dependencies[0].name, "express");
    assert!(info.reproducibility.is_reproducible);
}
