// ── IpcResponse serialization roundtrip: Healthy ────────────────────────────

#[test]
fn ipc_response_roundtrip_healthy() {
    let original = IpcResponse::Healthy;
    let encoded = postcard::to_allocvec(&original).expect("encode Healthy");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode Healthy");
    assert_eq!(decoded, original, "Healthy roundtrip should be equal");
}

// ── IpcResponse serialization roundtrip: BadRequest ─────────────────────────

#[test]
fn ipc_response_roundtrip_bad_request() {
    let original = IpcResponse::BadRequest;
    let encoded = postcard::to_allocvec(&original).expect("encode BadRequest");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode BadRequest");
    assert_eq!(decoded, original, "BadRequest roundtrip should be equal");
}

// ── IpcResponse serialization roundtrip: RuntimeError ───────────────────────

#[test]
fn ipc_response_roundtrip_runtime_error() {
    let original = IpcResponse::RuntimeError {
        message: String::from("test error message"),
    };
    let encoded = postcard::to_allocvec(&original).expect("encode RuntimeError");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode RuntimeError");
    assert_eq!(decoded, original, "RuntimeError roundtrip should be equal");
}

// ── IpcResponse serialization roundtrip: FrameError ─────────────────────────

#[test]
fn ipc_response_roundtrip_frame_error() {
    let original = IpcResponse::FrameError {
        message: String::from("bad magic header"),
    };
    let encoded = postcard::to_allocvec(&original).expect("encode FrameError");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode FrameError");
    assert_eq!(decoded, original, "FrameError roundtrip should be equal");
}

// ── WorkflowResolutionError Display messages ────────────────────────────────

#[test]
fn workflow_resolution_error_required_display() {
    let err = WorkflowResolutionError::Required;
    let msg = err.to_string();
    assert!(
        msg.contains("resolution required"),
        "expected 'resolution required' in '{msg}'"
    );
}

#[test]
fn workflow_resolution_error_invalid_artifact_display() {
    let err = WorkflowResolutionError::InvalidArtifact;
    let msg = err.to_string();
    assert!(msg.contains("invalid"), "expected 'invalid' in '{msg}'");
}

// ══════════════════════════════════════════════════════════════════════════════
// Additional edge-case tests
// ══════════════════════════════════════════════════════════════════════════════

// ── 1. bind: fixture owner removes its stale socket before re-binding ─────────

#[test]
fn bind_requires_owner_cleanup_after_previous_server_dropped()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let path = temp_socket_path("bind_rebind");
    let _cleanup = CleanupPath(&path);
    {
        let _server = IpcServer::bind(&path)?;
        assert!(path.exists());
    }
    let before = std::fs::symlink_metadata(&path)?;
    assert!(before.file_type().is_socket());
    assert!(matches!(
        IpcServer::bind(&path),
        Err(IpcServerError::BindFailed { .. })
    ));
    let after = std::fs::symlink_metadata(&path)?;
    assert!(after.file_type().is_socket());
    assert_eq!((after.dev(), after.ino()), (before.dev(), before.ino()));
    std::fs::remove_file(&path)?;
    assert!(!path.try_exists()?);
    let _rebound = IpcServer::bind(&path)?;
    assert!(std::fs::symlink_metadata(&path)?.file_type().is_socket());
    Ok(())
}

// ── 2. bind: path is a directory, not a file ─────────────────────────────────

