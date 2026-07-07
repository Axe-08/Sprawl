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
    Start,
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
        DaemonAction::Start => {
            let ctx = sprawl_daemon::process::DaemonContext::new()?;
            if !is_json {
                println!("Starting daemon...");
            }
            ctx.start(|| {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(async {
                    if let Err(e) = sprawl_daemon::run_daemon_loop().await {
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
                    true // mockup for windows for now
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
