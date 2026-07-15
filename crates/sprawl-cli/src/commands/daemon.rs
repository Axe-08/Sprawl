use clap::Args;
use clap::Subcommand;
use sprawl_core::Result;

#[derive(Args)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub action: DaemonAction,
}

#[derive(Subcommand)]
pub enum DaemonAction {
    /// Start the background watcher daemon
    Start {
        /// Automatically start the background indexer
        #[arg(long)]
        auto_index: bool,
    },
    /// Stop the running daemon
    Stop {
        /// Send SIGKILL instead of SIGTERM
        #[arg(long)]
        force: bool,
    },
    /// Show daemon status
    Status,
}

pub fn handle(args: &DaemonArgs, is_json: bool) -> Result<()> {
    match &args.action {
        DaemonAction::Start { auto_index } => {
            let ctx = sprawl_daemon::process::DaemonContext::new()?;
            if !is_json {
                println!("Starting daemon...");
            }
            
            let auto_index_flag = *auto_index;
            
            ctx.start(move || {
                let rt = tokio::runtime::Handle::current();
                
                rt.block_on(async {
                    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                    let data_dir = std::path::PathBuf::from(&home).join(".sprawl").join("archivist");
                    
                    #[cfg(feature = "real-archivist")]
                    let archivist_result = sprawl_archivist::Archivist::new_real(&data_dir).await;
                    #[cfg(not(feature = "real-archivist"))]
                    let archivist_result: sprawl_archivist::Result<_> = Ok(sprawl_archivist::Archivist::new(std::sync::Arc::new(sprawl_dev::MockDatabase), std::sync::Arc::new(sprawl_dev::MockEmbedder)));
                    
                    let mut archivist = match archivist_result {
                        Ok(a) => a,
                        Err(e) => {
                            tracing::error!("Failed to init archivist: {}", e);
                            return;
                        }
                    };

                    if auto_index_flag {
                        tracing::info!("Starting background indexer on daemon boot");
                        if let Err(e) = archivist.start_background_indexer(sprawl_archivist::SysRamMonitor) {
                            tracing::error!("Failed to start indexer: {}", e);
                        }
                    }

                    let archivist = std::sync::Arc::new(archivist);

                    let sentinel_data_dir = std::path::PathBuf::from(&home).join(".sprawl").join("sentinel");
                    let keyring = Box::new(sprawl_sentinel::scanner::OsKeyringStore::new("sprawl-secret-store"));
                    let ledger_path = std::path::PathBuf::from(&home).join(".sprawl").join("ledger.sqlite");
                    let conn = rusqlite::Connection::open(&ledger_path).expect("Failed to open ledger");
                    let _ = conn.execute(
                        "CREATE TABLE IF NOT EXISTS secrets (
                            id TEXT PRIMARY KEY,
                            source_file TEXT NOT NULL,
                            classification TEXT NOT NULL,
                            key_hash TEXT NOT NULL,
                            discovered_at TEXT NOT NULL,
                            keyring_ref TEXT NOT NULL
                        )",
                        []
                    );
                    let _ = conn.execute(
                        "CREATE TABLE IF NOT EXISTS ambiguous_secrets (
                            id TEXT PRIMARY KEY,
                            raw_value TEXT NOT NULL,
                            filepath TEXT NOT NULL,
                            status TEXT NOT NULL
                        )",
                        []
                    );
                    let ledger = Box::new(sprawl_sentinel::scanner::SqliteLedgerStore::new(conn));
                    
                    let sentinel = std::sync::Arc::new(sprawl_sentinel::scanner::SentinelScanner::new(vec![], keyring, ledger));

                    let ledger_path = std::path::PathBuf::from(&home).join(".sprawl").join("ledger.sqlite");
                    if let Err(e) = sprawl_daemon::run_daemon_loop(archivist, sentinel, ledger_path).await {
                        tracing::error!("Daemon loop failed: {}", e);
                    }
                });
                Ok(())
            })?;
        }
        DaemonAction::Stop { force: _ } => {
            let ctx = sprawl_daemon::process::DaemonContext::new()?;
            ctx.stop()?;
            if !is_json {
                println!("Daemon stopped.");
            }
        }
        DaemonAction::Status => {
            // we will need to read pid file and maybe check process
            let data_dir = sprawl_core::platform::sprawl_data_dir()?;
            let pid_file = data_dir.join("sprawl.pid");

            let is_running = if pid_file.exists() {
                let pid_str = std::fs::read_to_string(&pid_file).unwrap_or_default();
                let pid = pid_str.trim().parse::<u32>().unwrap_or(0);

                #[cfg(unix)]
                {
                    unsafe { libc::kill(pid as i32, 0) == 0 }
                }
                #[cfg(windows)]
                {
                    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
                    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
                    
                    unsafe {
                        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                        if handle != 0 && handle != INVALID_HANDLE_VALUE {
                            let mut exit_code: u32 = 0;
                            let res = windows_sys::Win32::System::Threading::GetExitCodeProcess(handle, &mut exit_code);
                            CloseHandle(handle);
                            res != 0 && exit_code == 259 // 259 is STILL_ACTIVE
                        } else {
                            false
                        }
                    }
                }
            } else {
                false
            };

            if is_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": if is_running { "running" } else { "stopped" }
                    })
                );
            } else {
                if is_running {
                    println!("Daemon is running.");
                } else {
                    println!("Daemon not running.");
                }
            }
        }
    }
    Ok(())
}
