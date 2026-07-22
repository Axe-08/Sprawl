use serde::{Deserialize, Serialize};
use sprawl_core::platform::sprawl_data_dir;
use sprawl_core::Result;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectEntry {
    pub id: String,
    pub root_path: String,
    pub status: String,
    pub ecosystem: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IpcRequest {
    Ping,
    Search { query: String, top_k: usize },
    Ask { query: String },
    GetSentinelInbox,
    SentinelAccept { id: uuid::Uuid },
    SentinelReject { id: uuid::Uuid },
    BatchClassify { secrets: Vec<sprawl_sentinel::llm::DiscoveredSecret> },
    StartIndexer,
    /// Poll the daemon for the current indexer progress
    IndexStatus,
    /// Register a directory as an active project
    RegisterProject { path: String },
    /// Set a project to idle (soft remove); set hard=true to fully delete from ledger
    UnregisterProject { path: String, hard: bool },
    /// List all projects in the ledger
    ListProjects,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IpcResponse {
    Pong { pid: u32, uptime_secs: u64 },
    SearchResults(Vec<sprawl_archivist::SearchResult>),
    AskResult(String),
    SentinelInbox(Vec<sprawl_sentinel::llm::DiscoveredSecret>),
    BatchClassifyResult(Vec<(uuid::Uuid, sprawl_sentinel::classify::SecretClassification)>),
    /// Live indexer progress
    IndexProgress {
        files_indexed: u64,
        files_total: u64,
        current_file: String,
        is_running: bool,
    },
    /// List of registered projects
    Projects(Vec<ProjectEntry>),
    /// Newly registered project ID
    ProjectRegistered { id: String },
    Ok,
    Error(String),
}

pub struct IpcServer {
    pub socket_path: PathBuf,
}

impl IpcServer {
    pub fn new() -> Result<Self> {
        let path = sprawl_data_dir()?.join("sprawl.sock");
        Ok(Self { socket_path: path })
    }

    #[cfg(unix)]
    pub async fn bind(&self) -> Result<tokio::net::UnixListener> {
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path).map_err(sprawl_core::SprawlError::Io)?;
        }
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                sprawl_core::SprawlError::Other(format!("Failed to create IPC directory: {}", e))
            })?;
        }
        use std::os::unix::fs::PermissionsExt;
        use tokio::net::UnixListener;
        let listener = UnixListener::bind(&self.socket_path)
            .map_err(|e| sprawl_core::SprawlError::Other(format!("IPC Bind failed: {}", e)))?;
        let mut perms = std::fs::metadata(&self.socket_path)
            .map_err(sprawl_core::SprawlError::Io)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&self.socket_path, perms)
            .map_err(sprawl_core::SprawlError::Io)?;
        Ok(listener)
    }

    #[cfg(windows)]
    pub async fn bind(&self) -> Result<()> {
        // Windows Named Pipes stub
        Ok(())
    }
}

pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    pub fn new() -> Result<Self> {
        let path = sprawl_data_dir()?.join("sprawl.sock");
        Ok(Self { socket_path: path })
    }

    #[cfg(unix)]
    pub async fn send_request(&self, req: &IpcRequest) -> Result<IpcResponse> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::UnixStream::connect(&self.socket_path).await
            .map_err(|e| sprawl_core::SprawlError::Other(format!("IPC connect failed: {}", e)))?;
        
        let req_json = serde_json::to_string(req)
            .map_err(|e| sprawl_core::SprawlError::Other(format!("IPC serialize failed: {}", e)))? + "\n";
        
        stream.write_all(req_json.as_bytes()).await
            .map_err(|e| sprawl_core::SprawlError::Other(format!("IPC write failed: {}", e)))?;
        
        let mut buf = vec![0; 1024 * 1024]; // 1MB buffer for large JSON responses
        let n = stream.read(&mut buf).await
            .map_err(|e| sprawl_core::SprawlError::Other(format!("IPC read failed: {}", e)))?;
        
        let resp_str = String::from_utf8_lossy(&buf[..n]);
        let resp: IpcResponse = serde_json::from_str(&resp_str)
            .map_err(|e| sprawl_core::SprawlError::Other(format!("IPC deserialize failed: {}", e)))?;
        
        Ok(resp)
    }

    #[cfg(windows)]
    pub async fn send_request(&self, _req: &IpcRequest) -> Result<IpcResponse> {
        // Stub for Windows
        Err(sprawl_core::SprawlError::Other("Windows IPC not implemented".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(unix)]
    async fn test_ipc_socket_creation_and_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", temp_dir.path());

        let server = IpcServer::new().unwrap();
        assert!(server.bind().await.is_ok());

        let metadata = std::fs::metadata(&server.socket_path).unwrap();
        let mode = metadata.permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "Socket permissions must be strictly 0600"
        );
    }
}
