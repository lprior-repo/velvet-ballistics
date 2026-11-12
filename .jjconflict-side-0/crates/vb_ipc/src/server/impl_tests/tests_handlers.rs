use super::impl_::MAX_CLIENTS;

// ── 1. accept_client when max clients reached ────────────────────────────────

#[test]
fn poll_rejects_extra_client_without_stopping_when_at_capacity() {
    let path = temp_socket_path("max_clients");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();
    let mut clients = Vec::with_capacity(MAX_CLIENTS);

    for _ in 0..MAX_CLIENTS {
        clients.push(make_client(&path));
        server
            .poll_once(&mut runtime, Some(Duration::from_millis(50)))
            .expect("accept should succeed");
    }

    let mut extra = make_client(&path);
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
    assert_eq!(result, Ok(true));
    assert_eq!(server.client_count(), MAX_CLIENTS);
    assert_eq!(server.rejected_clients(), 1);
    assert_eq!(extra.read(&mut [0; 1]).expect("rejected peer closes"), 0);
}

// ── 2. handle_readable when WouldBlock ───────────────────────────────────────

#[test]
fn handle_readable_returns_false_on_would_block() {
    let path = temp_socket_path("read_would_block");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    client
        .write_all(&[0x56, 0x42, 0x4C])
        .expect("write partial");
    client.flush().expect("flush");

    let result = server.handle_readable(1, &mut runtime, None);
    let Ok(val) = result else {
        panic!("handle_readable should not error")
    };
    assert_eq!(val, false, "should return false on WouldBlock");
}

// ── 3. handle_readable when partial header ───────────────────────────────────

#[test]
fn handle_readable_returns_false_for_partial_header() {
    let path = temp_socket_path("partial_header");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    let partial = &[0x54, 0x4C, 0x42, 0x56, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
    client.write_all(partial).expect("write partial header");
    client.flush().expect("flush");

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("poll should succeed");

    let result = server.handle_readable(1, &mut runtime, None);
    let Ok(val) = result else {
        panic!("handle_readable should not error")
    };
    assert_eq!(val, false, "should return false for partial header");
}

// ── 4. handle_readable when complete header but partial payload ──────────────

#[test]
fn handle_readable_returns_false_for_complete_header_partial_payload() {
    let path = temp_socket_path("partial_payload");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 10);
    let Ok(header_bytes) = header.encode() else {
        return;
    };
    client.write_all(&header_bytes).expect("write header");
    client.flush().expect("flush");

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("poll should succeed");

    let result = server.handle_readable(1, &mut runtime, None);
    assert_eq!(
        result,
        Ok(false),
        "should return false when payload incomplete"
    );
}

// ── 5. handle_readable when read returns 0 ───────────────────────────────────

#[test]
fn handle_readable_returns_true_when_client_disconnected() {
    let path = temp_socket_path("read_zero");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    {
        let _client = make_client(&path);
        server.accept_client().expect("accept should succeed");
    }

    let result = server.handle_readable(1, &mut runtime, None);
    let Ok(val) = result else {
        panic!("handle_readable should not error")
    };
    assert_eq!(val, true, "should return true when client disconnected");
}

// ── 6. handle_writable when WouldBlock ───────────────────────────────────────

