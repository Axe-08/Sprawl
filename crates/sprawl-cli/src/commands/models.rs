use clap::{Args, Subcommand};
use sprawl_core::SprawlError;
use sprawl_inference::{DeviceTarget, InferenceEngine, RealSysInfo, DEFAULT_MODEL};
use std::sync::mpsc;
use tokio::task;

#[derive(Args)]
pub struct ModelsArgs {
    #[command(subcommand)]
    pub command: ModelsCommand,
}

#[derive(Subcommand)]
pub enum ModelsCommand {
    /// Download missing models for inference backend
    Download,
}

pub async fn handle(args: &ModelsArgs, _json: bool) -> Result<(), SprawlError> {
    match &args.command {
        ModelsCommand::Download => {
            println!("Ensuring required inference models are downloaded...");
            
            let engine = InferenceEngine::new(DEFAULT_MODEL, DeviceTarget::Cpu, RealSysInfo);
            
            let (tx, rx) = mpsc::channel();
            
            let progress_task = task::spawn_blocking(move || {
                let mut last_pct = 255; // Invalid value to force first print
                while let Ok(progress) = rx.recv() {
                    match progress {
                        sprawl_inference::EngineProgress::Downloading { pct, bytes_done, bytes_total } => {
                            if pct != last_pct {
                                last_pct = pct;
                                print!("\rDownloading... {}% ({}/{}) bytes      ", pct, bytes_done, bytes_total);
                                use std::io::Write;
                                let _ = std::io::stdout().flush();
                            }
                        },
                        _ => {}
                    }
                }
            });

            match engine.download_model(Some(tx)).await {
                Ok(path) => {
                    // Just drop rx by ignoring it, this finishes the loop
                    progress_task.await.unwrap();
                    println!("\nModel successfully downloaded to: {}", path.display());
                    Ok(())
                }
                Err(e) => {
                    progress_task.await.unwrap();
                    println!("\nFailed to download model.");
                    Err(SprawlError::Other(e.to_string()))
                }
            }
        }
    }
}
