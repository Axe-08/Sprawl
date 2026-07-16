use clap::Args;
use sprawl_core::Result;

#[derive(Args)]
pub struct IndexArgs {
    /// Start the indexing process explicitly
    #[arg(long)]
    pub start: bool,
}

pub async fn handle(args: &IndexArgs, _is_json: bool) -> Result<()> {
    if !args.start {
        println!("Pass --start to explicitly start the background indexer.");
        return Ok(());
    }

    if let Ok(client) = sprawl_daemon::IpcClient::new() {
        if let Ok(resp) = client.send_request(&sprawl_daemon::IpcRequest::StartIndexer).await {
            match resp {
                sprawl_daemon::IpcResponse::Ok => println!("Indexer started by daemon."),
                sprawl_daemon::IpcResponse::Error(e) => {
                    return Err(sprawl_core::SprawlError::Other(format!("Daemon error: {}", e)));
                }
                _ => println!("Indexer signal sent."),
            }
        } else {
            println!("Daemon not running. Start it first with: sprawl daemon start");
        }
    } else {
        println!("Could not connect to daemon. Start it first with: sprawl daemon start");
    }

    Ok(())
}
