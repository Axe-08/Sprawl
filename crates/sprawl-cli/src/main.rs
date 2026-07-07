use clap::{Parser, Subcommand};
use std::process;
use tracing::Level;

mod commands;
mod config;

#[derive(Parser)]
#[command(name = "sprawl", about = "Local-first codebase overseer", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Enable verbose logging output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output results as JSON (for scripting)
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Control the background watcher daemon
    Daemon(commands::daemon::DaemonArgs),
    /// Manage WASM plugins
    Plugin(commands::plugin::PluginArgs),
    /// Reverse a Sweeper archive action
    Restore(commands::restore::RestoreArgs),
    /// Verify a vaulted secret via MCP network router
    Verify(commands::verify::VerifyArgs),
    /// Simulate revoking a token to assess blast radius
    SimulateRevoke(commands::simulate_revoke::SimulateRevokeArgs),
    /// Bundle a directory into a token-optimized markdown representation
    Bundle(commands::bundle::BundleArgs),
    /// Perform a deep analysis of a project using local LLM inference
    Analyze(commands::analyze::AnalyzeArgs),
    /// Start the Terminal UI
    Ui,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize tracing based on verbose flag
    let level = if cli.verbose { Level::DEBUG } else { Level::WARN };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(std::io::stderr)
        .init();

    let result = match &cli.command {
        Command::Daemon(args) => commands::daemon::handle(args, cli.json),
        Command::Plugin(args) => commands::plugin::handle(args, cli.json),
        Command::Restore(args) => commands::restore::handle(args, cli.json),
        Command::Verify(args) => commands::verify::handle(args, cli.json),
        Command::SimulateRevoke(args) => commands::simulate_revoke::handle(args, cli.json),
        Command::Bundle(args) => commands::bundle::handle(args, cli.json),
        Command::Analyze(args) => commands::analyze::handle(args, cli.json).await,
        Command::Ui => {
            if let Err(e) = sprawl_tui::run() {
                Err(sprawl_core::SprawlError::Other(format!("TUI Error: {}", e)))
            } else {
                Ok(())
            }
        }
    };

    if let Err(e) = result {
        if cli.json {
            eprintln!("{}", serde_json::json!({
                "status": "error",
                "error": e.to_string()
            }));
        } else {
            eprintln!("Error: {}", e);
        }
        
        let msg = e.to_string();
        if msg.contains("INSUFFICIENT HEADROOM") {
            process::exit(3);
        } else if msg.contains("safety gate") || msg.contains("Locked") {
            process::exit(2);
        } else {
            process::exit(1);
        }
    }
    
    process::exit(0);
}
