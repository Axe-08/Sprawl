use crate::safety_gate::{Ecosystem, NukeEligibility, ReproducibilityVerdict, SafetyGate};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

// Stubs for cross-crate dependencies
pub struct ProjectId(pub String);
pub struct ManifestEntry;
pub struct ResolvedConfig;
pub struct Archaeologist;
pub struct Manifest;
pub struct LedgerConnection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriageAction {
    Snooze,
    Archive,
    NukeSafe,
}

pub struct TriageItem {
    pub project_id: ProjectId,
    pub project_root: PathBuf,
    pub target_path: PathBuf,
    pub matched_pattern: String,
    pub size_bytes: u64,
    pub idle_days: i64,
    pub nuke_eligibility: NukeEligibility,
    pub recommended_action: TriageAction,
}

pub struct SweeperEngine {
    _config: ResolvedConfig,
    safety_gate: SafetyGate,
    _archaeologist: Archaeologist,
    _manifest: Manifest,
    _ledger: LedgerConnection,
}

impl SweeperEngine {
    pub fn new() -> Self {
        Self {
            _config: ResolvedConfig,
            safety_gate: SafetyGate::new(),
            _archaeologist: Archaeologist,
            _manifest: Manifest,
            _ledger: LedgerConnection,
        }
    }
}

impl Default for SweeperEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SweeperEngine {
    pub fn snooze(&self, _item: &TriageItem, _days: u32) -> Result<()> {
        // Update ledger to ignore for N days
        Ok(())
    }

    pub fn archive(&self, item: &TriageItem, destination: &Path) -> Result<()> {
        if !item.target_path.exists() {
            return Ok(());
        }

        if !destination.exists() {
            fs::create_dir_all(destination)?;
        }

        let target_name = item.target_path.file_name().unwrap();
        let archive_path = destination.join(target_name);

        // Rename (atomic on same filesystem)
        if let Err(_e) = fs::rename(&item.target_path, &archive_path) {
            // Fallback for cross-filesystem move would go here
            return Err(anyhow::anyhow!("Move failed"));
        }

        // Create Symlink
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&archive_path, &item.target_path)?;
        }
        #[cfg(windows)]
        {
            if archive_path.is_dir() {
                std::os::windows::fs::symlink_dir(&archive_path, &item.target_path)?;
            } else {
                std::os::windows::fs::symlink_file(&archive_path, &item.target_path)?;
            }
        }

        Ok(())
    }

    pub fn nuke(
        &self,
        item: &TriageItem,
        plugin_verdict: Option<&ReproducibilityVerdict>,
    ) -> Result<()> {
        let core_check = self
            .safety_gate
            .verify(&item.project_root, &Ecosystem::Unknown);
        let exec_eligibility = crate::safety_gate::nuke_eligible(plugin_verdict, &core_check);

        match exec_eligibility {
            NukeEligibility::Eligible => {
                if item.target_path.exists() {
                    if item.target_path.is_dir() {
                        fs::remove_dir_all(&item.target_path)?;
                    } else {
                        fs::remove_file(&item.target_path)?;
                    }
                }
                Ok(())
            }
            NukeEligibility::Locked { reason } => {
                Err(anyhow::anyhow!("Safety Gate veto: {}", reason))
            }
        }
    }

    pub fn restore(&self, target_path: &Path, archive_path: &Path) -> Result<()> {
        if target_path.symlink_metadata().is_ok() {
            #[cfg(windows)]
            {
                if let Err(_) = fs::remove_file(target_path) {
                    fs::remove_dir(target_path)?;
                }
            }
            #[cfg(not(windows))]
            {
                fs::remove_file(target_path)?;
            }
        }
        if archive_path.exists() {
            fs::rename(archive_path, target_path)?; // Move back
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_mock_project() -> (TempDir, PathBuf, PathBuf) {
        let dir = TempDir::new().unwrap();
        let project_root = dir.path().to_path_buf();
        let target_path = project_root.join("node_modules");
        fs::create_dir(&target_path).unwrap();

        let lockfile = project_root.join("package-lock.json");
        fs::write(&lockfile, "mock lockfile").unwrap();

        (dir, project_root, target_path)
    }

    #[test]
    fn test_nuke_succeeds_when_eligible() {
        let (_dir, root, target) = setup_mock_project();
        let engine = SweeperEngine::new();

        let item = TriageItem {
            project_id: ProjectId("1".into()),
            project_root: root,
            target_path: target.clone(),
            matched_pattern: "node_modules".into(),
            size_bytes: 100,
            idle_days: 20,
            nuke_eligibility: NukeEligibility::Eligible,
            recommended_action: TriageAction::NukeSafe,
        };

        let verdict = ReproducibilityVerdict {
            is_reproducible: true,
            details: "".into(),
        };

        assert!(target.exists());
        assert!(engine.nuke(&item, Some(&verdict)).is_ok());
        assert!(!target.exists());
    }

    #[test]
    fn test_nuke_fails_at_execution_time_if_filesystem_changed() {
        let (_dir, root, target) = setup_mock_project();
        let engine = SweeperEngine::new();

        let item = TriageItem {
            project_id: ProjectId("1".into()),
            project_root: root.clone(),
            target_path: target.clone(),
            matched_pattern: "node_modules".into(),
            size_bytes: 100,
            idle_days: 20,
            nuke_eligibility: NukeEligibility::Eligible, // Was eligible at listing time
            recommended_action: TriageAction::NukeSafe,
        };

        // Simulating the user deleting the lockfile before action occurs
        let lockfile = root.join("package-lock.json");
        fs::remove_file(&lockfile).unwrap();

        let verdict = ReproducibilityVerdict {
            is_reproducible: true,
            details: "".into(),
        };

        // Execution-time re-verification should catch it and throw a Safety Gate veto error
        let result = engine.nuke(&item, Some(&verdict));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Safety Gate veto"));
    }

    #[test]
    fn test_archive_creates_transparent_symlink() {
        let (_dir, root, target) = setup_mock_project();

        // Put a file inside the target so we can verify it's readable via symlink
        fs::write(target.join("test.txt"), "hello world").unwrap();

        let archive_dir = TempDir::new().unwrap();

        let item = TriageItem {
            project_id: ProjectId("1".into()),
            project_root: root,
            target_path: target.clone(),
            matched_pattern: "node_modules".into(),
            size_bytes: 100,
            idle_days: 20,
            nuke_eligibility: NukeEligibility::Locked { reason: "".into() },
            recommended_action: TriageAction::Archive,
        };

        let engine = SweeperEngine::new();
        assert!(engine.archive(&item, archive_dir.path()).is_ok());

        // Target is now a symlink
        let metadata = fs::symlink_metadata(&target).unwrap();
        assert!(metadata.file_type().is_symlink());

        // The file inside is still readable through the symlink!
        let content = fs::read_to_string(target.join("test.txt")).unwrap();
        assert_eq!(content, "hello world");
    }
}
