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

struct PythonDetector;

impl Guest for PythonDetector {
    fn detect(project_root: String) -> Option<StackInfo> {
        let root = Path::new(&project_root);
        
        let has_pyproject = root.join("pyproject.toml").exists();
        let has_setup_py = root.join("setup.py").exists();
        let has_setup_cfg = root.join("setup.cfg").exists();
        let has_requirements = root.join("requirements.txt").exists();
        
        if !has_pyproject && !has_setup_py && !has_setup_cfg && !has_requirements {
            return None;
        }
        
        let mut entry_points = Vec::new();
        if root.join("main.py").exists() {
            entry_points.push("main.py".to_string());
        }
        if root.join("app.py").exists() {
            entry_points.push("app.py".to_string());
        }
        
        let mut dependencies = Vec::new();
        
        // Parse pyproject.toml
        if has_pyproject {
            if let Ok(content) = fs::read_to_string(root.join("pyproject.toml")) {
                if let Ok(manifest) = content.parse::<Value>() {
                    // Check poetry scripts
                    if let Some(tool) = manifest.get("tool").and_then(|v| v.as_table()) {
                        if let Some(poetry) = tool.get("poetry").and_then(|v| v.as_table()) {
                            if let Some(scripts) = poetry.get("scripts").and_then(|v| v.as_table()) {
                                for (name, _) in scripts {
                                    entry_points.push(format!("script:{}", name));
                                }
                            }
                            
                            // Poetry dependencies
                            let mut parse_poetry_deps = |key: &str| {
                                if let Some(deps) = poetry.get(key).and_then(|v| v.as_table()) {
                                    for (name, spec) in deps {
                                        let mut version_spec = String::new();
                                        let mut is_local_path = false;
                                        
                                        if let Some(v_str) = spec.as_str() {
                                            version_spec = v_str.to_string();
                                        } else if let Some(table) = spec.as_table() {
                                            if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
                                                version_spec = v.to_string();
                                            }
                                            if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
                                                is_local_path = true;
                                                if version_spec.is_empty() {
                                                    version_spec = format!("path:{}", path);
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
                            parse_poetry_deps("dependencies");
                            parse_poetry_deps("dev-dependencies");
                        }
                    }
                }
            }
        }
        
        // Parse requirements.txt if present (very naive parsing for MVP)
        if has_requirements {
            if let Ok(content) = fs::read_to_string(root.join("requirements.txt")) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let is_local_path = line.starts_with("./") || line.starts_with("../") || line.starts_with("file://");
                    dependencies.push(Dependency {
                        name: line.to_string(), // In real implementation we'd split on ==, >=, etc.
                        version_spec: String::new(),
                        is_local_path,
                    });
                }
            }
        }
        
        let mut evidence = Vec::new();
        let has_poetry_lock = root.join("poetry.lock").exists();
        let has_pipfile_lock = root.join("Pipfile.lock").exists();
        
        if has_poetry_lock {
            evidence.push("Found poetry.lock".to_string());
        }
        if has_pipfile_lock {
            evidence.push("Found Pipfile.lock".to_string());
        }
        if has_requirements {
            evidence.push("Found requirements.txt".to_string());
        }
        
        let is_reproducible = has_poetry_lock || has_pipfile_lock || has_requirements;
        
        if !is_reproducible {
            evidence.push("No lockfile or requirements.txt found".to_string());
        }
        
        Some(StackInfo {
            ecosystem: "python".to_string(),
            entry_points,
            dependencies,
            reproducibility: ReproducibilityVerdict {
                is_reproducible,
                evidence,
            }
        })
    }
}

export!(PythonDetector);
