use clap::Args;
use sprawl_core::Result;
use std::os::unix::net::UnixStream;

#[derive(Args)]
pub struct StatusArgs {}

pub fn handle(_args: &StatusArgs, is_json: bool) -> Result<()> {
    let mut daemon_running = false;
    let mut daemon_pid = None;

    // Check daemon status via IPC
    // (We recreate the logic from daemon.rs status check here for completeness)
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let socket_path = std::path::PathBuf::from(home).join(".sprawl").join("sprawl.sock");
    
    if socket_path.exists() {
        if let Ok(_) = UnixStream::connect(&socket_path) {
            daemon_running = true;
            // TODO: In a real implementation we'd send a Ping and get the PID back.
            // For now we'll just say we don't know the PID if we just connect.
            
            // Try to read pid file as a fallback
            let pid_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
                .join(".sprawl")
                .join("sprawl.pid");
            if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    daemon_pid = Some(pid);
                }
            }
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
                    "pid": daemon_pid
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
                println!("  Daemon:           Running (PID {})", pid);
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
