#![forbid(unsafe_code)]
//! Tests for `IpcServer` implementation (`impl_.rs`).
//!
//! Covers:
//! - `IpcServer::bind` — socket binding and cleanup
//! - `IpcServer::poll_once` / `poll_once_with_resolver` — event polling
//! - `serve_ipc` — public dispatch function
//! - Private helpers via integration: `accept_client`, `handle_readable`,
//!   `handle_writable`, `remove_client`

use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

use mio::net::UnixStream;

use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

use super::error::IpcServerError;
use super::{IpcResponse, IpcServer, serve_ipc};
use crate::IPC_HEADER_LEN;
use crate::IPC_MAGIC;
use crate::IPC_VERSION;
use crate::IpcCommand;
use crate::IpcFrameHeader;
use crate::IpcPayload;
use crate::MaxPayloadBytes;
use vb_core::{RunId, WorkflowDigest};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn temp_socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("vb_ipc_impl_test_{name}_{}", std::process::id()))
}

fn make_runtime() -> Runtime {
    Runtime::new(NonZeroUsize::MIN, ShardConfig::default())
}

fn make_client(server_path: &std::path::Path) -> UnixStream {
    UnixStream::connect(server_path).expect("test client should connect to server socket")
}

/// Builds a valid IPC frame header + payload bytes for the given command and postcard payload.
fn build_frame(command: IpcCommand, correlation: u64, payload_bytes: &[u8]) -> Vec<u8> {
    let header = IpcFrameHeader::new(
        command,
        0,
        correlation,
        match u32::try_from(payload_bytes.len()) {
            Ok(v) => v,
            Err(_) => 0,
        },
    );
    let encoded = header.encode().expect("encode header");
    let mut frame = encoded.to_vec();
    frame.extend_from_slice(payload_bytes);
    frame
}

