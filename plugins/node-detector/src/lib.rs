use std::fs;
use std::path::Path;
use serde_json::Value;

wit_bindgen::generate!({
    world: "stack-detector-plugin",
    path: "../wit",
});

use exports::sprawl::stack_detector::detector::{
    Dependency, Guest, ReproducibilityVerdict, StackInfo,
};

struct NodeDetector;

impl Guest for NodeDetector {
    fn detect(project_root: String) -> Option<StackInfo> {
        let root = Path::new(&project_root);
        let pkg_json_path = root.join("package.json");
        
        let content = fs::read_to_string(&pkg_json_path).ok()?;
        let pkg: Value = serde_json::from_str(&content).ok()?;
        
        let mut entry_points = Vec::new();
        if let Some(main) = pkg.get("main").and_then(|v| v.as_str()) {
            entry_points.push(main.to_string());
        }
        if let Some(scripts) = pkg.get("scripts").and_then(|v| v.as_object()) {
            if scripts.contains_key("start") {
                entry_points.push("scripts.start".to_string());
            }
            if scripts.contains_key("dev") {
                entry_points.push("scripts.dev".to_string());
            }
        }
        
        let mut dependencies = Vec::new();
        let mut parse_deps = |key: &str| {
            if let Some(deps) = pkg.get(key).and_then(|v| v.as_object()) {
                for (name, version) in deps {
                    if let Some(v_str) = version.as_str() {
                        let is_local_path = v_str.starts_with("file:") 
                            || v_str.starts_with("link:") 
                            || v_str.starts_with("./") 
                            || v_str.starts_with("../");
                            
                        dependencies.push(Dependency {
                            name: name.clone(),
                            version_spec: v_str.to_string(),
                            is_local_path,
                        });
                    }
                }
            }
        };
        parse_deps("dependencies");
        parse_deps("devDependencies");
        
        let lockfiles = ["package-lock.json", "yarn.lock", "pnpm-lock.yaml", "bun.lockb"];
        let mut has_lockfile = false;
        let mut evidence = Vec::new();
        
        for lockfile in lockfiles {
            if root.join(lockfile).exists() {
                has_lockfile = true;
                evidence.push(format!("Found {}", lockfile));
            }
        }
        
        let mut has_patch_dir = false;
        for patch_dir in ["patches", ".patch-package"] {
            if root.join(patch_dir).exists() {
                has_patch_dir = true;
                evidence.push(format!("Found {}/ directory (veto)", patch_dir));
            }
        }
        
        if !has_lockfile {
            evidence.push("No lockfile found".to_string());
        }
        
        let is_reproducible = has_lockfile && !has_patch_dir;
        
        Some(StackInfo {
            ecosystem: "node".to_string(),
            entry_points,
            dependencies,
            reproducibility: ReproducibilityVerdict {
                is_reproducible,
                evidence,
            }
        })
    }
}

export!(NodeDetector);