#[test]
fn bind_fails_when_path_is_existing_directory() {
    let dir = std::env::temp_dir().join(format!("vb_ipc_dir_test_{}", std::process::id()));
    let _dir_cleanup = CleanupDir(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");

    let result = IpcServer::bind(&dir);
    assert!(
        matches!(result, Err(IpcServerError::BindFailed { .. })),
        "bind to a directory path should fail with BindFailed, got {}",
        match result {
            Ok(_) => "Ok".to_string(),
            Err(ref e) => e.to_string(),
        }
    );
}

// ── 3. client lifecycle: connect, health, disconnect, reconnect ──────────────

#[test]
fn client_can_reconnect_after_disconnect_on_same_server() {
    let path = temp_socket_path("reconnect");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    // First client connects, sends health, gets response, then disconnects.
    {
        let mut client = make_client(&path);
        server
            .poll_once(&mut runtime, Some(Duration::from_millis(100)))
            .expect("accept first client");

        let frame = build_frame(IpcCommand::Health, 1, &[]);
        client.write_all(&frame).expect("write health frame");
        client.flush().expect("flush");

        server
            .poll_once(&mut runtime, Some(Duration::from_millis(100)))
            .expect("process health");

        // Read response header.
        let response_header = read_exact_timeout(&mut client, IPC_HEADER_LEN);
        assert!(
            response_header.is_ok(),
            "should read response header from first client"
        );

        // Client drops here.
    }

    // Poll to detect the disconnect.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("detect disconnect");

    // Second client connects on the same server socket.
    let mut client2 = make_client(&path);
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept second client");

    let frame = build_frame(IpcCommand::Health, 2, &[]);
    client2.write_all(&frame).expect("write health frame 2");
    client2.flush().expect("flush");

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process health 2");

    let response_header = read_exact_timeout(&mut client2, IPC_HEADER_LEN);
    assert!(
        response_header.is_ok(),
        "should read response header from second client"
    );
}

// ── 4. error handling: server survives client_drop_mid_frame ──────────────────

#[test]
fn server_survives_client_drop_mid_frame() {
    let path = temp_socket_path("mid_frame_drop");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept client");

    // Send only a partial header (fewer bytes than IPC_HEADER_LEN).
    let partial = &[0x56, 0x42, 0x4C, 0x54, 0x01]; // 5 bytes: magic + version byte
    client.write_all(partial).expect("write partial");
    client.flush().expect("flush");

    // Process the partial read.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process partial frame");

    // Drop the client while the server has a partial read buffered.
    drop(client);

    // Poll to detect the disconnect and verify the server does not panic.
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
    assert!(
        result.is_ok(),
        "server should survive mid-frame client drop"
    );

    // Server should still accept new clients.
    let _new_client = make_client(&path);
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
    assert!(
        result.is_ok(),
        "server should accept new client after mid-frame drop"
    );
}

// ── 5. frame encoding edge case: zero-length correlation round-trips ─────────

#[test]
fn health_command_with_zero_correlation_round_trips() {
    let path = temp_socket_path("zero_corr");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept client");

    // Send health with correlation=0.
    let frame = build_frame(IpcCommand::Health, 0, &[]);
    client.write_all(&frame).expect("write health frame");
    client.flush().expect("flush");

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process health");

    let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);

    let response_header = response_header_bytes.expect("should read response header");
    // Verify correlation field (bytes 12..20) is zero.
    let correlation = u64::from_le_bytes(
        response_header
            .get(12..20)
            .and_then(|s| <[u8; 8]>::try_from(s).ok())
            .unwrap_or([0; 8]),
    );
    assert_eq!(correlation, 0, "correlation should be zero in response");
}

// ── 6. IpcResponse variant: CommandPayloadMismatch round-trips ───────────────

#[test]
fn ipc_response_roundtrip_command_payload_mismatch() {
    let original = IpcResponse::CommandPayloadMismatch;
    let encoded = postcard::to_allocvec(&original).expect("encode CommandPayloadMismatch");
    let decoded: IpcResponse =
        postcard::from_bytes(&encoded).expect("decode CommandPayloadMismatch");
    assert_eq!(
        decoded, original,
        "CommandPayloadMismatch roundtrip should be equal"
    );
}

// ── 7. IpcResponse variant: WorkflowResolutionRequired round-trips ────────────

#[test]
fn ipc_response_roundtrip_workflow_resolution_required() {
    let original = IpcResponse::WorkflowResolutionRequired;
    let encoded = postcard::to_allocvec(&original).expect("encode WorkflowResolutionRequired");
    let decoded: IpcResponse =
        postcard::from_bytes(&encoded).expect("decode WorkflowResolutionRequired");
    assert_eq!(
        decoded, original,
        "WorkflowResolutionRequired roundtrip should be equal"
    );
}

// ── 8. IpcResponse variant: CountOutOfRange round-trips ──────────────────────