/// Reads exactly `n` bytes from the stream with a short timeout.
fn read_exact_timeout(stream: &mut dyn Read, n: usize) -> Result<Vec<u8>, std::io::Error> {
    let mut buf = vec![0u8; n];
    let mut read_total = 0usize;
    while read_total < n {
        match stream.read(&mut buf[read_total..]) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "eof",
                ));
            }
            Ok(count) => read_total = read_total.saturating_add(count),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::yield_now();
            }
            Err(e) => return Err(e),
        }
    }
    Ok(buf)
}

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
fn bind_removes_existing_socket_file() {
    let path = temp_socket_path("bind_replaces");
    let _cleanup = CleanupPath(&path);

    // Create a stale socket file.
    std::fs::write(&path, b"stale").expect("write stale file");
    assert!(path.exists(), "stale file should be present");

    let result = IpcServer::bind(&path);

    assert!(
        result.is_ok(),
        "bind should remove stale socket and succeed"
    );
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
    let header_bytes = header.encode().expect("header encode");
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
    let header_bytes = header.encode().expect("header encode");
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
fn server_responds_with_error_for_invalid_magic() {
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
                    message.contains("magic") || message.contains("invalid"),
                    "frame error should mention invalid magic, got '{message}'"
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

    // Server processes both commands.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process poll should succeed");

    // Read both response headers.
    for i in 1..=2 {
        let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);
        assert!(
            response_header_bytes.is_ok(),
            "should read response header {i}"
        );
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

/// RAII guard that removes the socket path on drop.
struct CleanupPath<'a>(&'a std::path::Path);

impl Drop for CleanupPath<'_> {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(self.0) {
            eprintln!("cleanup remove_file failed: {error}");
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Additional coverage tests
// ══════════════════════════════════════════════════════════════════════════════

use super::dispatch::serve_ipc_with_resolver;
use super::{WorkflowResolutionError, WorkflowResolver};
use crate::SubmitRunPayload;

// ── serve_ipc with None timeout ─────────────────────────────────────────────

#[test]
fn serve_ipc_with_none_timeout_returns_ok_when_client_connected() {
    let path = temp_socket_path("serve_none_timeout");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    // Connect a client so the poll has an event to process and does not block.
    let _client = make_client(&path);

    let result = serve_ipc(&mut server, &mut runtime, None);

    assert_eq!(
        result,
        Ok(true),
        "serve_ipc with None timeout should succeed and return continue"
    );
}

// ── serve_ipc_with_resolver with None timeout and None resolver ─────────────

#[test]
fn serve_ipc_with_resolver_none_timeout_none_resolver_returns_ok_when_client_connected() {
    let path = temp_socket_path("serve_ipc_resolver_none");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    // Connect a client so the poll has an event to process and does not block.
    let _client = make_client(&path);

    let result = serve_ipc_with_resolver(&mut server, &mut runtime, None, None);

    assert_eq!(
        result,
        Ok(true),
        "serve_ipc_with_resolver with None timeout and None resolver should return Ok(true)"
    );
}

// ── IpcServerError Display for accept_failed ────────────────────────────────

#[test]
fn ipc_server_error_accept_failed_display() {
    let err = IpcServerError::AcceptFailed {
        source: std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("accept failed"),
        "expected 'accept failed' in '{msg}'"
    );
}

// ── IpcServerError Display for response_encode_failed ───────────────────────

#[test]
fn ipc_server_error_response_encode_failed_display() {
    let err = IpcServerError::ResponseEncodeFailed;
    let msg = err.to_string();
    assert!(
        msg.contains("response encode failed"),
        "expected 'response encode failed' in '{msg}'"
    );
}

// ── IpcServerError Display for incomplete_frame ─────────────────────────────

#[test]
fn ipc_server_error_incomplete_frame_display() {
    let err = IpcServerError::IncompleteFrame;
    let msg = err.to_string();
    assert!(
        msg.contains("incomplete IPC frame"),
        "expected 'incomplete IPC frame' in '{msg}'"
    );
}

// ── WorkflowResolver trait: NotFound error case ─────────────────────────────

struct NotFoundResolver;

impl WorkflowResolver for NotFoundResolver {
    fn resolve_workflow(
        &mut self,
        _digest: vb_core::WorkflowDigest,
    ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
        Err(WorkflowResolutionError::NotFound)
    }
}

#[test]
fn workflow_resolver_not_found_error_message() {
    let err = WorkflowResolutionError::NotFound;
    let msg = err.to_string();
    assert!(msg.contains("not found"), "expected 'not found' in '{msg}'");
}

#[test]
fn workflow_resolver_not_found_is_rejected_by_dispatch() {
    let path = temp_socket_path("resolver_not_found");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let mut client = make_client(&path);

    // Accept the client.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("accept poll should succeed");

    // Build a SubmitRun payload with a zeroed (unlikely) digest.
    let submit = SubmitRunPayload {
        run_id: vb_core::RunId::new(42u64),
        workflow: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        input: vec![],
    };
    let ipc_payload = crate::IpcPayload::SubmitRun(submit);
    let payload_bytes = postcard::to_allocvec(&ipc_payload).expect("encode payload");
    let frame = build_frame(IpcCommand::SubmitRun, 99, &payload_bytes);
    client
        .write_all(&frame)
        .expect("client should write submit frame");
    client.flush().expect("client should flush");

    // Process with a resolver that always returns NotFound.
    let mut resolver = NotFoundResolver;
    let result = server.poll_once_with_resolver(
        &mut runtime,
        Some(Duration::from_millis(100)),
        Some(&mut resolver),
    );
    assert!(
        matches!(result, Ok(_)),
        "poll_once_with_resolver should succeed"
    );

    // Read response header.
    let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);
    let Ok(response_header) = response_header_bytes else {
        panic!("should read response header")
    };

    // Read response payload.
    let payload_len = u32::from_le_bytes(
        response_header
            .get(20..24)
            .and_then(|s| <[u8; 4]>::try_from(s).ok())
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
            Ok(IpcResponse::PayloadError { message, .. }) => {
                assert!(
                    message.contains("not found") || message.contains("digest"),
                    "expected 'not found' or 'digest' in payload error message, got '{message}'"
                );
            }
            Ok(IpcResponse::WorkflowDigestMismatch) => {
                // Also acceptable: server detected digest mismatch before calling resolver.
            }
            Ok(IpcResponse::WorkflowResolutionUnsupported) => {
                // Also acceptable for certain code paths.
            }
            Ok(other) => {
                assert!(
                    !matches!(other, IpcResponse::Healthy),
                    "Unexpected Healthy response when resolver returned NotFound"
                );
            }
            Err(e) => {
                assert!(false, "response payload decode failed: {e}");
            }
        }
    }
}

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

// ── 1. bind: re-binding after server drop frees the socket ────────────────────

