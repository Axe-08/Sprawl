use clap::Args;
use clap::Subcommand;
use sprawl_core::Result;
use std::path::PathBuf;

#[derive(Args)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub action: PluginAction,
}

#[derive(Subcommand)]
pub enum PluginAction {
    /// Install a WASM StackDetector plugin from a local path
    Install {
        source: PathBuf,
        /// Optional SHA-256 checksum to verify the plugin before installation
        #[arg(long)]
        checksum: Option<String>,
    },
    /// List all installed plugins
    List,
    /// Remove an installed plugin by name
    Remove { name: String },
    /// Update a plugin from a new local path
    Update { name: String, source: PathBuf },
}

pub fn handle(args: &PluginArgs, is_json: bool) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let plugin_dir = PathBuf::from(home).join(".sprawl").join("plugins");

    if !plugin_dir.exists() {
        std::fs::create_dir_all(&plugin_dir).map_err(|e| {
            sprawl_core::SprawlError::Other(format!("Failed to create plugins dir: {}", e))
        })?;
    }

    match &args.action {
        PluginAction::Install { source, checksum } => {
            if !source.exists() {
                return Err(sprawl_core::SprawlError::Other(
                    "Source file does not exist".into(),
                ));
            }
            if source.extension().and_then(|s| s.to_str()) != Some("wasm") {
                return Err(sprawl_core::SprawlError::Other(
                    "Only .wasm files are supported".into(),
                ));
            }

            if let Some(expected_hash) = checksum {
                let bytes = std::fs::read(source).map_err(|e| {
                    sprawl_core::SprawlError::Other(format!("Failed to read source file: {}", e))
                })?;
                use sha2::Digest;
                let mut hasher = sha2::Sha256::new();
                hasher.update(&bytes);
                let result = hasher.finalize();
                let computed_hash = hex::encode(result);

                if computed_hash != *expected_hash {
                    return Err(sprawl_core::SprawlError::Other(format!(
                        "Checksum verification failed. Expected {}, got {}",
                        expected_hash, computed_hash
                    )));
                }
                
                if !is_json {
                    println!("Checksum verified successfully.");
                }
            }

            let name = source.file_stem().unwrap().to_string_lossy().to_string();
            let dest = plugin_dir.join(format!("{}.wasm", name));

            std::fs::copy(source, &dest)
                .map_err(|e| sprawl_core::SprawlError::Other(format!("Install failed: {}", e)))?;

            if !is_json {
                println!("Successfully installed plugin '{}'", name);
            }
        }
        PluginAction::List => {
            let mut plugins = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&plugin_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                        let name = path.file_stem().unwrap().to_string_lossy().to_string();
                        plugins.push(name);
                    }
                }
            }

            if is_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "plugins": plugins
                    })
                );
            } else {
                if plugins.is_empty() {
                    println!("No plugins installed.");
                } else {
                    println!("Installed plugins:");
                    for plugin in plugins {
                        println!("  - {}", plugin);
                    }
                }
            }
        }
        PluginAction::Remove { name } => {
            let dest = plugin_dir.join(format!("{}.wasm", name));
            if dest.exists() {
                std::fs::remove_file(&dest).map_err(|e| {
                    sprawl_core::SprawlError::Other(format!("Failed to remove: {}", e))
                })?;
                if !is_json {
                    println!("Removed plugin '{}'", name);
                }
            } else {
                return Err(sprawl_core::SprawlError::Other(format!(
                    "Plugin '{}' not found",
                    name
                )));
            }
        }
        PluginAction::Update { name, source } => {
            if !source.exists() {
                return Err(sprawl_core::SprawlError::Other(
                    "Source file does not exist".into(),
                ));
            }
            let dest = plugin_dir.join(format!("{}.wasm", name));
            std::fs::copy(source, &dest)
                .map_err(|e| sprawl_core::SprawlError::Other(format!("Update failed: {}", e)))?;

            if !is_json {
                println!("Successfully updated plugin '{}'", name);
            }
        }
    }
    Ok(())
}
