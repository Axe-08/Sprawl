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
        /// Optional path to a manifest.json containing the Ed25519 signature
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Allow installing unsigned plugins
        #[arg(long)]
        allow_unsigned: bool,
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
        PluginAction::Install { source, checksum, manifest, allow_unsigned } => {
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

            if let Some(manifest_path) = manifest {
                let manifest_str = std::fs::read_to_string(manifest_path)
                    .map_err(|e| sprawl_core::SprawlError::Other(format!("Failed to read manifest: {}", e)))?;
                let parsed: sprawl_plugin_host::verify::PluginManifest = serde_json::from_str(&manifest_str)
                    .map_err(|e| sprawl_core::SprawlError::Other(format!("Failed to parse manifest: {}", e)))?;
                
                let public_key_bytes = sprawl_plugin_host::verify::COMMUNITY_SIGNING_KEY;
                let host = sprawl_plugin_host::PluginHost::new(*allow_unsigned, Some(&public_key_bytes))
                    .map_err(|e| sprawl_core::SprawlError::Other(format!("Failed to init PluginHost: {}", e)))?;
                
                let name = source.file_stem().unwrap().to_string_lossy().to_string();
                let _ = host.load_plugin(source, &name, Some(&parsed))
                    .map_err(|e| sprawl_core::SprawlError::Other(format!("Plugin verification failed: {}", e)))?;
            } else if !*allow_unsigned {
                 return Err(sprawl_core::SprawlError::Other(
                    "Cannot install unsigned plugin unless --allow-unsigned is provided".into()
                ));
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
