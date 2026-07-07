pub mod ipc;
pub mod process;
pub mod watcher;

pub use ipc::IpcServer;
pub use process::DaemonContext;
pub use watcher::{EventDeduplicator, FilesystemWatcher};

use sprawl_core::Result;

/// Main entry loop for the daemon.
pub async fn run_daemon_loop() -> Result<()> {
    tracing::info!("Daemon entering main run loop");

    // 1. Initialize IPC Server for TUI/CLI
    let ipc = IpcServer::new()?;
    ipc.bind().await?;

    // 2. Setup Filesystem Watcher (normally we'd read project roots from the Ledger first)
    let project_roots = vec![];
    let config_paths = vec![];
    let (_watcher, rx) = FilesystemWatcher::new(&project_roots, &config_paths)?;

    let mut dedup = EventDeduplicator::new();

    // 3. Main event loop (blocking / async combined)
    // For this mockup, we would use tokio channels or select over the blocking notify rx.
    loop {
        // Read events...
        if let Ok(Ok(e)) = rx.try_recv() {
            dedup.ingest(e);
        }

        // Flush deduplicator...
        if let Some(batches) = dedup.flush_if_ready() {
            for (root, events) in batches {
                tracing::info!(
                    "Processing {} events for project root: {}",
                    events.len(),
                    root.display()
                );
                // Here we would dispatch to Event Router (Archaeologist, Sweeper, Sentinel)
            }
        }

        // Sleep to avoid pegging CPU (satisfies NFR-1)
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}