#[test]
fn ipc_response_roundtrip_count_out_of_range() {
    let original = IpcResponse::CountOutOfRange {
        actual: 65536,
        limit: 1000,
    };
    let encoded = postcard::to_allocvec(&original).expect("encode CountOutOfRange");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode CountOutOfRange");
    assert_eq!(
        decoded, original,
        "CountOutOfRange roundtrip should be equal"
    );
}

// ── 9. IpcResponse variant: PayloadError round-trips ─────────────────────────

#[test]
fn ipc_response_roundtrip_payload_error() {
    let original = IpcResponse::PayloadError {
        diagnostic: 0x3004,
        message: String::from("invalid magic header detected"),
    };
    let encoded = postcard::to_allocvec(&original).expect("encode PayloadError");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode PayloadError");
    assert_eq!(decoded, original, "PayloadError roundtrip should be equal");
}

// ── 10. IpcServerError Display: ReadBufferTooLarge ───────────────────────────

#[test]
fn ipc_server_error_read_buffer_too_large_display() {
    let err = IpcServerError::ReadBufferTooLarge;
    let msg = err.to_string();
    assert!(
        msg.contains("read buffer exceeded"),
        "expected 'read buffer exceeded' in '{msg}'"
    );
}

// ── 11. IpcServerError Display: ResponseWriteFailed ──────────────────────────

#[test]
fn ipc_server_error_response_write_failed_display() {
    let err = IpcServerError::ResponseWriteFailed {
        source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken pipe"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("response write failed"),
        "expected 'response write failed' in '{msg}'"
    );
}

// ── 12. Multiple sequential clients: three clients connect, each sends health ─

#[test]
fn sequential_clients_each_get_health_response() {
    let path = temp_socket_path("seq_clients_health");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    for i in 0..3u64 {
        let mut client = make_client(&path);

        // Accept the client.
        server
            .poll_once(&mut runtime, Some(Duration::from_millis(100)))
            .expect("accept poll should succeed");

        // Send health frame with correlation = i.
        let correlation = i.saturating_add(100);
        let frame = build_frame(IpcCommand::Health, correlation, &[]);
        client.write_all(&frame).expect("write health frame");
        client.flush().expect("flush");

        // Process the command.
        server
            .poll_once(&mut runtime, Some(Duration::from_millis(100)))
            .expect("process poll should succeed");

        // Read response.
        let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);
        assert!(
            response_header_bytes.is_ok(),
            "should read response header for client {i}"
        );
        let response_header = response_header_bytes.expect("header");

        // Verify magic.
        let magic = u32::from_le_bytes(
            response_header
                .get(..4)
                .and_then(|s| <[u8; 4]>::try_from(s).ok())
                .unwrap_or([0; 4]),
        );
        assert_eq!(
            magic, IPC_MAGIC,
            "response magic should be valid for client {i}"
        );

        // Read and verify payload.
        let payload_len = u32::from_le_bytes(
            response_header
                .get(20..24)
                .and_then(|s| <[u8; 4]>::try_from(s).ok())
                .unwrap_or([0; 4]),
        );
        let payload_len_usize = match usize::try_from(payload_len) {
            Ok(v) => v,
            Err(_) => {
                assert!(false, "payload_len overflow for client {i}");
                return;
            }
        };
        if payload_len_usize > 0 {
            let response_payload = read_exact_timeout(&mut client, payload_len_usize);
            assert!(
                response_payload.is_ok(),
                "should read response payload for client {i}"
            );
            let payload = response_payload.expect("read payload");
            let decoded: Result<IpcResponse, _> = postcard::from_bytes(&payload);
            match decoded {
                Ok(IpcResponse::Healthy) => {}
                Ok(other) => {
                    assert!(false, "expected Healthy for client {i}, got {other:?}");
                }
                Err(e) => {
                    assert!(false, "response decode failed for client {i}: {e}");
                }
            }
        }

        // Drop the client to test sequential lifecycle.
        drop(client);

        // Poll to clean up the disconnect.
        server
            .poll_once(&mut runtime, Some(Duration::from_millis(100)))
            .expect("disconnect poll should succeed");
    }
}

// ── 13. IpcPayload serialization edge case: Health payload round-trips ────────