#[test]
fn handle_writable_returns_false_on_would_block() {
    let path = temp_socket_path("write_would_block");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let _runtime = make_runtime();

    let _client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    let stream = server.client_stream_mut(1).expect("client should exist");
    let big_buf = vec![0u8; 1024 * 1024];
    let mut total = 0;
    loop {
        match stream.write(&big_buf[total..]) {
            Ok(0) => break,
            Ok(n) => {
                total += n;
                if total >= big_buf.len() {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }

    let write_buf = server
        .client_write_buffer_mut(1)
        .expect("client should exist");
    write_buf.extend_from_slice(&[0xFF; 100]);

    let result = server.handle_writable(1);
    let Ok(val) = result else {
        panic!("handle_writable should not error")
    };
    assert_eq!(val, false, "should return false on WouldBlock");
}

// ── 7. handle_writable when write succeeds and buffer is empty ───────────────

#[test]
fn handle_writable_drains_buffer_and_reregisters_readable() {
    let path = temp_socket_path("write_drain");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let _runtime = make_runtime();

    let mut client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    let write_buf = server
        .client_write_buffer_mut(1)
        .expect("client should exist");
    write_buf.extend_from_slice(&[0xAB; 10]);

    let result = server.handle_writable(1);
    let Ok(val) = result else {
        panic!("handle_writable should not error")
    };
    assert_eq!(val, false, "should return false after draining");

    assert_eq!(
        server.client_write_buffer_mut(1).map(|b| b.len()),
        Some(0),
        "write_buffer should be empty"
    );

    let mut client_buf = [0u8; 10];
    let n = client
        .read(&mut client_buf)
        .expect("client should read data");
    assert_eq!(n, 10, "client should receive 10 bytes");
    assert_eq!(
        &client_buf, &[0xAB; 10],
        "client should receive correct data"
    );
}

// ── 8. handle_writable when write succeeds but buffer has remaining bytes ────

#[test]
fn handle_writable_partial_write_leaves_remaining_bytes() {
    let path = temp_socket_path("write_partial");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let _runtime = make_runtime();

    let mut client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    let stream = server.client_stream_mut(1).expect("client should exist");
    let big_buf = vec![0u8; 1024 * 1024];
    let mut total = 0;
    loop {
        match stream.write(&big_buf[total..]) {
            Ok(0) => break,
            Ok(n) => {
                total += n;
                if total >= big_buf.len() {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }

    let mut small_buf = [0u8; 1];
    let read_result = client.read(&mut small_buf);
    assert_eq!(read_result.map_err(|e| e.to_string()), Ok(1));

    let write_buf = server
        .client_write_buffer_mut(1)
        .expect("client should exist");
    write_buf.extend_from_slice(&[0xCD; 10 * 1024 * 1024]);

    let result = server.handle_writable(1);
    let Ok(val) = result else {
        panic!("handle_writable should not error")
    };
    assert_eq!(val, false, "should return false after partial write");

    let remaining = server
        .client_write_buffer_mut(1)
        .expect("client should exist")
        .len();
    assert!(
        remaining > 0,
        "write_buffer should have remaining bytes after partial write"
    );
    // Note: depending on kernel buffer state, write may return Ok(0) or a partial
    // count. The key invariant is that the buffer is not empty after the call.
}

// ── 9. handle_writable when write fails with non-WouldBlock error ────────────

#[test]
fn handle_writable_preserves_broken_pipe_error() {
    let path = temp_socket_path("write_broken");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let _runtime = make_runtime();

    {
        let _client = make_client(&path);
        server.accept_client().expect("accept should succeed");
    }

    let write_buf = server
        .client_write_buffer_mut(1)
        .expect("client should exist");
    write_buf.extend_from_slice(&[0xFF; 10]);

    let error = server
        .handle_writable(1)
        .expect_err("broken pipe must remain observable");
    assert!(
        matches!(error, IpcServerError::ResponseWriteFailed { source }
        if matches!(source.kind(), std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset))
    );
}

// ── 10. remove_client actually removes from internal maps ────────────────────

#[test]
fn remove_client_removes_from_internal_maps() {
    let path = temp_socket_path("remove_client");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let _runtime = make_runtime();

    let _client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    assert_eq!(server.client_count(), 1, "should have 1 client");

    server.remove_client(1);

    assert_eq!(
        server.client_count(),
        0,
        "should have 0 clients after removal"
    );
}

// ── 11. poll_once when a client is readable AND writable in same poll ────────

#[test]
fn poll_once_processes_readable_and_writable_for_same_client() {
    let path = temp_socket_path("poll_rw_same");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    server
        .reregister_client(1, mio::Interest::READABLE | mio::Interest::WRITABLE)
        .expect("reregister should succeed");

    let frame = build_frame(IpcCommand::Health, 1, &[]);
    client.write_all(&frame).expect("write health frame");
    client.flush().expect("flush");

    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
    assert!(result.is_ok(), "poll with readable+writable should succeed");
}

// ── 12. poll_once with multiple simultaneous events ──────────────────────────

#[test]
fn poll_once_processes_multiple_simultaneous_events() {
    let path = temp_socket_path("poll_multi");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client1 = make_client(&path);
    let mut client2 = make_client(&path);

    server.accept_client().expect("accept client 1");
    server.accept_client().expect("accept client 2");

    let frame1 = build_frame(IpcCommand::Health, 1, &[]);
    let frame2 = build_frame(IpcCommand::Health, 2, &[]);
    client1.write_all(&frame1).expect("write frame 1");
    client2.write_all(&frame2).expect("write frame 2");
    client1.flush().expect("flush 1");
    client2.flush().expect("flush 2");

    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
    assert!(
        matches!(result, Ok(_)),
        "poll with multiple clients should succeed"
    );

    let header1 = read_exact_timeout(&mut client1, IPC_HEADER_LEN);
    assert!(matches!(header1, Ok(_)), "client 1 should get response");

    let header2 = read_exact_timeout(&mut client2, IPC_HEADER_LEN);
    assert!(matches!(header2, Ok(_)), "client 2 should get response");
}

// ── cleanup helpers ──────────────────────────────────────────────────────────

/// RAII guard that removes a directory on drop.
struct CleanupDir<'a>(&'a std::path::Path);

impl Drop for CleanupDir<'_> {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(self.0) {
            eprintln!("cleanup remove_dir_all failed: {error}");
        }
    }
}

// -- Additional coverage tests for mod.rs --

#[test]
fn poll_once_with_resolver_uses_test_poll_error_when_set() {
    let path = temp_socket_path("test_poll_error");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    server.set_test_poll_once_result(Err(IpcServerError::TooManyClients));

    let result = server.poll_once_with_resolver(&mut runtime, Some(Duration::ZERO), None);
    assert_eq!(
        result,
        Err(IpcServerError::TooManyClients),
        "poll_once_with_resolver should return test_poll_error when set"
    );
}

// ── 5. poll_once removes client when handle_writable returns true ────────────

#[test]
fn poll_once_removes_client_when_writable_returns_true() {
    let path = temp_socket_path("poll_writable_true");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let client = make_client(&path);
    server.accept_client().expect("accept should succeed");

    // Queue data in write_buffer so handle_writable will try to write
    let write_buf = server
        .client_write_buffer_mut(1)
        .expect("client should exist");
    write_buf.extend_from_slice(&[0xFF; 100]);

    // Reregister as WRITABLE so poll will call handle_writable
    server
        .reregister_client(1, mio::Interest::READABLE | mio::Interest::WRITABLE)
        .expect("reregister should succeed");

    // Drop client to cause broken pipe on next write
    drop(client);

    // Poll should process the writable event and remove the client
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
    assert!(result.is_ok(), "poll should succeed");
    assert_eq!(
        server.client_count(),
        0,
        "client should be removed after writable error"
    );
}

