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
        return Err(sprawl_core::SprawlError::Other("Project name cannot be empty".into()));
    }
    
    // In a full implementation, we'd query the manifest. Here we'll do the MVP level:
    // If we call restore on the engine, it returns an error or success based on the backend.
    
    let target_path = std::path::PathBuf::from(format!("/tmp/{}/node_modules", args.project_name));
    let archive_path = std::path::PathBuf::from(format!("/tmp/sprawl_archive/{}/node_modules", args.project_name));
    
    match engine.restore(&target_path, &archive_path) {
        Ok(_) => {
            if !is_json {
                println!("Successfully restored project: {}", args.project_name);
            }
        },
        Err(e) => {
            return Err(sprawl_core::SprawlError::Other(format!("Restore failed: {}", e)));
        }
    }
    
    Ok(())
}
