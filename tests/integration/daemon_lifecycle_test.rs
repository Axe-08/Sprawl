use sprawl_daemon::ipc::IpcServer;

#[tokio::test]
async fn test_daemon_lifecycle() {
    // Isolated environment for the test socket
    let temp_dir = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", temp_dir.path());
    
    // Test that the daemon starts and binds its IPC socket correctly
    let server = IpcServer::new().unwrap();
    
    // Bind the socket
    server.bind().await.unwrap();

    // Verify socket exists
    let socket_path = temp_dir.path().join(".sprawl/sprawl.sock");
    assert!(socket_path.exists(), "IPC socket was not created at expected path");

    // Clean up
}
