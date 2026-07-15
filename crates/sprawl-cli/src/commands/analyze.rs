use clap::Args;
use sprawl_archaeologist::analyze::analyze_deep;
use sprawl_core::Result;
use sprawl_inference::{DeviceTarget, InferenceEngine, RealSysInfo, DEFAULT_MODEL};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Args)]
pub struct AnalyzeArgs {
    /// The directory to analyze
    pub dir: PathBuf,
    /// Enable deep analysis via inference
    #[arg(long)]
    pub deep: bool,
}

pub async fn handle(args: &AnalyzeArgs, is_json: bool) -> Result<()> {
    if args.deep {
        if !is_json {
            println!("Initializing Inference Engine...");
        }
        let mut engine = InferenceEngine::new(DEFAULT_MODEL, DeviceTarget::Cpu, RealSysInfo);

        match analyze_deep(&args.dir, &mut engine).await {
            Ok(derived) => {
                if is_json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "name": derived.name,
                            "ecosystem": derived.ecosystem,
                            "frameworks": derived.frameworks
                        })
                    );
                } else {
                    println!("Analysis Complete:");
                    println!("Name: {}", derived.name);
                    println!("Ecosystem: {}", derived.ecosystem);
                    println!("Frameworks: {:?}", derived.frameworks);

                    print!("Save configuration? [L]ocal / [g]lobal / [n]o: ");
                    io::stdout().flush().unwrap();

                    let mut input = String::new();
                    io::stdin().read_line(&mut input).unwrap();
                    let choice = input.trim().to_lowercase();

                    let config_toml = format!(
                        "[project]\nname = \"{}\"\necosystem = \"{}\"\nframeworks = {:?}\n",
                        derived.name, derived.ecosystem, derived.frameworks
                    );

                    if choice == "l" || choice.is_empty() {
                        let path = args.dir.join(".sprawl.toml");
                        std::fs::write(&path, config_toml).map_err(sprawl_core::SprawlError::Io)?;
                        println!("Saved to {}", path.display());
                    } else if choice == "g" {
                        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                        let path = PathBuf::from(home)
                            .join(".sprawl")
                            .join("cache")
                            .join(format!("{}.toml", derived.name));
                        if let Some(parent) = path.parent() {
                            std::fs::create_dir_all(parent)
                                .map_err(sprawl_core::SprawlError::Io)?;
                        }
                        std::fs::write(&path, config_toml).map_err(sprawl_core::SprawlError::Io)?;
                        println!("Saved to global cache: {}", path.display());
                    } else {
                        println!("Discarded.");
                    }
                }
            }
            Err(e) => return Err(e),
        }
    } else {
        if !is_json {
            println!("Running Archaeologist Fast Path (WASM plugins)...");
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let plugin_dir = PathBuf::from(home).join(".sprawl").join("plugins");

        let host = sprawl_plugin_host::PluginHost::new(true, None).expect("Failed to init PluginHost");
        let mut registry = sprawl_plugin_host::PluginRegistry::new();

        if let Ok(entries) = std::fs::read_dir(&plugin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                    let name = path.file_stem().unwrap().to_string_lossy().to_string();
                    if let Ok(plugin) = host.load_plugin(&path, &name, None) {
                        registry.register(plugin);
                    }
                }
            }
        }

        let arch = sprawl_archaeologist::Archaeologist::new(host, registry);

        match arch.detect_stack(&args.dir).await {
            Ok((Some(primary), _)) => {
                if is_json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "ecosystem": primary.ecosystem,
                            "reproducible": primary.reproducibility.is_reproducible,
                            "evidence": primary.reproducibility.evidence,
                            "entry_points": primary.entry_points,
                            "dependencies_count": primary.dependencies.len()
                        })
                    );
                } else {
                    println!("Detection successful via fast-path!");
                    println!("Ecosystem: {}", primary.ecosystem);
                    println!("Reproducible: {}", primary.reproducibility.is_reproducible);
                    if !primary.reproducibility.is_reproducible {
                        println!("Evidence against reproducibility:");
                        for ev in primary.reproducibility.evidence {
                            println!("  - {}", ev);
                        }
                    }
                    println!("Entry points: {:?}", primary.entry_points);
                    println!("Dependencies: {} found", primary.dependencies.len());
                }
            }
            Ok((None, _)) => {
                if is_json {
                    println!(
                        "{}",
                        serde_json::json!({"error": "No known stack detected"})
                    );
                } else {
                    println!("No known stack detected via fast-path.");
                    println!("Hint: Try using `sprawl analyze --deep` for L2 analysis.");
                }
            }
            Err(e) => {
                return Err(sprawl_core::SprawlError::Other(format!(
                    "Fast-path detection failed: {}",
                    e
                )));
            }
        }
    }
    Ok(())
}
