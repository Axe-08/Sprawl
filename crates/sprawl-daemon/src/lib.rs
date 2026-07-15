pub mod ipc;
pub mod process;
pub mod watcher;

pub use ipc::{IpcServer, IpcRequest, IpcResponse, IpcClient};
pub use process::DaemonContext;
pub use watcher::{EventDeduplicator, FilesystemWatcher};

use sprawl_core::Result;
use std::sync::Arc;

/// Main entry loop for the daemon.
pub async fn run_daemon_loop(
    archivist: Arc<sprawl_archivist::Archivist>,
    sentinel: Arc<sprawl_sentinel::scanner::SentinelScanner>
) -> Result<()> {
    tracing::info!("Daemon entering main run loop");

    // 1. Initialize IPC Server for TUI/CLI
    let ipc = IpcServer::new()?;
    
    #[cfg(unix)]
    let listener = ipc.bind().await?;

    // Spawn IPC listener in background
    #[cfg(unix)]
    tokio::spawn({
        let archivist = archivist.clone();
        let sentinel = sentinel.clone();
        async move {
            loop {
                match listener.accept().await {
                    Ok((mut socket, _addr)) => {
                        let archivist_clone = archivist.clone();
                        let sentinel_clone = sentinel.clone();
                        tokio::spawn(async move {
                            use tokio::io::{AsyncReadExt, AsyncWriteExt};
                            let mut buf = vec![0; 1024 * 1024]; // 1MB buffer
                            if let Ok(n) = socket.read(&mut buf).await {
                                let req_str = String::from_utf8_lossy(&buf[..n]);
                                if let Ok(req) = serde_json::from_str::<IpcRequest>(&req_str) {
                                    let resp = match req {
                                        IpcRequest::Search { query, top_k } => {
                                            match archivist_clone.search(&query, top_k).await {
                                                Ok(results) => IpcResponse::SearchResults(results),
                                                Err(e) => IpcResponse::Error(e.to_string()),
                                            }
                                        }
                                        IpcRequest::GetSentinelInbox => {
                                            IpcResponse::SentinelInbox(vec![])
                                        }
                                        IpcRequest::SentinelAccept { id: _ } => {
                                            IpcResponse::Ok
                                        }
                                        IpcRequest::SentinelReject { id: _ } => {
                                            IpcResponse::Ok
                                        }
                                    };
                                    
                                    if let Ok(resp_json) = serde_json::to_string(&resp) {
                                        let _ = socket.write_all(resp_json.as_bytes()).await;
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => tracing::error!("Failed to accept IPC connection: {}", e),
                }
            }
        }
    });

    // 2. Setup Filesystem Watcher (normally we'd read project roots from the Ledger first)
    let project_roots = vec![];
    let config_paths = vec![];
    let (_watcher, rx) = FilesystemWatcher::new(&project_roots, &config_paths)?;

    let mut dedup = EventDeduplicator::new();

    // 3. Main event loop (Filesystem changes)
    loop {
        tokio::select! {
            // Filesystem events
            Ok(e) = tokio::task::spawn_blocking(move || rx.recv()) => {
                if let Ok(event) = e {
                    dedup.ingest(event);
                }
            }
            
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                // Flush deduplicator
                if let Some(batches) = dedup.flush_if_ready() {
                    for (root, events) in batches {
                        tracing::info!("Processing {} events for project root: {}", events.len(), root.display());
                        // Dispatch to Event Router
                    }
                }
            }
        }
    }
}