#[test]
fn ipc_payload_health_roundtrip_via_frame() {
    let payload = IpcPayload::Health;
    let encoded = postcard::to_allocvec(&payload).expect("encode Health payload");
    let decoded: IpcPayload = postcard::from_bytes(&encoded).expect("decode Health payload");
    assert_eq!(
        decoded,
        IpcPayload::Health,
        "Health payload should round-trip"
    );
}

// ── IpcResponse serialization roundtrip: AcceptedRun ─────────────────────────

#[test]
fn ipc_response_roundtrip_accepted_run() {
    let original = IpcResponse::AcceptedRun { run_id: 42 };
    let encoded = postcard::to_allocvec(&original).expect("encode AcceptedRun");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode AcceptedRun");
    assert_eq!(decoded, original, "AcceptedRun roundtrip should be equal");
}

// ── IpcResponse serialization roundtrip: ShuttingDown ────────────────────────

#[test]
fn ipc_response_roundtrip_shutting_down() {
    let original = IpcResponse::ShuttingDown;
    let encoded = postcard::to_allocvec(&original).expect("encode ShuttingDown");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode ShuttingDown");
    assert_eq!(decoded, original, "ShuttingDown roundtrip should be equal");
}

// ── IpcResponse serialization roundtrip: TraceCount ──────────────────────────

#[test]
fn ipc_response_roundtrip_trace_count() {
    let original = IpcResponse::TraceCount { count: 12345 };
    let encoded = postcard::to_allocvec(&original).expect("encode TraceCount");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode TraceCount");
    assert_eq!(decoded, original, "TraceCount roundtrip should be equal");
}

// ── IpcResponse serialization roundtrip: Events ──────────────────────────────

#[test]
fn ipc_response_roundtrip_events_empty() {
    let original = IpcResponse::Events { events: vec![] };
    let encoded = postcard::to_allocvec(&original).expect("encode Events");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode Events");
    assert_eq!(decoded, original, "Events roundtrip should be equal");
}

#[test]
fn ipc_response_roundtrip_events_with_items() {
    let original = IpcResponse::Events {
        events: vec![
            crate::IpcTraceEvent {
                sequence: 1,
                kind: crate::IpcTraceEventKind::RunSubmitted {
                    run: RunId::new(10),
                },
            },
            crate::IpcTraceEvent {
                sequence: 2,
                kind: crate::IpcTraceEventKind::RunFinished {
                    run: RunId::new(10),
                },
            },
        ],
    };
    let encoded = postcard::to_allocvec(&original).expect("encode Events");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode Events");
    assert_eq!(decoded, original, "Events roundtrip should be equal");
}

// ── IpcResponse serialization roundtrip: Inspected ───────────────────────────

#[test]
fn ipc_response_roundtrip_inspected() {
    let original = IpcResponse::Inspected { run_id: 99 };
    let encoded = postcard::to_allocvec(&original).expect("encode Inspected");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode Inspected");
    assert_eq!(decoded, original, "Inspected roundtrip should be equal");
}

// ── IpcResponse serialization roundtrip: WorkflowResolutionUnsupported ───────

#[test]
fn ipc_response_roundtrip_workflow_resolution_unsupported() {
    let original = IpcResponse::WorkflowResolutionUnsupported;
    let encoded = postcard::to_allocvec(&original).expect("encode WorkflowResolutionUnsupported");
    let decoded: IpcResponse =
        postcard::from_bytes(&encoded).expect("decode WorkflowResolutionUnsupported");
    assert_eq!(
        decoded, original,
        "WorkflowResolutionUnsupported roundtrip should be equal"
    );
}

// ── IpcResponse serialization roundtrip: WorkflowDigestMismatch ──────────────

#[test]
fn ipc_response_roundtrip_workflow_digest_mismatch() {
    let original = IpcResponse::WorkflowDigestMismatch;
    let encoded = postcard::to_allocvec(&original).expect("encode WorkflowDigestMismatch");
    let decoded: IpcResponse =
        postcard::from_bytes(&encoded).expect("decode WorkflowDigestMismatch");
    assert_eq!(
        decoded, original,
        "WorkflowDigestMismatch roundtrip should be equal"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Coverage tests for impl_.rs branches
// ══════════════════════════════════════════════════════════════════════════════

