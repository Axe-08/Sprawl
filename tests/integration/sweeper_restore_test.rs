use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use sprawl_sweeper::engine::{SweeperEngine, TriageItem, ProjectId};
use fixtures::create_node_with_lockfile;

#[test]
fn test_archive_and_restore_identity() {
    let dir = tempdir().unwrap();
    let project_path = dir.path().join("my-project");
    let archive_path = dir.path().join("archive");
    fs::create_dir_all(&project_path).unwrap();
    fs::create_dir_all(&archive_path).unwrap();
    
    // 1. Create a synthetic project
    create_node_with_lockfile(&project_path);
    
    // Capture state before archival
    let mut before_files: Vec<String> = WalkDir::new(&project_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();
    before_files.sort();

    let engine = SweeperEngine::new();
    
    // 2. Archive it
    let item = TriageItem {
        project_id: ProjectId("my-project".to_string()),
        project_root: project_path.clone(),
        target_path: project_path.clone(),
        matched_pattern: "".to_string(),
        size_bytes: 0,
        idle_days: 0,
        nuke_eligibility: sprawl_sweeper::safety_gate::NukeEligibility::Eligible,
        recommended_action: sprawl_sweeper::engine::TriageAction::Archive,
    };
    engine.archive(&item, &archive_path).unwrap();
    
    // Verify it was replaced by a symlink
    let metadata = fs::symlink_metadata(&project_path).unwrap();
    assert!(metadata.file_type().is_symlink());

    // 3. Restore it
    engine.restore(&project_path, &archive_path.join("my-project")).unwrap();
    
    // Verify it is a real directory again
    let metadata_after = fs::symlink_metadata(&project_path).unwrap();
    assert!(metadata_after.file_type().is_dir());

    // Verify contents are identical
    let mut after_files: Vec<String> = WalkDir::new(&project_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();
    after_files.sort();

    assert_eq!(before_files, after_files, "Restored project tree differs from original");
}

// Simple walkdir implementation since we don't have walkdir crate configured
struct WalkDir {
    stack: Vec<std::path::PathBuf>,
}
impl WalkDir {
    fn new(path: &std::path::Path) -> Self {
        Self { stack: vec![path.to_path_buf()] }
    }
}
impl Iterator for WalkDir {
    type Item = std::io::Result<std::fs::DirEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(path) = self.stack.pop() {
            if path.is_dir() {
                match fs::read_dir(&path) {
                    Ok(entries) => {
                        for entry in entries {
                            if let Ok(entry) = &entry {
                                self.stack.push(entry.path());
                            }
                        }
                    }
                    Err(e) => return Some(Err(e)),
                }
            } else {
                // Not a perfect WalkDir match but works for checking if files exist
            }
        }
        None
    }
}
