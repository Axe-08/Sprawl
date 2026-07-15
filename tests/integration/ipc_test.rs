use sprawl_daemon::{IpcClient, IpcRequest, IpcResponse, IpcServer};
use std::time::Duration;

#[tokio::test]
#[cfg(unix)]
async fn test_ipc_communication() {
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", temp_dir.path());

    // Initialize Server
    let server = IpcServer::new().unwrap();
    let listener = server.bind().await.unwrap();

    // Start server loop in background
    let server_task = tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = vec![0; 8192];
            if let Ok(n) = socket.read(&mut buf).await {
                let req_str = String::from_utf8_lossy(&buf[..n]);
                if let Ok(req) = serde_json::from_str::<IpcRequest>(&req_str) {
                    match req {
                        IpcRequest::Search { query, .. } => {
                            if query == "test" {
                                let resp = IpcResponse::SearchResults(vec![]);
                                let resp_str = serde_json::to_string(&resp).unwrap();
                                let _ = socket.write_all(resp_str.as_bytes()).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    // Let server start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Client connects and sends request
    let client = IpcClient::new().unwrap();
    let req = IpcRequest::Search {
        query: "test".to_string(),
        top_k: 5,
    };

    let resp = client.send_request(&req).await.unwrap();
    match resp {
        IpcResponse::SearchResults(results) => {
            assert!(results.is_empty());
        }
        _ => panic!("Unexpected response"),
    }

    server_task.abort();
}
