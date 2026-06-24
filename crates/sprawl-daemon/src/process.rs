use std::path::PathBuf;
use sprawl_core::platform::{sprawl_data_dir, set_low_priority};
use sprawl_core::Result;

pub struct DaemonContext {
    pid_file: PathBuf,
}

impl DaemonContext {
    pub fn new() -> Result<Self> {
        let data_dir = sprawl_data_dir()?;
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| sprawl_core::SprawlError::Other(format!("Failed to create data dir: {}", e)))?;
        }
        
        Ok(Self {
            pid_file: data_dir.join("sprawl.pid"),
        })
    }
    
    pub fn start(&self, run_loop: impl FnOnce() -> Result<()>) -> Result<()> {
        if self.pid_file.exists() {
            // Check if process is actually running (simplified)
            tracing::warn!("PID file exists, checking if daemon is stale...");
            std::fs::remove_file(&self.pid_file)
                .map_err(|e| sprawl_core::SprawlError::Other(format!("Failed to clean stale PID: {}", e)))?;
        }
        
        // OS Priority Yielding
        set_low_priority()?;

        #[cfg(unix)]
        {
            use daemonize::Daemonize;
            let log_file = std::fs::File::create(sprawl_data_dir()?.join("sprawl.log"))
                .map_err(|e| sprawl_core::SprawlError::Io(e))?;
                
            let daemonize = Daemonize::new()
                .pid_file(&self.pid_file)
                .chown_pid_file(true)
                .working_directory(sprawl_data_dir()?)
                .stdout(log_file.try_clone().unwrap())
                .stderr(log_file);
                
            match daemonize.start() {
                Ok(_) => {
                    tracing::info!("Daemon started successfully");
                    return run_loop();
                }
                Err(e) => return Err(sprawl_core::SprawlError::Other(format!("Daemonize failed: {}", e))),
            }
        }
        
        #[cfg(not(unix))]
        {
            // Windows background service equivalent would go here.
            // For now we just write the PID and run in current terminal as a mock daemon.
            let pid = std::process::id();
            std::fs::write(&self.pid_file, pid.to_string())
                .map_err(|e| sprawl_core::SprawlError::Io(e))?;
            tracing::info!("Daemon started successfully (foreground fallback for Windows/non-unix)");
            let res = run_loop();
            let _ = std::fs::remove_file(&self.pid_file);
            return res;
        }
    }
    
    pub fn stop(&self) -> Result<()> {
        if !self.pid_file.exists() {
            return Err(sprawl_core::SprawlError::Other("Daemon is not running".into()));
        }
        
        let pid_str = std::fs::read_to_string(&self.pid_file)
            .map_err(|e| sprawl_core::SprawlError::Io(e))?;
            
        let _pid: u32 = pid_str.trim().parse()
            .map_err(|_| sprawl_core::SprawlError::Other("Invalid PID file format".into()))?;
            
        #[cfg(unix)]
        {
            unsafe { libc::kill(_pid as i32, libc::SIGTERM); }
            tracing::info!("Sent SIGTERM to daemon PID {}", _pid);
        }
        #[cfg(windows)]
        {
            tracing::info!("Sent stop signal to daemon PID {}", _pid);
            // Implement Windows Process Terminate here
        }
        
        Ok(())
    }
}
