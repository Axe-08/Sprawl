use std::fs;
use std::path::Path;

wit_bindgen::generate!({
    world: "stack-detector-plugin",
    path: "../wit",
});

use exports::sprawl::stack_detector::detector::{
    Dependency, Guest, ReproducibilityVerdict, StackInfo,
};

struct GoDetector;

impl Guest for GoDetector {
    fn detect(project_root: String) -> Option<StackInfo> {
        let root = Path::new(&project_root);
        let go_mod_path = root.join("go.mod");
        
        if !go_mod_path.exists() {
            return None;
        }
        
        let mut entry_points = Vec::new();
        if root.join("main.go").exists() {
            entry_points.push("main.go".to_string());
        }
        
        // Basic check for cmd/*/main.go
        if let Ok(entries) = fs::read_dir(root.join("cmd")) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_dir() {
                        let main_go = entry.path().join("main.go");
                        if main_go.exists() {
                            if let Some(path_str) = main_go.strip_prefix(root).ok().and_then(|p| p.to_str()) {
                                entry_points.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        let mut dependencies = Vec::new();
        
        if let Ok(content) = fs::read_to_string(&go_mod_path) {
            let mut in_require = false;
            let mut in_replace = false;
            
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with("//") {
                    continue;
                }
                
                if line.starts_with("require (") {
                    in_require = true;
                    continue;
                } else if line.starts_with("replace (") {
                    in_replace = true;
                    continue;
                } else if line == ")" {
                    in_require = false;
                    in_replace = false;
                    continue;
                }
                
                if line.starts_with("require ") || in_require {
                    let parts: Vec<&str> = line.strip_prefix("require ").unwrap_or(line).split_whitespace().collect();
                    if parts.len() >= 2 {
                        dependencies.push(Dependency {
                            name: parts[0].to_string(),
                            version_spec: parts[1].to_string(),
                            is_local_path: false,
                        });
                    }
                }
                
                if line.starts_with("replace ") || in_replace {
                    let parts: Vec<&str> = line.strip_prefix("replace ").unwrap_or(line).split_whitespace().collect();
                    if parts.len() >= 3 && parts[parts.len() - 2] == "=>" {
                        let target = parts[parts.len() - 1];
                        if target.starts_with("./") || target.starts_with("../") || target.starts_with('/') {
                            // Update dependency if it was already added
                            let name = parts[0];
                            if let Some(dep) = dependencies.iter_mut().find(|d| d.name == name) {
                                dep.is_local_path = true;
                            } else {
                                dependencies.push(Dependency {
                                    name: name.to_string(),
                                    version_spec: target.to_string(),
                                    is_local_path: true,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        let mut evidence = Vec::new();
        let has_go_sum = root.join("go.sum").exists();
        
        if has_go_sum {
            evidence.push("Found go.sum".to_string());
        } else {
            evidence.push("No go.sum found".to_string());
        }
        
        let has_local_replace = dependencies.iter().any(|d| d.is_local_path);
        if has_local_replace {
            evidence.push("Found local replace directive (veto)".to_string());
        }
        
        let is_reproducible = has_go_sum && !has_local_replace;
        
        Some(StackInfo {
            ecosystem: "go".to_string(),
            entry_points,
            dependencies,
            reproducibility: ReproducibilityVerdict {
                is_reproducible,
                evidence,
            }
        })
    }
}

export!(GoDetector);
