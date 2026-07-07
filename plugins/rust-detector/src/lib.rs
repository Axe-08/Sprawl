use std::fs;
use std::path::Path;
use toml::Value;

wit_bindgen::generate!({
    world: "stack-detector-plugin",
    path: "../wit",
});

use exports::sprawl::stack_detector::detector::{
    Dependency, Guest, ReproducibilityVerdict, StackInfo,
};

struct RustDetector;

impl Guest for RustDetector {
    fn detect(project_root: String) -> Option<StackInfo> {
        let root = Path::new(&project_root);
        let cargo_toml_path = root.join("Cargo.toml");
        
        let content = fs::read_to_string(&cargo_toml_path).ok()?;
        let manifest: Value = toml::from_str(&content).ok()?;
        
        let mut entry_points = Vec::new();
        if root.join("src/main.rs").exists() {
            entry_points.push("src/main.rs".to_string());
        }
        if root.join("src/lib.rs").exists() {
            entry_points.push("src/lib.rs".to_string());
        }
        if let Some(workspace) = manifest.get("workspace").and_then(|v| v.as_table()) {
            if let Some(members) = workspace.get("members").and_then(|v| v.as_array()) {
                for m in members {
                    if let Some(m_str) = m.as_str() {
                        entry_points.push(format!("workspace:{}", m_str));
                    }
                }
            }
        }
        
        let mut dependencies = Vec::new();
        
        let mut parse_deps = |key: &str| {
            if let Some(deps) = manifest.get(key).and_then(|v| v.as_table()) {
                for (name, spec) in deps {
                    let mut version_spec = String::new();
                    let mut is_local_path = false;
                    
                    if let Some(v_str) = spec.as_str() {
                        version_spec = v_str.to_string();
                    } else if let Some(table) = spec.as_table() {
                        if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
                            version_spec = v.to_string();
                        }
                        if table.contains_key("path") {
                            is_local_path = true;
                            if version_spec.is_empty() {
                                version_spec = "path".to_string();
                            }
                        }
                    }
                    
                    dependencies.push(Dependency {
                        name: name.clone(),
                        version_spec,
                        is_local_path,
                    });
                }
            }
        };
        
        parse_deps("dependencies");
        parse_deps("dev-dependencies");
        
        let mut evidence = Vec::new();
        let has_lockfile = root.join("Cargo.lock").exists();
        if has_lockfile {
            evidence.push("Found Cargo.lock".to_string());
        } else {
            evidence.push("No Cargo.lock found".to_string());
        }
        
        let mut has_patch_with_local = false;
        if let Some(patch) = manifest.get("patch").and_then(|v| v.as_table()) {
            for (_, crates) in patch {
                if let Some(crates_table) = crates.as_table() {
                    for (_, spec) in crates_table {
                        if let Some(table) = spec.as_table() {
                            if table.contains_key("path") {
                                has_patch_with_local = true;
                                evidence.push("Found [patch] section with local path (veto)".to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        let is_reproducible = has_lockfile && !has_patch_with_local;
        
        Some(StackInfo {
            ecosystem: "rust".to_string(),
            entry_points,
            dependencies,
            reproducibility: ReproducibilityVerdict {
                is_reproducible,
                evidence,
            }
        })
    }
}

export!(RustDetector);
