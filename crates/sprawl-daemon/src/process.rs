use sprawl_core::platform::{set_low_priority, sprawl_data_dir};
use sprawl_core::Result;
use std::path::PathBuf;

pub struct DaemonContext {
    pid_file: PathBuf,
}

impl DaemonContext {
    pub fn new() -> Result<Self> {
        let data_dir = sprawl_data_dir()?;
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir).map_err(|e| {
                sprawl_core::SprawlError::Other(format!("Failed to create data dir: {}", e))
            })?;
        }

        Ok(Self {
            pid_file: data_dir.join("sprawl.pid"),
        })
    }

    pub fn start(&self, run_loop: impl FnOnce() -> Result<()>) -> Result<()> {
        if self.pid_file.exists() {
            tracing::warn!("PID file exists, checking if daemon is stale...");
            let _ = std::fs::remove_file(&self.pid_file);
        }

        set_low_priority()?;

        let pid = std::process::id();
        std::fs::write(&self.pid_file, pid.to_string()).map_err(sprawl_core::SprawlError::Io)?;
        tracing::info!("Daemon started successfully (foreground blocking)");

        let res = run_loop();

        let _ = std::fs::remove_file(&self.pid_file);
        res
    }

    pub fn stop(&self) -> Result<()> {
        if !self.pid_file.exists() {
            return Err(sprawl_core::SprawlError::Other(
                "Daemon is not running".into(),
            ));
        }

        let pid_str =
            std::fs::read_to_string(&self.pid_file).map_err(sprawl_core::SprawlError::Io)?;

        let _pid: u32 = pid_str
            .trim()
            .parse()
            .map_err(|_| sprawl_core::SprawlError::Other("Invalid PID file format".into()))?;

        #[cfg(unix)]
        {
            unsafe {
                libc::kill(_pid as i32, libc::SIGTERM);
            }
            tracing::info!("Sent SIGTERM to daemon PID {}", _pid);
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
            use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
            
            unsafe {
                let handle = OpenProcess(PROCESS_TERMINATE, 0, _pid);
                if handle != 0 && handle != INVALID_HANDLE_VALUE {
                    let res = TerminateProcess(handle, 1);
                    CloseHandle(handle);
                    if res == 0 {
                        return Err(sprawl_core::SprawlError::Other("Failed to terminate Windows process".into()));
                    }
                    tracing::info!("Terminated Windows daemon PID {}", _pid);
                } else {
                    return Err(sprawl_core::SprawlError::Other("Could not open Windows process for termination".into()));
                }
            }
        }

        Ok(())
    }
}
