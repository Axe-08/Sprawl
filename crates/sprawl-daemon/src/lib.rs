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
    sentinel: Arc<sprawl_sentinel::scanner::SentinelScanner>,
    ledger_path: std::path::PathBuf,
) -> Result<()> {
    tracing::info!("Daemon entering main run loop");

    // 1. Initialize IPC Server for TUI/CLI
    let ipc = IpcServer::new()?;
    
    #[cfg(unix)]
    let listener = ipc.bind().await?;

    let start_time = std::time::Instant::now();

    // Spawn IPC listener in background
    #[cfg(unix)]
    tokio::spawn({
        let archivist = archivist.clone();
        let sentinel = sentinel.clone();
        let start_time = start_time;
        async move {
            loop {
                match listener.accept().await {
                    Ok((mut socket, _addr)) => {
                        let archivist_clone = archivist.clone();
                        let sentinel_clone = sentinel.clone();
                        let start_time = start_time;
                        tokio::spawn(async move {
                            use tokio::io::{AsyncReadExt, AsyncWriteExt};
                            let mut buf = vec![0; 1024 * 1024]; // 1MB buffer
                            if let Ok(n) = socket.read(&mut buf).await {
                                let req_str = String::from_utf8_lossy(&buf[..n]);
                                if let Ok(req) = serde_json::from_str::<IpcRequest>(&req_str) {
                                    let resp = match req {
                                        IpcRequest::Ping => {
                                            IpcResponse::Pong {
                                                pid: std::process::id(),
                                                uptime_secs: start_time.elapsed().as_secs(),
                                            }
                                        }
                                        IpcRequest::Search { query, top_k } => {
                                            match archivist_clone.search(&query, top_k).await {
                                                Ok(results) => IpcResponse::SearchResults(results),
                                                Err(e) => IpcResponse::Error(e.to_string()),
                                            }
                                        }
                                        IpcRequest::GetSentinelInbox => {
                                            IpcResponse::SentinelInbox(sentinel_clone.get_ambiguous_secrets())
                                        }
                                        IpcRequest::SentinelAccept { id } => {
                                            sentinel_clone.mark_accepted(id);
                                            IpcResponse::Ok
                                        }
                                        IpcRequest::SentinelReject { id } => {
                                            sentinel_clone.mark_rejected(id);
                                            IpcResponse::Ok
                                        }
                                        IpcRequest::BatchClassify { secrets } => {
                                            let mut engine = sprawl_inference::InferenceEngine::new(
                                                sprawl_inference::DEFAULT_MODEL,
                                                sprawl_inference::DeviceTarget::Cpu,
                                                sprawl_inference::RealSysInfo,
                                            );
                                            match sprawl_sentinel::llm::batch_classify(&secrets, &mut engine).await {
                                                Ok(results) => IpcResponse::BatchClassifyResult(results),
                                                Err(e) => IpcResponse::Error(e.to_string()),
                                            }
                                        }
                                        IpcRequest::StartIndexer => {
                                            match archivist_clone.start_background_indexer(sprawl_archivist::SysRamMonitor) {
                                                Ok(_) => IpcResponse::Ok,
                                                Err(e) => IpcResponse::Error(e.to_string()),
                                            }
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

    let project_roots: Vec<std::path::PathBuf> = {
        let mut results = Vec::new();
        if let Ok(conn) = rusqlite::Connection::open(&ledger_path) {
            if let Ok(mut stmt) = conn.prepare("SELECT root_path FROM projects WHERE status IN ('active', 'idle')") {
                if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                    for r in rows {
                        if let Ok(path) = r {
                            results.push(std::path::PathBuf::from(path));
                        }
                    }
                }
            } else {
                let _ = conn.execute(
                    "CREATE TABLE IF NOT EXISTS projects (
                        id TEXT PRIMARY KEY,
                        root_path TEXT UNIQUE NOT NULL,
                        ecosystem TEXT,
                        status TEXT NOT NULL DEFAULT 'active',
                        last_seen TEXT,
                        created_at TEXT
                    )",
                    []
                );
            }
        }
        results
    };
    tracing::info!("Watching {} project roots", project_roots.len());
    let config_paths = vec![];
    let (_watcher, rx) = FilesystemWatcher::new(&project_roots, &config_paths)?;

    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::task::spawn_blocking(move || {
        while let Ok(event) = rx.recv() {
            let _ = event_tx.send(event);
        }
    });

    let mut dedup = EventDeduplicator::new();

    // 3. Main event loop (Filesystem changes)
    loop {
        tokio::select! {
            // Filesystem events
            Some(e) = event_rx.recv() => {
                if let Ok(event) = e {
                    dedup.ingest(event);
                }
            }
            
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                // Flush deduplicator
                if let Some(batches) = dedup.flush_if_ready() {
                    for (_root, events) in batches {
                        for event in events {
                            if let notify::EventKind::Create(_) | notify::EventKind::Modify(_) = event.kind {
                                for path in &event.paths {
                                    let archivist_clone = archivist.clone();
                                    let path_clone = path.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = archivist_clone.index_file(&path_clone).await {
                                            tracing::warn!("Index failed for {}: {}", path_clone.display(), e);
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
