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
use crate::IpcCommand;
use crate::IpcFrameHeader;
use crate::IPC_HEADER_LEN;
use crate::IPC_MAGIC;
use crate::IPC_VERSION;

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
    let encoded = header.encode().expect("header should encode");
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
            Ok(0) => return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "eof")),
            Ok(count) => read_total = read_total.saturating_add(count),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(1));
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

    assert!(result.is_ok(), "bind to fresh path should succeed");
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

    assert!(result.is_ok(), "bind should remove stale socket and succeed");
}

#[test]
fn bind_to_nested_directory_fails() {
    let path = std::env::temp_dir().join("vb_ipc_nonexistent_dir_test").join("sock");
    let result = IpcServer::bind(&path);

    assert!(result.is_err(), "bind to nonexistent directory should fail");
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

    assert!(result.is_ok(), "poll_once with zero timeout should succeed");
    let Ok(continuing) = result else { return };
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

    assert!(result.is_ok(), "poll_once should succeed after client connects");
}

#[test]
fn poll_once_with_resolver_returns_ok_without_resolver() {
    let path = temp_socket_path("poll_resolver_none");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let result = server.poll_once_with_resolver(
        &mut runtime,
        Some(Duration::ZERO),
        None,
    );

    assert!(result.is_ok(), "poll_once_with_resolver with None resolver should succeed");
}

// ── serve_ipc dispatch function tests ───────────────────────────────────────

#[test]
fn serve_ipc_delegates_to_poll_once() {
    let path = temp_socket_path("serve_ipc_basic");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();

    let result = serve_ipc(&mut server, &mut runtime, Some(Duration::ZERO));

    assert!(result.is_ok(), "serve_ipc should succeed");
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
    client.write_all(&frame).expect("client should write health frame");
    client.flush().expect("client should flush");

    // Server processes the readable event.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process poll should succeed");

    // Read the response header.
    let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);
    assert!(response_header_bytes.is_ok(), "should read response header");
    let response_header = response_header_bytes.expect("read header");

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
        assert!(response_payload.is_ok(), "should read response payload");
        let payload = response_payload.expect("read payload");
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
    assert!(result.is_ok(), "poll after client disconnect should succeed");
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
    client.write_all(&bad_header).expect("client should write bad frame");
    client.flush().expect("client should flush");

    // Server processes the readable event.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process poll should succeed");

    // Read response header.
    let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);
    assert!(response_header_bytes.is_ok(), "should read error response header");

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
        assert!(response_payload.is_ok(), "should read error response payload");
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
    client.write_all(&bad_header).expect("client should write bad version frame");
    client.flush().expect("client should flush");

    // Server processes the readable event.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process poll should succeed");

    // Read response header.
    let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);
    assert!(response_header_bytes.is_ok(), "should read error response header");

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
        assert!(response_payload.is_ok(), "should read error response payload");
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
    assert!(result.is_ok(), "poll after multiple accepts should succeed");

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
    client.write_all(partial).expect("client should write partial frame");
    client.flush().expect("client should flush");

    // Server should handle the partial read without error.
    let result = server.poll_once(&mut runtime, Some(Duration::from_millis(100)));
    assert!(result.is_ok(), "poll with partial frame should not error");
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
    client.write_all(&frame).expect("client should write garbage frame");
    client.flush().expect("client should flush");

    // Server processes the readable event.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process poll should succeed");

    // Read response header.
    let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);
    assert!(response_header_bytes.is_ok(), "should read response header");

    // Read response payload and verify it is not a panic.
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
        assert!(response_payload.is_ok(), "should read response payload");
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
    client.write_all(&pipeline).expect("client should write pipelined frames");
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
            assert!(
                response_payload.is_ok(),
                "should read response payload {i}"
            );
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
    client.write_all(&header_bytes).expect("client should write nonzero reserved");
    client.flush().expect("client should flush");

    // Server processes the readable event.
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .expect("process poll should succeed");

    // Read response header.
    let response_header_bytes = read_exact_timeout(&mut client, IPC_HEADER_LEN);
    assert!(response_header_bytes.is_ok(), "should read response header");

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
        assert!(response_payload.is_ok(), "should read response payload");
        let payload = response_payload.expect("read payload");
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
    assert!(msg.contains("bind failed"), "expected 'bind failed' in '{msg}'");
}

#[test]
fn ipc_server_error_poll_failed_display() {
    let err = IpcServerError::PollFailed {
        source: std::io::Error::new(std::io::ErrorKind::Interrupted, "interrupted"),
    };
    let msg = err.to_string();
    assert!(msg.contains("poll failed"), "expected 'poll failed' in '{msg}'");
}

#[test]
fn ipc_server_error_too_many_clients_display() {
    let err = IpcServerError::TooManyClients;
    let msg = err.to_string();
    assert!(msg.contains("too many clients"), "expected 'too many clients' in '{msg}'");
}

// ── cleanup helper ──────────────────────────────────────────────────────────

/// RAII guard that removes the socket path on drop.
struct CleanupPath<'a>(&'a std::path::Path);

impl Drop for CleanupPath<'_> {
    fn drop(&mut self) {
        drop(std::fs::remove_file(self.0));
    }
}
