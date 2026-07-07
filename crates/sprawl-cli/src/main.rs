use clap::{Parser, Subcommand};
use sprawl_archaeologist::analyze::analyze_deep;
use sprawl_archaeologist::bundle::{BundleOptions, Bundler};
use sprawl_inference::{DeviceTarget, InferenceEngine, RealSysInfo, DEFAULT_MODEL};
use std::io::{self, Write};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify a vaulted secret via MCP network router
    Verify {
        /// The UUID of the secret in the ledger
        #[arg(short, long)]
        key: String,
    },
    /// Simulate revoking a token to assess blast radius
    SimulateRevoke {
        /// The UUID of the secret to simulate revoking
        #[arg(short, long)]
        key: String,
    },
    /// Bundle a directory into a token-optimized markdown representation
    Bundle {
        /// The directory to bundle
        dir: PathBuf,
        /// Maximum tokens allowed
        #[arg(long, default_value_t = 32768)]
        max_tokens: usize,
        /// Output file path (if omitted, prints to stdout)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Perform a deep analysis of a project using local LLM inference
    Analyze {
        /// The directory to analyze
        dir: PathBuf,
        /// Enable deep analysis via inference
        #[arg(long)]
        deep: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Verify { key } => {
            let secret_id = Uuid::parse_str(key).unwrap_or(Uuid::nil());
            match sprawl_sentinel::verify::verify_mcp(secret_id) {
                Ok(status) => println!("Verification result: {:?}", status),
                Err(e) => eprintln!("Verification failed: {}", e),
            }
        }
        Commands::SimulateRevoke { key } => {
            // M19 will implement the actual graph resolution via Archivist
            println!("Simulating revocation for key {}...", key);
            println!(
                "No immediate blast radius detected (Archivist graph indexing not yet active)."
            );
        }
        Commands::Bundle {
            dir,
            max_tokens,
            output,
        } => {
            let bundler = Bundler::new();
            let opts = BundleOptions {
                max_tokens: *max_tokens,
                output_path: output.clone(),
            };

            match bundler.bundle_directory(dir, &opts) {
                Ok(content) => {
                    if let Some(out_path) = output {
                        if let Err(e) = std::fs::write(out_path, content) {
                            eprintln!("Failed to write bundle: {}", e);
                        } else {
                            println!("Bundle written to {}", out_path.display());
                        }
                    } else {
                        println!("{}", content);
                    }
                }
                Err(e) => eprintln!("Failed to bundle: {}", e),
            }
        }
        Commands::Analyze { dir, deep } => {
            if *deep {
                println!("Initializing Inference Engine...");
                let mut engine =
                    InferenceEngine::new(DEFAULT_MODEL, DeviceTarget::Cpu, RealSysInfo);

                match analyze_deep(dir, &mut engine).await {
                    Ok(derived) => {
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
                            let path = dir.join(".sprawl.toml");
                            std::fs::write(&path, config_toml).unwrap();
                            println!("Saved to {}", path.display());
                        } else if choice == "g" {
                            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                            let path = PathBuf::from(home)
                                .join(".sprawl")
                                .join("cache")
                                .join(format!("{}.toml", derived.name));
                            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                            std::fs::write(&path, config_toml).unwrap();
                            println!("Saved to global cache: {}", path.display());
                        } else {
                            println!("Discarded.");
                        }
                    }
                    Err(e) => eprintln!("Analysis failed: {}", e),
                }
            } else {
                println!("Use --deep for Archaeologist L2/L3 analysis.");
            }
        }
    }
}
