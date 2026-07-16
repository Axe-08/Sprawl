use clap::Args;
use sprawl_core::Result;

#[derive(Args)]
pub struct RestoreArgs {
    /// Name of the archived project to restore
    pub project_name: String,
}

pub fn handle(args: &RestoreArgs, is_json: bool) -> Result<()> {
    // M12 Restore command. We will load the manifest, find the most recent matching Archive action that hasn't been restored.
    // If it's a Nuke, error out.
    // Right now, SweeperEngine uses the mock backend, but we'll stub the manifest loading and restore logic.
    let engine = sprawl_sweeper::engine::SweeperEngine::new();

    // As per M12 spec, we simulate restoring a project archive.
    if args.project_name.is_empty() {
        return Err(sprawl_core::SprawlError::Other(
            "Project name cannot be empty".into(),
        ));
    }

    let data_dir = sprawl_core::platform::sprawl_data_dir()?;
    let archive_dir = data_dir.join("archive").join(&args.project_name);

    if !archive_dir.exists() {
        return Err(sprawl_core::SprawlError::Other(format!(
            "No archived project named '{}' found.\nLooked in: {}\nUse `sprawl triage list` to see what can be restored.",
            args.project_name,
            archive_dir.display()
        )));
    }

    // The restore target is the parent of the archive dir (original location encoded in manifest)
    let manifest_path = archive_dir.join(".sprawl-manifest.json");
    let original_path: std::path::PathBuf = if manifest_path.exists() {
        let manifest_str = std::fs::read_to_string(&manifest_path)
            .map_err(sprawl_core::SprawlError::Io)?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_str)
            .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
        std::path::PathBuf::from(manifest["original_path"].as_str().unwrap_or("."))
    } else {
        // Fallback: restore to current directory
        std::env::current_dir().map_err(sprawl_core::SprawlError::Io)?.join(&args.project_name)
    };

    match engine.restore(&original_path, &archive_dir) {
        Ok(_) => {
            if !is_json { println!("Successfully restored '{}' to {}", args.project_name, original_path.display()); }
        }
        Err(e) => return Err(sprawl_core::SprawlError::Other(format!("Restore failed: {}", e))),
    }

    Ok(())
}
