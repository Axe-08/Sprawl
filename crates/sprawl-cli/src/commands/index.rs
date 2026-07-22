use clap::Args;
use sprawl_core::Result;

#[derive(Args)]
pub struct IndexArgs {
    /// Start the indexing process and show live progress
    #[arg(long)]
    pub start: bool,
}

pub async fn handle(args: &IndexArgs, _is_json: bool) -> Result<()> {
    if !args.start {
        println!("Pass --start to explicitly start the background indexer.");
        println!("Tip: Make sure to register projects first with `sprawl project add <path>`.");
        return Ok(());
    }

    let client = match sprawl_daemon::IpcClient::new() {
        Ok(c) => c,
        Err(_) => {
            println!("Could not connect to daemon. Start it first with: sprawl daemon start");
            return Ok(());
        }
    };

    // Kick off the indexer
    match client.send_request(&sprawl_daemon::IpcRequest::StartIndexer).await {
        Ok(sprawl_daemon::IpcResponse::Ok) => {
            println!("Indexer started. Waiting for progress...");
        }
        Ok(sprawl_daemon::IpcResponse::Error(e)) => {
            return Err(sprawl_core::SprawlError::Other(format!("Daemon error: {}", e)));
        }
        Err(_) | Ok(_) => {
            println!("Daemon not running. Start it first with: sprawl daemon start");
            return Ok(());
        }
    }

    // Brief pause to let the indexer do its pre-count pass
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // Set up indicatif progress bar
    let pb = indicatif::ProgressBar::new(0);
    pb.set_style(
        indicatif::ProgressStyle::with_template(
            "{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} files  {msg}",
        )
        .unwrap()
        .progress_chars("━━╸ "),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    // Poll loop — check every 800ms
    loop {
        let client = match sprawl_daemon::IpcClient::new() {
            Ok(c) => c,
            Err(_) => break,
        };

        match client.send_request(&sprawl_daemon::IpcRequest::IndexStatus).await {
            Ok(sprawl_daemon::IpcResponse::IndexProgress {
                files_indexed,
                files_total,
                current_file,
                is_running,
            }) => {
                if files_total > 0 {
                    pb.set_length(files_total);
                }
                pb.set_position(files_indexed);
                if !current_file.is_empty() {
                    // Truncate long paths from left so the bar stays readable
                    let display = if current_file.len() > 50 {
                        format!("…{}", &current_file[current_file.len() - 49..])
                    } else {
                        current_file.clone()
                    };
                    pb.set_message(display);
                }

                if !is_running && files_indexed >= files_total && files_total > 0 {
                    pb.finish_with_message(format!(
                        "Done — {} files indexed",
                        files_indexed
                    ));
                    break;
                }

                // If not yet running (pre-count phase), show spinner
                if !is_running && files_total == 0 {
                    pb.set_message("Counting files...");
                }
            }
            _ => {
                pb.abandon_with_message("Lost connection to daemon.");
                break;
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }

    Ok(())
}
