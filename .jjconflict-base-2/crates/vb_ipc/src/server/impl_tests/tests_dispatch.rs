// ── serve_ipc dispatch function tests ───────────────────────────────────────

#[test]
fn serve_ipc_delegates_to_poll_once() {
    let path = temp_socket_path("serve_ipc_basic");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let result = serve_ipc(&mut server, &mut runtime, Some(Duration::ZERO));

    assert_eq!(
        result,
        Ok(true),
        "serve_ipc with no events should return Ok(true)"
    );
}

// ── client accept + health command round-trip ───────────────────────────────

#[test]
fn server_processes_health_command_from_client() {
    let path = temp_socket_path("health_cmd");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);

    // Accept the client.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept poll should succeed");

    // Send a Health frame.
    let frame = build_frame(IpcCommand::Health, 1, &[]);
    client
        .write_all(&frame)
        .expect("client should write health frame");
    client.flush().expect("client should flush");

    // Server processes the readable event.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process poll should succeed");

    // Read the response header.
    let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);
    let response_header = response_header_bytes.expect("should read response header");

    // Verify magic and version.
    let magic = u32::from_le_bytes(
        response_header
            .get(..4)
            .map(|s| <[u8; 4]>::try_from(s).ok())
            .flatten()
            .unwrap_or([0; 4]),
    );
    assert_eq!(magic, IPC_MAGIC, "response should have valid magic");

    // Decode response header to get payload length.
    let payload_len = u32::from_le_bytes(
        response_header
            .get(20..24)
            .map(|s| <[u8; 4]>::try_from(s).ok())
            .flatten()
            .unwrap_or([0; 4]),
    );

    // Read and decode response payload.
    let payload_len_usize = match usize::try_from(payload_len) {
        Ok(v) => v,
        Err(_) => return,
    };
    if payload_len_usize > 0 {
        let response_payload = read_exact_timeout(&mut client, payload_len_usize);
        let Ok(payload) = response_payload else {
            panic!("should read response payload")
        };
        let decoded: Result<IpcResponse, _> = postcard::from_bytes(&payload);
        match decoded {
            Ok(IpcResponse::Healthy) => {}
            Ok(other) => {
                assert!(false, "expected Healthy response, got {other:?}");
            }
            Err(e) => {
                assert!(false, "response payload decode failed: {e}");
            }
        }
    }
    assert_eq!(
        server.client_count(),
        1,
        "valid health frame must keep the client connected"
    );
}

// ── client disconnect handling ───────────────────────────────────────────────

#[test]
fn server_handles_client_disconnect_gracefully() {
    let path = temp_socket_path("client_disconnect");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    {
        let _client = make_client(&path);
        // Accept the client.
        server
            .poll_once(&mut runtime, Some(Duration::from_millis(100)))
            .expect("accept poll should succeed");
        // Client goes out of scope and disconnects here.
    }

    // Poll again to detect the disconnect.
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
    assert!(
        result.is_ok(),
        "poll after client disconnect should succeed"
    );
    assert_eq!(
        server.client_count(),
        0,
        "client should be removed after disconnect"
    );
}

#[test]
fn slow_client_partial_frame_keeps_read_buffer_bounded() {
    let path = temp_socket_path("slow_client_partial_frame");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();
    let mut client = make_client(&path);

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept poll should succeed");

    let header = IpcFrameHeader::new(
        IpcCommand::Health,
        0,
        77,
        match u32::try_from(MaxPayloadBytes::DEFAULT.get()) {
            Ok(value) => value,
            Err(_) => return,
        },
    );
    let Ok(header_bytes) = header.encode() else {
        return;
    };
    client
        .write_all(&header_bytes)
        .expect("client should write partial header-only frame");
    client.flush().expect("client should flush");

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("server should retain bounded partial frame");

    assert_eq!(
        server.clients.len(),
        1,
        "slow client with partial bounded frame should remain connected"
    );
    let Some(connection) = server.clients.values().next() else {
        return;
    };
    assert_eq!(
        connection.read_buffer.len(),
        IPC_HEADER_LEN,
        "server must keep only received bytes, not allocate declared payload"
    );
    assert!(
        connection.write_buffer.is_empty(),
        "partial frame must not produce a response before payload arrives"
    );
}

#[test]
fn slow_client_oversized_frame_disconnects_without_unbounded_growth() {
    let path = temp_socket_path("slow_client_oversized_frame");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();
    let mut client = make_client(&path);

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept poll should succeed");

    let oversized = match u32::try_from(MaxPayloadBytes::DEFAULT.get()) {
        Ok(value) => match value.checked_add(1) {
            Some(next) => next,
            None => return,
        },
        Err(_) => return,
    };
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 78, oversized);
    let Ok(header_bytes) = header.encode() else {
        return;
    };
    client
        .write_all(&header_bytes)
        .expect("client should write oversized header");
    client.flush().expect("client should flush");

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("server should reject oversized frame");

    assert!(
        server.clients.is_empty(),
        "oversized slow-client frame must be rejected by disconnecting the client"
    );
}

// ── invalid frame header handling ───────────────────────────────────────────

#[test]
fn server_disconnects_invalid_magic_without_response() {
    let path = temp_socket_path("bad_magic");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);

    // Accept the client.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept poll should succeed");

    // Send a frame with bad magic (0 instead of IPC_MAGIC).
    let mut bad_header = [0u8; IPC_HEADER_LEN];
    bad_header[..4].copy_from_slice(&0u32.to_le_bytes());
    bad_header[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    bad_header[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    client
        .write_all(&bad_header)
        .expect("client should write bad frame");
    client.flush().expect("client should flush");

    // Server processes the readable event.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process poll should succeed");

    assert!(
        server.clients.is_empty(),
        "invalid magic frame must disconnect the client without allocating a response"
    );
}

// ── unsupported version handling ────────────────────────────────────────────

#[test]
fn server_responds_with_error_for_unsupported_version() {
    let path = temp_socket_path("bad_version");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);

    // Accept the client.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept poll should succeed");

    // Send a frame with wrong version.
    let mut bad_header = [0u8; IPC_HEADER_LEN];
    bad_header[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    bad_header[4..6].copy_from_slice(&99u16.to_le_bytes());
    bad_header[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    client
        .write_all(&bad_header)
        .expect("client should write bad version frame");
    client.flush().expect("client should flush");

    // Server processes the readable event.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process poll should succeed");

    // Read response header.
    let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);
    assert!(
        response_header_bytes.is_ok(),
        "should read error response header"
    );

    // Read response payload.
    let response_header = response_header_bytes.expect("header");
    let payload_len = u32::from_le_bytes(
        response_header
            .get(20..24)
            .map(|s| <[u8; 4]>::try_from(s).ok())
            .flatten()
            .unwrap_or([0; 4]),
    );
    let payload_len_usize = match usize::try_from(payload_len) {
        Ok(v) => v,
        Err(_) => return,
    };
    if payload_len_usize > 0 {
        let response_payload = read_exact_timeout(&mut client, payload_len_usize);
        assert!(
            response_payload.is_ok(),
            "should read error response payload"
        );
        let payload = response_payload.expect("read payload");
        let decoded: Result<IpcResponse, _> = postcard::from_bytes(&payload);
        match decoded {
            Ok(IpcResponse::FrameError { message }) => {
                assert!(
                    message.contains("version") || message.contains("unsupported"),
                    "frame error should mention unsupported version, got '{message}'"
                );
            }
            Ok(other) => {
                assert!(false, "expected FrameError, got {other:?}");
            }
            Err(e) => {
                assert!(false, "error response decode failed: {e}");
            }
        }
    }
}

