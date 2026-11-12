// ── multiple client connections ─────────────────────────────────────────────

#[test]
fn server_accepts_multiple_clients() {
    let path = temp_socket_path("multi_client");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let client1 = make_client(&path);
    let client2 = make_client(&path);
    let client3 = make_client(&path);

    // Accept all three clients.
    for _ in 0..3 {
        server
            .poll_once(&mut runtime, Some(Duration::from_millis(100)))
            .expect("poll should succeed");
    }

    // Verify server still works (all clients connected).
    let result = server.poll_once(&mut runtime, Some(Duration::ZERO));
    let Ok(_) = result else {
        panic!("poll after multiple accepts should succeed")
    };

    // Explicitly keep clients alive until here.
    drop(client1);
    drop(client2);
    drop(client3);
}

// ── partial frame handling ──────────────────────────────────────────────────

#[test]
fn server_waits_for_complete_frame_when_partial_sent() {
    let path = temp_socket_path("partial_frame");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);

    // Accept the client.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept poll should succeed");

    // Send only partial header (fewer bytes than IPC_HEADER_LEN).
    let partial = &[0x56, 0x42, 0x4C]; // 3 bytes of magic
    client
        .write_all(partial)
        .expect("client should write partial frame");
    client.flush().expect("client should flush");

    // Server should handle the partial read without error.
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
    assert!(
        matches!(result, Ok(_)),
        "poll with partial frame should not error"
    );
}

// ── garbage payload handling ────────────────────────────────────────────────

#[test]
fn server_responds_with_error_for_garbage_payload() {
    let path = temp_socket_path("garbage_payload");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);

    // Accept the client.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept poll should succeed");

    // Send a valid Health header with a garbage payload.
    let garbage = vec![0xFF_u8; 10];
    let frame = build_frame(IpcCommand::Health, 42, &garbage);
    client
        .write_all(&frame)
        .expect("client should write garbage frame");
    client.flush().expect("client should flush");

    // Server processes the readable event.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process poll should succeed");

    // Read response header.
    let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);

    // Read response payload and verify it is not a panic.
    let response_header = response_header_bytes.expect("should read response header");
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
            matches!(response_payload, Ok(_)),
            "should read response payload"
        );
    }
}

// ── pipelined commands ──────────────────────────────────────────────────────

#[test]
fn server_processes_multiple_commands_from_same_client() {
    let path = temp_socket_path("pipelined");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);

    // Accept the client.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept poll should succeed");

    // Send two Health frames back-to-back.
    let frame1 = build_frame(IpcCommand::Health, 1, &[]);
    let frame2 = build_frame(IpcCommand::Health, 2, &[]);
    let mut pipeline = Vec::new();
    pipeline.extend_from_slice(&frame1);
    pipeline.extend_from_slice(&frame2);
    client
        .write_all(&pipeline)
        .expect("client should write pipelined frames");
    client.flush().expect("client should flush");

    // Each bounded turn dispatches at most one request per connection.
    for i in 1..=2 {
        server
            .poll_once(&mut runtime, Some(Duration::from_millis(100)))
            .expect("process poll should succeed");
        let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);
        assert!(
            response_header_bytes.is_ok(),
            "should read response header {i}"
        );
        let response_header = response_header_bytes.expect("header");
        let header_bytes: [u8; IPC_HEADER_LEN] =
            response_header.as_slice().try_into().expect("fixed header");
        let decoded =
            IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT).expect("valid header");
        assert_eq!(decoded.correlation, i);
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
            let Ok(_) = response_payload else {
                panic!("should read response payload {i}");
            };
        }
    }
}

// ── reserved non-zero field handling ────────────────────────────────────────

#[test]
fn server_responds_with_error_for_nonzero_reserved_field() {
    let path = temp_socket_path("nonzero_reserved");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);

    // Accept the client.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept poll should succeed");

    // Build a header with reserved field set to non-zero.
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    header_bytes[8..10].copy_from_slice(&0u16.to_le_bytes()); // flags
    header_bytes[10..12].copy_from_slice(&1u16.to_le_bytes()); // reserved != 0
    header_bytes[12..20].copy_from_slice(&1u64.to_le_bytes()); // correlation
    header_bytes[20..24].copy_from_slice(&0u32.to_le_bytes()); // payload_len
    client
        .write_all(&header_bytes)
        .expect("client should write nonzero reserved");
    client.flush().expect("client should flush");

    // Server processes the readable event.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process poll should succeed");

    // Read response header.
    let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);

    // Read response payload.
    let response_header = response_header_bytes.expect("should read response header");
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
        let Ok(payload) = response_payload else {
            panic!("should read response payload")
        };
        let decoded: Result<IpcResponse, _> = postcard::from_bytes(&payload);
        match decoded {
            Ok(IpcResponse::FrameError { message }) => {
                assert!(
                    message.contains("reserved") || message.contains("non-zero"),
                    "frame error should mention reserved, got '{message}'"
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

// ── error variant tests ─────────────────────────────────────────────────────

#[test]
fn ipc_server_error_bind_failed_display() {
    let err = IpcServerError::BindFailed {
        source: std::io::Error::new(std::io::ErrorKind::AddrInUse, "addr in use"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("bind failed"),
        "expected 'bind failed' in '{msg}'"
    );
}

#[test]
fn ipc_server_error_poll_failed_display() {
    let err = IpcServerError::PollFailed {
        source: std::io::Error::new(std::io::ErrorKind::Interrupted, "interrupted"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("poll failed"),
        "expected 'poll failed' in '{msg}'"
    );
}

#[test]
fn ipc_server_error_too_many_clients_display() {
    let err = IpcServerError::TooManyClients;
    let msg = err.to_string();
    assert!(
        msg.contains("too many clients"),
        "expected 'too many clients' in '{msg}'"
    );
}

// ── cleanup helper ──────────────────────────────────────────────────────────

