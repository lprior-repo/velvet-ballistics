// ── bind tests ──────────────────────────────────────────────────────────────

#[test]
fn bind_creates_server_at_new_socket_path() {
    let path = temp_socket_path("bind_new");
    let _cleanup = CleanupPath(&path);
    let result = IpcServer::bind(&path);

    let Ok(_) = result else {
        panic!("bind to fresh path should succeed")
    };
    assert!(path.exists(), "socket file should exist after bind");
}

#[test]
fn bind_preserves_existing_regular_file() -> Result<(), Box<dyn std::error::Error>> {
    let path = temp_socket_path("bind_preserves_file");
    let _cleanup = CleanupPath(&path);
    std::fs::write(&path, b"owned contents")?;
    assert!(matches!(
        IpcServer::bind(&path),
        Err(IpcServerError::BindFailed { .. })
    ));
    assert_eq!(std::fs::read(&path)?, b"owned contents");
    Ok(())
}

#[test]
fn bind_to_nested_directory_fails() {
    let path = std::env::temp_dir()
        .join("vb_ipc_nonexistent_dir_test")
        .join("sock");
    let result = IpcServer::bind(&path);

    let Err(_) = result else {
        panic!("bind to nonexistent directory should fail")
    };
}

// ── poll_once tests ─────────────────────────────────────────────────────────

#[test]
fn poll_once_returns_ok_when_no_events() {
    let path = temp_socket_path("poll_no_events");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    // Poll with zero timeout: no clients connected, should return immediately.
    let result = server.poll_once(&mut runtime, Some(Duration::ZERO));

    let Ok(continuing) = result else {
        panic!("poll_once with zero timeout should succeed")
    };
    assert!(continuing, "server should indicate continue (not shutdown)");
}

#[test]
fn poll_once_accepts_client_connection() {
    let path = temp_socket_path("poll_accept");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let _client = make_client(&path);

    // Poll to accept the client.
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));

    assert!(
        result.is_ok(),
        "poll_once should succeed after client connects"
    );
}

#[test]
fn poll_once_with_resolver_returns_ok_without_resolver() {
    let path = temp_socket_path("poll_resolver_none");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let result = server.poll_once_with_resolver(&mut runtime, Some(Duration::ZERO), None);

    assert!(
        result.is_ok(),
        "poll_once_with_resolver with None resolver should succeed"
    );
}

