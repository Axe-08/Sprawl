use clap::Args;
use sprawl_core::Result;

#[derive(Args)]
pub struct StatusArgs {}

pub async fn handle(_args: &StatusArgs, is_json: bool) -> Result<()> {
    let mut daemon_running = false;
    let mut daemon_pid = None;

    let ping_result = {
        if let Ok(client) = sprawl_daemon::IpcClient::new() {
            client.send_request(&sprawl_daemon::IpcRequest::Ping).await.ok()
        } else {
            None
        }
    };

    let mut uptime = 0;
    match ping_result {
        Some(sprawl_daemon::IpcResponse::Pong { pid, uptime_secs }) => {
            daemon_running = true;
            daemon_pid = Some(pid);
            uptime = uptime_secs;
        }
        _ => {
            daemon_running = false;
        }
    }
    // Determine backend features at compile time
    let archivist_backend = if cfg!(feature = "real-archivist") {
        "real"
    } else {
        "mock"
    };

    let inference_backend = if cfg!(feature = "real-inference") {
        "real"
    } else {
        "mock"
    };

    let mut projects_active = 0;
    let mut projects_idle = 0;
    let mut sentinel_unreviewed = 0;
    let mut sweeper_queue_total = 0usize;
    let mut sweeper_nuke_eligible = 0usize;

    if let Ok(ledger_path) = sprawl_core::platform::sprawl_data_dir().map(|d| d.join("ledger.sqlite")) {
        if let Ok(conn) = rusqlite::Connection::open(&ledger_path) {
            let _ = conn.query_row("SELECT count(*) FROM projects WHERE status = 'active'", [], |row| {
                projects_active = row.get::<_, usize>(0).unwrap_or(0);
                Ok(())
            });
            let _ = conn.query_row("SELECT count(*) FROM projects WHERE status = 'idle'", [], |row| {
                projects_idle = row.get::<_, usize>(0).unwrap_or(0);
                Ok(())
            });
            let _ = conn.query_row("SELECT count(*) FROM ambiguous_secrets WHERE status = 'pending'", [], |row| {
                sentinel_unreviewed = row.get::<_, usize>(0).unwrap_or(0);
                Ok(())
            });
            
            // Count triage candidates from known project roots
            let candidate_patterns = ["node_modules", "dist", "target", ".venv", "__pycache__", ".next", "build"];
            if let Ok(mut stmt) = conn.prepare("SELECT root_path FROM projects") {
                let roots: Vec<String> = stmt.query_map([], |r| r.get(0))
                    .map(|mapped| mapped.flatten().collect())
                    .unwrap_or_default();
                for root in roots {
                    for pattern in candidate_patterns {
                        let candidate = std::path::PathBuf::from(&root).join(pattern);
                        if candidate.exists() {
                            sweeper_queue_total += 1;
                            let has_lockfile = candidate.join("package-lock.json").exists()
                                || candidate.join("yarn.lock").exists()
                                || candidate.join("Cargo.lock").exists();
                            if has_lockfile {
                                sweeper_nuke_eligible += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    if is_json {
        println!(
            "{}",
            serde_json::json!({
                "daemon": {
                    "running": daemon_running,
                    "pid": daemon_pid,
                    "uptime_secs": uptime
                },
                "projects_indexed": {
                    "active": projects_active,
                    "idle": projects_idle
                },
                "sentinel_unreviewed": sentinel_unreviewed,
                "sweeper_queue_total": sweeper_queue_total,
                "sweeper_nuke_eligible": sweeper_nuke_eligible,
                "archivist_backend": archivist_backend,
                "inference_backend": inference_backend
            })
        );
    } else {
        println!("Sprawl Status ─────────────────────────────────────");
        
        if daemon_running {
            if let Some(pid) = daemon_pid {
                println!("  Daemon:           Running (PID {}, Uptime: {}s)", pid, uptime);
            } else {
                println!("  Daemon:           Running");
            }
            println!("  Daemon IPC:       Connected");
        } else {
            println!("  Daemon:           Not running");
            println!("  Daemon IPC:       Disconnected");
        }

        println!("  Projects indexed: {} active, {} idle", projects_active, projects_idle);
        println!("  Sentinel inbox:   {} unreviewed secrets", sentinel_unreviewed);
        println!("  Sweeper queue:    {} items ({} nuke-eligible)", sweeper_queue_total, sweeper_nuke_eligible);
        
        let archivist_str = if archivist_backend == "mock" { "Mock backend (real-archivist not built)" } else { "Real backend" };
        let inference_str = if inference_backend == "mock" { "Mock backend (real-inference not built)" } else { "Real backend" };
        
        println!("  Archivist:        {}", archivist_str);
        println!("  Inference:        {}", inference_str);
        println!("─────────────────────────────────────────────────────");
    }

    Ok(())
}
