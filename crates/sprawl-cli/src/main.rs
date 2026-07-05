use clap::{Parser, Subcommand};
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
}

fn main() {
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
            println!("No immediate blast radius detected (Archivist graph indexing not yet active).");
        }
    }
}
