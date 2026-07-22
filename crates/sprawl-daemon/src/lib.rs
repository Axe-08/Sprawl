pub mod ipc;
pub mod process;
pub mod watcher;

pub use ipc::{IpcServer, IpcRequest, IpcResponse, IpcClient, ProjectEntry as IpcProjectEntry};
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
                                        IpcRequest::Ask { query } => {
                                            let res: Result<String> = async {
                                                let results = archivist_clone.search(&query, 3).await.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
                                                if results.is_empty() {
                                                    return Ok("No context found in the codebase to answer the query.".to_string());
                                                }
                                                let mut context_block = String::new();
                                                for (i, r) in results.iter().enumerate() {
                                                    context_block.push_str(&format!("--- FILE: {} (Match {}) ---\n{}\n\n", r.file_path, i+1, r.chunk_text));
                                                }
                                                let prompt = format!(
                                                    "You are an expert programming assistant answering questions about a codebase. Use ONLY the provided codebase context to answer the question.\n\nContext:\n{}\nQuestion: {}\nAnswer:",
                                                    context_block, query
                                                );
                                                let mut engine = sprawl_inference::InferenceEngine::new(
                                                    sprawl_inference::DEFAULT_MODEL,
                                                    sprawl_inference::DeviceTarget::Cpu,
                                                    sprawl_inference::RealSysInfo,
                                                );
                                                engine.preflight_check().map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
                                                let path = engine.ensure_model(None).await.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
                                                engine.load_model(&path, None).map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
                                                engine.run_prompt(&prompt).await.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))
                                            }.await;
                                            match res {
                                                Ok(answer) => IpcResponse::AskResult(answer),
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
                                            
                                            // Load model first
                                            let res = async {
                                                let path = engine.ensure_model(None).await?;
                                                engine.load_model(&path, None)?;
                                                sprawl_sentinel::llm::batch_classify(&secrets, &mut engine).await
                                            }.await;

                                            match res {
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
                                        IpcRequest::IndexStatus => {
                                            let (indexed, total, current, running) = archivist_clone.index_progress();
                                            IpcResponse::IndexProgress {
                                                files_indexed: indexed,
                                                files_total: total,
                                                current_file: current,
                                                is_running: running,
                                            }
                                        }
                                        IpcRequest::RegisterProject { path } => {
                                            let result: std::result::Result<String, String> = (|| {
                                                let ledger_path = sprawl_core::platform::sprawl_data_dir()
                                                    .map_err(|e| e.to_string())?
                                                    .join("ledger.sqlite");
                                                let conn = sprawl_core::ledger::initialize_db(&ledger_path)
                                                    .map_err(|e| e.to_string())?;
                                                let id = uuid::Uuid::new_v4().to_string();
                                                let now = chrono::Utc::now().to_rfc3339();
                                                conn.execute(
                                                    "INSERT OR IGNORE INTO projects (id, root_path, status, last_seen, created_at) VALUES (?1, ?2, 'active', ?3, ?3)",
                                                    rusqlite::params![id, path, now],
                                                ).map_err(|e| e.to_string())?;
                                                // If it already existed, fetch its id
                                                let existing_id: String = conn.query_row(
                                                    "SELECT id FROM projects WHERE root_path = ?1",
                                                    rusqlite::params![path],
                                                    |r| r.get(0),
                                                ).map_err(|e| e.to_string())?;
                                                Ok(existing_id)
                                            })();
                                            match result {
                                                Ok(id) => IpcResponse::ProjectRegistered { id },
                                                Err(e) => IpcResponse::Error(e),
                                            }
                                        }
                                        IpcRequest::UnregisterProject { path, hard } => {
                                            let result: std::result::Result<(), String> = (|| {
                                                let ledger_path = sprawl_core::platform::sprawl_data_dir()
                                                    .map_err(|e| e.to_string())?
                                                    .join("ledger.sqlite");
                                                let conn = sprawl_core::ledger::initialize_db(&ledger_path)
                                                    .map_err(|e| e.to_string())?;
                                                if hard {
                                                    conn.execute(
                                                        "DELETE FROM projects WHERE root_path = ?1",
                                                        rusqlite::params![path],
                                                    ).map_err(|e| e.to_string())?;
                                                } else {
                                                    conn.execute(
                                                        "UPDATE projects SET status = 'idle' WHERE root_path = ?1",
                                                        rusqlite::params![path],
                                                    ).map_err(|e| e.to_string())?;
                                                }
                                                Ok(())
                                            })();
                                            match result {
                                                Ok(_) => IpcResponse::Ok,
                                                Err(e) => IpcResponse::Error(e),
                                            }
                                        }
                                        IpcRequest::ListProjects => {
                                            let result: std::result::Result<Vec<ipc::ProjectEntry>, String> = (|| {
                                                let ledger_path = sprawl_core::platform::sprawl_data_dir()
                                                    .map_err(|e| e.to_string())?
                                                    .join("ledger.sqlite");
                                                let conn = sprawl_core::ledger::initialize_db(&ledger_path)
                                                    .map_err(|e| e.to_string())?;
                                                let mut stmt = conn.prepare(
                                                    "SELECT id, root_path, status, ecosystem, created_at FROM projects ORDER BY created_at DESC"
                                                ).map_err(|e| e.to_string())?;
                                                let entries = stmt.query_map([], |r| {
                                                    Ok(ipc::ProjectEntry {
                                                        id: r.get(0)?,
                                                        root_path: r.get(1)?,
                                                        status: r.get(2)?,
                                                        ecosystem: r.get(3)?,
                                                        created_at: r.get(4)?,
                                                    })
                                                }).map_err(|e| e.to_string())?;
                                                Ok(entries.flatten().collect())
                                            })();
                                            match result {
                                                Ok(projects) => IpcResponse::Projects(projects),
                                                Err(e) => IpcResponse::Error(e),
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