#[test]
fn bind_succeeds_after_previous_server_dropped() {
    let path = temp_socket_path("bind_rebind");
    let _cleanup = CleanupPath(&path);

    // Bind once, then drop the server.
    {
        let _server = IpcServer::bind(&path).expect("first bind should succeed");
        assert!(path.exists(), "socket should exist while server is alive");
    }

    // The socket file may or may not be cleaned up by mio on drop.
    // IpcServer::bind handles a stale file, so re-binding must succeed.
    let result = IpcServer::bind(&path);
    assert!(
        result.is_ok(),
        "re-binding after server drop should succeed, got {:?}",
        result.err()
    );
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

// ── IpcResponse serialization roundtrip: RunList ─────────────────────────────

#[test]
fn ipc_response_roundtrip_run_list_empty() {
    let original = IpcResponse::RunList { runs: vec![] };
    let encoded = postcard::to_allocvec(&original).expect("encode RunList");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode RunList");
    assert_eq!(decoded, original, "RunList roundtrip should be equal");
}

#[test]
fn ipc_response_roundtrip_run_list_with_entries() {
    let original = IpcResponse::RunList {
        runs: vec![
            crate::RunSummary {
                run_id: RunId::new(1),
                workflow: WorkflowDigest::from_bytes([0xAA; 32]),
                state: crate::RunListState::Active,
                submitted_seq: 100,
                finished_seq: None,
                step_count: 5,
                steps_completed: 2,
            },
            crate::RunSummary {
                run_id: RunId::new(2),
                workflow: WorkflowDigest::from_bytes([0xBB; 32]),
                state: crate::RunListState::Finished,
                submitted_seq: 200,
                finished_seq: Some(250),
                step_count: 8,
                steps_completed: 8,
            },
        ],
    };
    let encoded = postcard::to_allocvec(&original).expect("encode RunList");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode RunList");
    assert_eq!(decoded, original, "RunList roundtrip should be equal");
}

// ── IpcResponse serialization roundtrip: Metrics ─────────────────────────────

#[test]
fn ipc_response_roundtrip_metrics() {
    let original = IpcResponse::Metrics(crate::RuntimeMetrics {
        shards: vec![crate::ShardMetrics {
            shard_id: 0,
            active_runs: 3,
            ready_queue_depth: 1,
            action_queue_depth: 2,
            timer_count: 0,
            frame_pool_free: 100,
            frame_pool_total: 256,
            trace_ring_fill_pct: 12.5_f32,
            steps_total: 42,
            actions_total: 7,
        }],
        journal: crate::JournalMetrics {
            writer_queue_depth: 0,
            total_events: 1000,
            total_runs: 50,
        },
        ipc: crate::IpcMetrics {
            connected_clients: 2,
            commands_processed: 200,
        },
        totals: crate::AggregateMetrics {
            runs_active: 3,
            runs_waiting: 1,
            runs_failed_total: 5,
            runs_finished_total: 45,
        },
    });
    let encoded = postcard::to_allocvec(&original).expect("encode Metrics");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode Metrics");
    assert_eq!(decoded, original, "Metrics roundtrip should be equal");
}

// ── IpcResponse serialization roundtrip: VerifyWorkflow ──────────────────────

#[test]
fn ipc_response_roundtrip_verify_workflow() {
    let original = IpcResponse::VerifyWorkflow {
        result: crate::VerificationResult {
            certificates: vec![
                crate::CertificateWire {
                    kind: crate::GateKind::Gate07ExpressionStackDepth,
                    status: crate::PassFail::Pass,
                    details: String::new(),
                },
                crate::CertificateWire {
                    kind: crate::GateKind::Gate08AccessorPathSegments,
                    status: crate::PassFail::Fail,
                    details: String::from("stack depth exceeded"),
                },
            ],
            total_checks: 2,
            pass_count: 1,
            fail_count: 1,
        },
    };
    let encoded = postcard::to_allocvec(&original).expect("encode VerifyWorkflow");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode VerifyWorkflow");
    assert_eq!(
        decoded, original,
        "VerifyWorkflow roundtrip should be equal"
    );
}

// ── IpcResponse serialization roundtrip: TaintReport ─────────────────────────

#[test]
fn ipc_response_roundtrip_taint_report_safe() {
    let original = IpcResponse::TaintReport {
        sources: vec![1, 5],
        sinks: vec![10],
        finish_safe: true,
        paths: vec![],
    };
    let encoded = postcard::to_allocvec(&original).expect("encode TaintReport");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode TaintReport");
    assert_eq!(decoded, original, "TaintReport roundtrip should be equal");
}

#[test]
fn ipc_response_roundtrip_taint_report_with_paths() {
    let original = IpcResponse::TaintReport {
        sources: vec![0],
        sinks: vec![9],
        finish_safe: false,
        paths: vec![
            crate::TaintPathWire {
                from: 0,
                to: 3,
                status: crate::TaintPathStatus::Warning,
            },
            crate::TaintPathWire {
                from: 3,
                to: 9,
                status: crate::TaintPathStatus::Dangerous,
            },
        ],
    };
    let encoded = postcard::to_allocvec(&original).expect("encode TaintReport");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode TaintReport");
    assert_eq!(decoded, original, "TaintReport roundtrip should be equal");
}

// ── IpcResponse serialization roundtrip: WorkflowGraph ───────────────────────

#[test]
fn ipc_response_roundtrip_workflow_graph() {
    let original = IpcResponse::WorkflowGraph {
        nodes: vec![
            crate::NodeDescriptor {
                step_idx: 0,
                kind: crate::NodeKind::Nop,
                next: Some(1),
                title: String::from("start"),
            },
            crate::NodeDescriptor {
                step_idx: 1,
                kind: crate::NodeKind::Finish,
                next: None,
                title: String::from("end"),
            },
        ],
        edges: vec![crate::EdgeDescriptor {
            from: 0,
            to: 1,
            label: None,
            edge_type: crate::EdgeType::Fallthrough,
        }],
    };
    let encoded = postcard::to_allocvec(&original).expect("encode WorkflowGraph");
    let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode WorkflowGraph");
    assert_eq!(decoded, original, "WorkflowGraph roundtrip should be equal");
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

// ── RunListState serialization roundtrip ─────────────────────────────────────

#[test]
fn run_list_state_roundtrip_active() {
    let original = crate::RunListState::Active;
    let encoded = postcard::to_allocvec(&original).expect("encode Active");
    let decoded: crate::RunListState = postcard::from_bytes(&encoded).expect("decode Active");
    assert_eq!(decoded, original);
}

#[test]
fn run_list_state_roundtrip_finished() {
    let original = crate::RunListState::Finished;
    let encoded = postcard::to_allocvec(&original).expect("encode Finished");
    let decoded: crate::RunListState = postcard::from_bytes(&encoded).expect("decode Finished");
    assert_eq!(decoded, original);
}

#[test]
fn run_list_state_roundtrip_failed() {
    let original = crate::RunListState::Failed;
    let encoded = postcard::to_allocvec(&original).expect("encode Failed");
    let decoded: crate::RunListState = postcard::from_bytes(&encoded).expect("decode Failed");
    assert_eq!(decoded, original);
}

#[test]
fn run_list_state_roundtrip_cancelled() {
    let original = crate::RunListState::Cancelled;
    let encoded = postcard::to_allocvec(&original).expect("encode Cancelled");
    let decoded: crate::RunListState = postcard::from_bytes(&encoded).expect("decode Cancelled");
    assert_eq!(decoded, original);
}

// ══════════════════════════════════════════════════════════════════════════════
// Coverage tests for impl_.rs branches
// ══════════════════════════════════════════════════════════════════════════════

use super::impl_::MAX_CLIENTS;

// ── 1. accept_client when max clients reached ────────────────────────────────

#[test]
fn accept_client_returns_too_many_clients_when_at_capacity() {
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

    let _extra = make_client(&path);
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
    assert_eq!(
        result,
        Err(IpcServerError::TooManyClients),
        "should fail when max clients reached"
    );
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

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("poll should succeed");

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

    let partial = &[0x56, 0x42, 0x4C, 0x54, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
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
    let header_bytes = header.encode().expect("encode header");
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
fn handle_writable_returns_true_on_broken_pipe() {
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

    let result = server.handle_writable(1);
    let Ok(val) = result else {
        panic!("handle_writable should not error")
    };
    assert_eq!(val, true, "should return true on broken pipe");
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

    // Send a bad header (invalid magic) so handle_readable enters the error branch
    let mut bad_header = [0u8; IPC_HEADER_LEN];
    bad_header[..4].copy_from_slice(&0u32.to_le_bytes());
    bad_header[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
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
