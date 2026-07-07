use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ecosystem {
    Node,
    Rust,
    Python,
    Go,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ReproducibilityVerdict {
    pub is_reproducible: bool,
    pub details: String,
}

#[derive(Debug, Clone)]
pub struct CoreReproducibilityCheck {
    pub lockfile_found: bool,
    pub lockfile_path: Option<PathBuf>,
    pub patch_dirs_found: Vec<PathBuf>,
    pub local_path_deps_found: Vec<String>,
    pub is_reproducible: bool,
    pub veto_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NukeEligibility {
    Eligible,
    Locked { reason: String },
}

pub struct SafetyGate;

impl SafetyGate {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SafetyGate {
    fn default() -> Self {
        Self::new()
    }
}

impl SafetyGate {
    /// Independently verify reproducibility for a project directory.
    pub fn verify(&self, project_root: &Path, ecosystem: &Ecosystem) -> CoreReproducibilityCheck {
        let lockfile = self.find_lockfile(project_root, ecosystem);
        let patch_dirs = self.find_patch_dirs(project_root);
        let local_deps = self.find_local_path_deps(project_root, ecosystem);

        let is_reproducible = lockfile.is_some() && patch_dirs.is_empty() && local_deps.is_empty();

        let veto_reason = if lockfile.is_none() {
            Some("No lockfile found".into())
        } else if !patch_dirs.is_empty() {
            Some(format!("Patch directories found: {:?}", patch_dirs))
        } else if !local_deps.is_empty() {
            Some(format!("Local path dependencies found: {:?}", local_deps))
        } else {
            None
        };

        CoreReproducibilityCheck {
            lockfile_found: lockfile.is_some(),
            lockfile_path: lockfile,
            patch_dirs_found: patch_dirs,
            local_path_deps_found: local_deps,
            is_reproducible,
            veto_reason,
        }
    }

    fn find_lockfile(&self, project_root: &Path, ecosystem: &Ecosystem) -> Option<PathBuf> {
        let lockfiles = match ecosystem {
            Ecosystem::Node => vec![
                "package-lock.json",
                "yarn.lock",
                "pnpm-lock.yaml",
                "bun.lockb",
            ],
            Ecosystem::Rust => vec!["Cargo.lock"],
            Ecosystem::Python => vec!["poetry.lock", "Pipfile.lock", "requirements.txt"],
            Ecosystem::Go => vec!["go.sum"],
            Ecosystem::Unknown => vec![
                "package-lock.json",
                "yarn.lock",
                "pnpm-lock.yaml",
                "bun.lockb",
                "Cargo.lock",
                "poetry.lock",
                "Pipfile.lock",
                "requirements.txt",
                "go.sum",
            ],
        };

        for lf in lockfiles {
            let path = project_root.join(lf);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    fn find_patch_dirs(&self, project_root: &Path) -> Vec<PathBuf> {
        let mut patch_dirs = Vec::new();
        let candidates = vec!["patches", ".patch-package", "node_modules/.patch-package"];
        for dir in candidates {
            let path = project_root.join(dir);
            if path.is_dir() {
                patch_dirs.push(path);
            }
        }
        patch_dirs
    }

    fn find_local_path_deps(&self, project_root: &Path, _ecosystem: &Ecosystem) -> Vec<String> {
        let mut local_deps = Vec::new();
        // Naive parser for mock/MVP. In production, this would do a shallow read
        // of package.json / Cargo.toml to detect local linkages.
        let pkg_json = project_root.join("package.json");
        if pkg_json.exists() {
            if let Ok(contents) = std::fs::read_to_string(&pkg_json) {
                if contents.contains("file:") || contents.contains("link:") {
                    local_deps.push("Local paths detected in package.json".into());
                }
            }
        }

        let cargo_toml = project_root.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(contents) = std::fs::read_to_string(&cargo_toml) {
                if contents.contains("path =") {
                    local_deps.push("Local paths detected in Cargo.toml".into());
                }
            }
        }

        local_deps
    }
}

pub fn nuke_eligible(
    plugin_verdict: Option<&ReproducibilityVerdict>,
    core_check: &CoreReproducibilityCheck,
) -> NukeEligibility {
    match (plugin_verdict, core_check.is_reproducible) {
        // Case 1: both agree it's reproducible -> eligible
        (Some(v), true) if v.is_reproducible => NukeEligibility::Eligible,
        // Case 2-4: core vetoes -> locked, regardless of plugin
        (_, false) => NukeEligibility::Locked {
            reason: core_check.veto_reason.clone().unwrap_or_default(),
        },
        // Case 5: plugin says not reproducible, core finds no issue -> conservative lock
        (Some(_), true) => NukeEligibility::Locked {
            reason: "Plugin reports non-reproducible (conservative)".into(),
        },
        // Case 6: no plugin / plugin crashed -> unknown, locked
        (None, _) => NukeEligibility::Locked {
            reason: "No plugin available — reproducibility unknown".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock check constructors for matrix testing
    fn mock_check(reproducible: bool, veto: Option<&str>) -> CoreReproducibilityCheck {
        CoreReproducibilityCheck {
            lockfile_found: true,
            lockfile_path: None,
            patch_dirs_found: vec![],
            local_path_deps_found: vec![],
            is_reproducible: reproducible,
            veto_reason: veto.map(|s| s.to_string()),
        }
    }

    fn mock_verdict(is_reproducible: bool) -> ReproducibilityVerdict {
        ReproducibilityVerdict {
            is_reproducible,
            details: "mock".into(),
        }
    }

    #[test]
    fn test_safety_gate_matrix_row_1() {
        // 1. Lockfile, no patches, plugin says true -> Nuke enabled
        let plugin = Some(mock_verdict(true));
        let core = mock_check(true, None);
        assert_eq!(
            nuke_eligible(plugin.as_ref(), &core),
            NukeEligibility::Eligible
        );
    }

    #[test]
    fn test_safety_gate_matrix_row_2_3_4() {
        // Core vetoes (No lockfile, or patch dir, or local dep)
        let plugin = Some(mock_verdict(true)); // Plugin is hallucinating/wrong
        let core = mock_check(false, Some("Core Veto"));

        match nuke_eligible(plugin.as_ref(), &core) {
            NukeEligibility::Locked { reason } => assert_eq!(reason, "Core Veto"),
            _ => panic!("Should be locked"),
        }
    }

    #[test]
    fn test_safety_gate_matrix_row_5() {
        // Core clean, Plugin false -> Conservative Lock
        let plugin = Some(mock_verdict(false));
        let core = mock_check(true, None);

        match nuke_eligible(plugin.as_ref(), &core) {
            NukeEligibility::Locked { reason } => {
                assert!(reason.contains("Plugin reports non-reproducible"))
            }
            _ => panic!("Should be locked"),
        }
    }

    #[test]
    fn test_safety_gate_matrix_row_6() {
        // No plugin available -> Locked
        let core = mock_check(true, None);
        match nuke_eligible(None, &core) {
            NukeEligibility::Locked { reason } => assert!(reason.contains("No plugin available")),
            _ => panic!("Should be locked"),
        }
    }
}
