use clap::Args;
use sprawl_archivist::{Archivist, SysRamMonitor};
use sprawl_core::Result;
use std::path::PathBuf;

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

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    #[allow(unused_variables)]
    let data_dir = PathBuf::from(home).join(".sprawl").join("archivist");

    #[cfg(feature = "real-archivist")]
    let mut archivist = Archivist::new_real(&data_dir).await.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
    
    #[cfg(not(feature = "real-archivist"))]
    let mut archivist = Archivist::new_mock().map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

    println!("Starting background indexer...");
    archivist.start_background_indexer(SysRamMonitor).map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
    
    if let Some(handle) = archivist.indexer_handle.take() {
        // Wait for it to finish its indexing pass if we started it explicitly
        let _ = handle.join();
        println!("Indexing complete.");
    }
    
    Ok(())
}
