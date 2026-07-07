use clap::Args;
use sprawl_core::Result;
use uuid::Uuid;

#[derive(Args)]
pub struct VerifyArgs {
    /// The UUID of the secret in the ledger
    #[arg(short, long)]
    pub key: String,
}

pub fn handle(args: &VerifyArgs, is_json: bool) -> Result<()> {
    let secret_id = Uuid::parse_str(&args.key).unwrap_or(Uuid::nil());
    match sprawl_sentinel::verify::verify_mcp(secret_id) {
        Ok(status) => {
            if is_json {
                println!(
                    "{}",
                    serde_json::json!({"status": "ok", "result": format!("{:?}", status)})
                );
            } else {
                println!("Verification result: {:?}", status);
            }
            Ok(())
        }
        Err(e) => Err(sprawl_core::SprawlError::Other(format!(
            "Verification failed: {}",
            e
        ))),
    }
}
