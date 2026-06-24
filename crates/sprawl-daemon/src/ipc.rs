use sprawl_core::platform::sprawl_data_dir;
use sprawl_core::Result;
use std::path::PathBuf;

pub struct IpcServer {
    socket_path: PathBuf,
}

impl IpcServer {
    pub fn new() -> Result<Self> {
        let path = sprawl_data_dir()?.join("sprawl.sock");
        Ok(Self { socket_path: path })
    }

    /// Set up and bind the local IPC socket/pipe
    pub async fn bind(&self) -> Result<()> {
        if self.socket_path.exists() {
            tracing::warn!("Socket file exists, cleaning up: {}", self.socket_path.display());
            std::fs::remove_file(&self.socket_path)
                .map_err(|e| sprawl_core::SprawlError::Io(e))?;
        }

        #[cfg(unix)]
        {
            use tokio::net::UnixListener;
            use std::os::unix::fs::PermissionsExt;
            
            let _listener = UnixListener::bind(&self.socket_path)
                .map_err(|e| sprawl_core::SprawlError::Other(format!("IPC Bind failed: {}", e)))?;
                
            // Apply 0600 restrictive permissions per OQ-09
            let mut perms = std::fs::metadata(&self.socket_path)
                .map_err(|e| sprawl_core::SprawlError::Io(e))?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&self.socket_path, perms)
                .map_err(|e| sprawl_core::SprawlError::Io(e))?;
                
            tracing::info!("IPC Server listening at {}", self.socket_path.display());
            
            // This is where the tokio loop would spawn to accept IPC connections 
            // returning state to TUI/CLI queries.
            // tokio::spawn(async move { ... listener.accept().await ... });
        }

        #[cfg(windows)]
        {
            tracing::info!("Windows Named Pipe setup would occur here.");
            // Named pipe \\.\pipe\sprawl using tokio::net::windows::named_pipe
        }
        
        Ok(())
    }
}
