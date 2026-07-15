use clap::Args;
use sprawl_core::Result;

#[derive(Args)]
pub struct StatusArgs {}

pub fn handle(_args: &StatusArgs, is_json: bool) -> Result<()> {
    let mut daemon_running = false;
    let mut daemon_pid = None;

    let rt = tokio::runtime::Runtime::new()?;
    let ping_result = rt.block_on(async {
        if let Ok(client) = sprawl_daemon::IpcClient::new() {
            client.send_request(&sprawl_daemon::IpcRequest::Ping).await.ok()
        } else {
            None
        }
    });

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

    // Stubs for ledger-backed counts (M19 will wire these up)
    let projects_active = 0;
    let projects_idle = 0;
    let sentinel_unreviewed = 0;
    let sweeper_queue_total = 0;
    let sweeper_nuke_eligible = 0;

    if is_json {
        println!(
            "{}",
            serde_json::json!({
                "daemon": {
                    "running": daemon_running,
                    "pid": daemon_pid,
                    "uptime_secs": uptime
                },
                "sentinel_unreviewed": sentinel_unreviewed,
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
