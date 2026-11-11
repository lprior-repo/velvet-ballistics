// ── 6. handle_readable send_response error paths ─────────────────────────────

use crate::server::helpers::test_hooks;

struct ForcePostcardFailGuard;
impl Drop for ForcePostcardFailGuard {
    fn drop(&mut self) {
        test_hooks::FORCE_POSTCARD_FAIL.set(false);
    }
}

#[test]
fn handle_readable_propagates_send_response_error_for_bad_header() {
    let path = temp_socket_path("read_bad_header_send_fail");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    test_hooks::FORCE_POSTCARD_FAIL.set(true);
    let _guard = ForcePostcardFailGuard;

    // Send a bad header with valid magic so early magic validation passes and
    // handle_readable reaches the frame-decode error response branch.
    let mut bad_header = [0u8; IPC_HEADER_LEN];
    bad_header[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    bad_header[4..6].copy_from_slice(&99u16.to_le_bytes());
    bad_header[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    client.write_all(&bad_header).expect("write bad header");
    client.flush().expect("flush");

    // Process the readable event — send_response fails, so poll_once returns Err
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
    assert!(
        matches!(result, Err(IpcServerError::ResponseEncodeFailed)),
        "poll_once should propagate ResponseEncodeFailed, got {:?}",
        result
    );

    // Client is NOT removed because poll_once returned Err before the removal
    assert_eq!(
        server.client_count(),
        1,
        "client should remain after poll_once errors"
    );
}

#[test]
fn handle_readable_propagates_send_response_error_for_valid_frame() {
    let path = temp_socket_path("read_valid_send_fail");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    test_hooks::FORCE_POSTCARD_FAIL.set(true);
    let _guard = ForcePostcardFailGuard;

    // Send a valid Health frame
    let frame = build_frame(IpcCommand::Health, 1, &[]);
    client.write_all(&frame).expect("write health frame");
    client.flush().expect("flush");

    // Process the readable event — send_response fails, so poll_once returns Err
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
    assert!(
        matches!(result, Err(IpcServerError::ResponseEncodeFailed)),
        "poll_once should propagate ResponseEncodeFailed, got {:?}",
        result
    );

    // Client is NOT removed because poll_once returned Err before the removal
    assert_eq!(
        server.client_count(),
        1,
        "client should remain after poll_once errors"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Additional coverage tests for impl_.rs edge cases
// ══════════════════════════════════════════════════════════════════════════════

// ── 1. handle_readable when client is not in map ─────────────────────────────

#[test]
fn handle_readable_returns_true_when_client_missing() {
    let path = temp_socket_path("read_missing");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let result = server.handle_readable(999, &mut runtime, None);
    assert_eq!(
        result,
        Ok(true),
        "handle_readable should return Ok(true) when client is missing"
    );
}

// ── 2. handle_writable when client is not in map ─────────────────────────────

#[test]
fn handle_writable_returns_true_when_client_missing() {
    let path = temp_socket_path("write_missing");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");

    let result = server.handle_writable(999);
    assert_eq!(
        result,
        Ok(true),
        "handle_writable should return Ok(true) when client is missing"
    );
}

// ── 3. handle_writable when write_buffer is empty ────────────────────────────

#[test]
fn handle_writable_returns_false_when_write_buffer_empty() {
    let path = temp_socket_path("write_empty");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");

    let _client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    let result = server.handle_writable(1);
    assert_eq!(
        result,
        Ok(false),
        "handle_writable should return Ok(false) when write_buffer is empty"
    );
}

// ── 4. poll_once_with_resolver when test_poll_result is set ──────────────────

#[test]
fn poll_once_with_resolver_uses_test_poll_result_when_set() {
    let path = temp_socket_path("test_poll_result");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    server.set_test_poll_once_result(Ok(false));

    let result = server.poll_once_with_resolver(&mut runtime, Some(Duration::ZERO), None);
    assert_eq!(
        result,
        Ok(false),
        "poll_once_with_resolver should return test_poll_result when set"
    );
}

// ── 5. handle_readable when read buffer exceeds max ──────────────────────────

#[test]
fn server_rejects_client_when_read_buffer_exceeds_max() {
    let path = temp_socket_path("read_buffer_overflow");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    // Send a valid Health frame (24 bytes header, 0 bytes payload)
    let frame = build_frame(IpcCommand::Health, 1, &[]);
    client.write_all(&frame).expect("write health frame");

    // Send garbage in chunks, interleaving with server polls
    let _max_buffer = IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT.get();
    let chunk = [0u8; 4096];
    let mut total_written = 0usize;

    // Process the health frame first
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process health frame");

    let mut poll_count = 0;
    let max_polls = 300;
    while server.client_count() > 0 && poll_count < max_polls {
        // Try to write a chunk to the client
        match client.write(&chunk) {
            Ok(n) => total_written += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("client write failed: {e:?}"),
        }

        match server.poll_once(&mut runtime, Some(Duration::from_millis(50))) {
            Ok(_) => {}
            Err(IpcServerError::ReadBufferTooLarge) => break,
            Err(e) => panic!("unexpected error: {e:?}"),
        }
        poll_count += 1;
    }

    assert!(
        server.client_count() == 0 || poll_count < max_polls,
        "client should be removed or ReadBufferTooLarge should be raised within {max_polls} polls, total_written={total_written}"
    );
}

// ── serve_ipc returns Ok(false) when poll_once indicates shutdown ─────────────

#[test]
fn serve_ipc_returns_ok_false_when_poll_once_indicates_shutdown() {
    let path = temp_socket_path("serve_shutdown");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    server.set_test_poll_once_result(Ok(false));

    let result = serve_ipc(&mut server, &mut runtime, Some(Duration::ZERO));

    assert_eq!(
        result,
        Ok(false),
        "serve_ipc should return Ok(false) when poll_once indicates shutdown"
    );
}
