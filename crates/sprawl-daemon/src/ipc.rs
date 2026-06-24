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

    pub async fn bind(&self) -> Result<()> {
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .map_err(|e| sprawl_core::SprawlError::Io(e))?;
        }

        #[cfg(unix)]
        {
            use tokio::net::UnixListener;
            use std::os::unix::fs::PermissionsExt;
            
            let _listener = UnixListener::bind(&self.socket_path)
                .map_err(|e| sprawl_core::SprawlError::Other(format!("IPC Bind failed: {}", e)))?;
                
            let mut perms = std::fs::metadata(&self.socket_path)
                .map_err(|e| sprawl_core::SprawlError::Io(e))?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&self.socket_path, perms)
                .map_err(|e| sprawl_core::SprawlError::Io(e))?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(unix)]
    async fn test_ipc_socket_creation_and_permissions() {
        use std::os::unix::fs::PermissionsExt;
        
        // Mock the data dir by temporarily setting HOME to a tempdir
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", temp_dir.path());
        
        let server = IpcServer::new().unwrap();
        assert!(server.bind().await.is_ok());
        
        // Validate 0600 permissions
        let metadata = std::fs::metadata(&server.socket_path).unwrap();
        let mode = metadata.permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "Socket permissions must be strictly 0600");
    }
}
