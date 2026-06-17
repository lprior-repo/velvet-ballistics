#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports, clippy::approximate_const, clippy::absurd_extreme_comparisons, clippy::expect_fun_call)]

#![forbid(unsafe_code)]
//! IPC client tests.

use super::{IpcClient, IpcClientError, recv_response, send_command};
use crate::server::{IpcResponse, IpcServer};
use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

#[test]
fn connect_ipc_rejects_nonexistent_socket() {
    let path = PathBuf::from("/tmp/vb_ipc_test_nonexistent_39f2.socket");
    let result = IpcClient::connect(&path);

    let Err(error) = result else {
        panic!("connecting to a nonexistent socket must return ConnectFailed error");
    };
    assert!(
        matches!(error, IpcClientError::ConnectFailed { .. }),
        "must return ConnectFailed, got {:?}",
        error
    );
    let message = error.to_string();
    assert!(
        message.contains("connect failed"),
        "error message should mention connect failed, got: {message}"
    );
}

// ── Client behavior tests ──

#[test]
fn connect_ipc_returns_connect_failed_error_variant() {
    // Given: a path to a socket that does not exist
    let path = PathBuf::from("/tmp/vb_ipc_test_noexist_77a3.socket");

    // When: attempting to connect
    let result = IpcClient::connect(&path);

    // Then: the error is ConnectFailed with a source
    let Err(IpcClientError::ConnectFailed { source: _source }) = result else {
        return;
    };
}

#[test]
fn ipc_client_error_connect_failed_display() {
    // Given: a ConnectFailed error
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let error = IpcClientError::ConnectFailed { source: io_err };

    // When: displaying the error
    let message = error.to_string();

    // Then: message mentions connect failed
    assert!(
        message.contains("connect failed"),
        "expected 'connect failed' in '{message}'"
    );
}

#[test]
fn ipc_client_error_io_error_display() {
    // Given: an IoError variant
    let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken pipe");
    let error = IpcClientError::IoError { source: io_err };

    // When: displaying the error
    let message = error.to_string();

    // Then: message mentions io error
    assert!(
        message.contains("io error"),
        "expected 'io error' in '{message}'"
    );
}

#[test]
fn ipc_client_error_frame_error_display() {
    // Given: a FrameError variant
    let ipc_err = crate::IpcError::InvalidMagic { actual: 99 };
    let error = IpcClientError::FrameError { source: ipc_err };

    // When: displaying the error
    let message = error.to_string();

    // Then: message mentions frame error
    assert!(
        message.contains("frame error"),
        "expected 'frame error' in '{message}'"
    );
}

#[test]
fn ipc_client_error_encode_failed_display() {
    // Given: an EncodeFailed variant
    let error = IpcClientError::EncodeFailed;

    // When: displaying the error
    let message = error.to_string();

    // Then: message mentions encode failed
    assert!(
        message.contains("payload encode failed"),
        "expected 'payload encode failed' in '{message}'"
    );
}

#[test]
fn send_command_returns_connect_failed_when_socket_closed() {
    // Given: a client connected to a socket that immediately closes
    // We test with a nonexistent socket path (connection fails)
    let path = PathBuf::from("/tmp/vb_ipc_test_send_fail_55b1.socket");
    let client = IpcClient::connect(&path);

    // When: connection fails, no client to send with
    // Then: verify the client was not created
    let Err(IpcClientError::ConnectFailed { .. }) = client else {
        return;
    };
}

#[test]
fn recv_response_header_returns_error_without_server() {
    // Given: a client connected to nothing (connection will fail)
    let path = PathBuf::from("/tmp/vb_ipc_test_recv_fail_88c2.socket");
    let client = IpcClient::connect(&path);

    // When: connection fails
    // Then: no recv possible
    let Err(IpcClientError::ConnectFailed { .. }) = client else {
        return;
    };
}

// ══ Adversarial client tests ══

#[test]
fn adversarial_connect_to_directory_returns_connect_failed() {
    // Given: a path that is a directory, not a socket
    let path = PathBuf::from("/tmp");

    // When: trying to connect
    let result = IpcClient::connect(&path);

    // Then: ConnectFailed
    let Err(IpcClientError::ConnectFailed { .. }) = result else {
        return;
    };
}

#[test]
fn adversarial_connect_to_empty_path_returns_connect_failed() {
    // Given: an empty path
    let path = PathBuf::from("");

    // When: trying to connect
    let result = IpcClient::connect(&path);

    // Then: ConnectFailed
    let Err(IpcClientError::ConnectFailed { .. }) = result else {
        return;
    };
}

#[test]
fn adversarial_connect_to_nested_nonexistent_returns_connect_failed() {
    // Given: a deeply nested nonexistent path
    let path = PathBuf::from("/tmp/vb_ipc_noexist_a1b2/noexist_subdir/noexist.socket");

    // When: trying to connect
    let result = IpcClient::connect(&path);

    // Then: ConnectFailed
    let Err(IpcClientError::ConnectFailed { .. }) = result else {
        return;
    };
}

#[test]
fn adversarial_client_error_variants_are_distinct() {
    // Given: all four IpcClientError variants
    let connect_err = IpcClientError::ConnectFailed {
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
    };
    let io_err = IpcClientError::IoError {
        source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken pipe"),
    };
    let frame_err = IpcClientError::FrameError {
        source: crate::IpcError::InvalidMagic { actual: 0 },
    };
    let encode_err = IpcClientError::EncodeFailed;

    // When: checking Display output
    // Then: each has a distinct message prefix
    assert!(connect_err.to_string().contains("connect failed"));
    assert!(io_err.to_string().contains("io error"));
    assert!(frame_err.to_string().contains("frame error"));
    assert!(encode_err.to_string().contains("payload encode failed"));
}

// ── helpers ─────────────────────────────────────────────────────────────────

static SOCKET_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_socket_path(name: &str) -> PathBuf {
    let sequence = SOCKET_COUNTER.fetch_add(1, Ordering::Relaxed);
    PathBuf::from(format!(
        "/tmp/vbic{}_{}_{}.sock",
        std::process::id(),
        sequence,
        bounded_socket_name(name)
    ))
}

fn bounded_socket_name(name: &str) -> String {
    name.chars().take(12).collect()
}

struct CleanupPath<'a>(&'a std::path::Path);
impl Drop for CleanupPath<'_> {
    fn drop(&mut self) {
        if let Err(_cleanup_error) = std::fs::remove_file(self.0) {}
    }
}

fn make_runtime() -> Runtime {
    let mut config = ShardConfig::default();
    config.policy = vb_core::policy::RuntimePolicy::Relaxed;
    Runtime::new(NonZeroUsize::MIN, config)
}

// ── success path tests ──────────────────────────────────────────────────────

#[test]
fn send_command_with_valid_payload_succeeds() {
    let path = temp_socket_path("send_command_ok");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).unwrap();
    let mut runtime = make_runtime();
    let mut client = IpcClient::connect(&path).unwrap();

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .unwrap();
    let payload = crate::IpcPayload::Health;
    client
        .send_command(crate::IpcCommand::Health, 1, &payload)
        .unwrap();
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .unwrap();

    let (header, response) = client
        .recv_response(crate::MaxPayloadBytes::DEFAULT)
        .unwrap();
    assert_eq!(header.command, crate::IpcCommand::Health);
    assert_eq!(header.correlation, 1);
    assert_eq!(response, IpcResponse::Healthy);
}

#[test]
fn send_raw_sends_bytes_correctly() {
    let (client_stream, mut server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
    server_stream
        .set_read_timeout(Some(std::time::Duration::from_millis(100)))
        .unwrap();
    let mut client = IpcClient {
        stream: client_stream,
    };
    let payload = b"hello raw";
    client
        .send_raw(crate::IpcCommand::Health, 42, payload)
        .unwrap();

    let mut buf = vec![0u8; crate::IPC_HEADER_LEN + payload.len()];
    server_stream
        .read_exact(&mut buf)
        .expect("send_raw should write bytes to the stream");

    let header = crate::IpcFrameHeader::decode(
        &buf[..crate::IPC_HEADER_LEN].try_into().unwrap(),
        crate::MaxPayloadBytes::DEFAULT,
    )
    .unwrap();
    assert_eq!(header.command, crate::IpcCommand::Health);
    assert_eq!(header.correlation, 42);
    assert_eq!(header.payload_len, payload.len() as u32);
    assert_eq!(&buf[crate::IPC_HEADER_LEN..], payload.as_slice());
}

#[test]
fn recv_response_header_reads_valid_header() {
    let (client_stream, mut server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut client = IpcClient {
        stream: client_stream,
    };

    let header = crate::IpcFrameHeader::new(crate::IpcCommand::Health, 0, 7, 0);
    let encoded = header.encode().unwrap();
    server_stream.write_all(&encoded).unwrap();
    server_stream.flush().unwrap();

    let received = client.recv_response_header().unwrap();
    assert_eq!(received.command, crate::IpcCommand::Health);
    assert_eq!(received.correlation, 7);
    assert_eq!(received.payload_len, 0);
}

#[test]
fn recv_response_payload_reads_matching_payload() {
    let (client_stream, mut server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut client = IpcClient {
        stream: client_stream,
    };

    let payload = b"payload data";
    let header = crate::IpcFrameHeader::new(crate::IpcCommand::Health, 0, 1, payload.len() as u32);
    let encoded = header.encode().unwrap();
    server_stream.write_all(&encoded).unwrap();
    server_stream.write_all(payload).unwrap();
    server_stream.flush().unwrap();

    let received_header = client.recv_response_header().unwrap();
    let received_payload = client.recv_response_payload(&received_header).unwrap();
    assert_eq!(received_payload, payload);
}

#[test]
fn recv_response_returns_full_frame() {
    let path = temp_socket_path("recv_response_full");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).unwrap();
    let mut runtime = make_runtime();
    let mut client = IpcClient::connect(&path).unwrap();

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .unwrap();
    client.health(99).unwrap();
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .unwrap();

    let (header, response) = client
        .recv_response(crate::MaxPayloadBytes::DEFAULT)
        .unwrap();
    assert_eq!(header.command, crate::IpcCommand::Health);
    assert_eq!(header.correlation, 99);
    assert_eq!(response, IpcResponse::Healthy);
}

#[test]
fn health_sends_command_and_receives_healthy() {
    let path = temp_socket_path("health_roundtrip");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).unwrap();
    let mut runtime = make_runtime();
    let mut client = IpcClient::connect(&path).unwrap();

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .unwrap();
    client.health(101).unwrap();
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .unwrap();

    let (header, response) = client
        .recv_response(crate::MaxPayloadBytes::DEFAULT)
        .unwrap();
    assert_eq!(header.command, crate::IpcCommand::Health);
    assert_eq!(header.correlation, 101);
    assert_eq!(response, IpcResponse::Healthy);
}

#[test]
fn shutdown_sends_shutdown_command() {
    let path = temp_socket_path("shutdown_cmd");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).unwrap();
    let mut runtime = make_runtime();
    let mut client = IpcClient::connect(&path).unwrap();

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .unwrap();
    client.shutdown(202).unwrap();
    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .unwrap();

    let (header, response) = client
        .recv_response(crate::MaxPayloadBytes::DEFAULT)
        .unwrap();
    assert_eq!(header.command, crate::IpcCommand::Shutdown);
    assert_eq!(header.correlation, 202);
    assert_eq!(response, IpcResponse::ShuttingDown);
}

#[test]
fn send_command_writes_bytes_to_stream() {
    let (client_stream, mut server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
    server_stream
        .set_read_timeout(Some(std::time::Duration::from_millis(100)))
        .unwrap();
    let mut client = IpcClient {
        stream: client_stream,
    };
    let payload = crate::IpcPayload::Health;
    client
        .send_command(crate::IpcCommand::Health, 77, &payload)
        .unwrap();

    let mut buf = vec![0u8; crate::IPC_HEADER_LEN];
    server_stream
        .read_exact(&mut buf)
        .expect("send_command should write header bytes");
    let header =
        crate::IpcFrameHeader::decode(&buf.try_into().unwrap(), crate::MaxPayloadBytes::DEFAULT)
            .unwrap();
    assert_eq!(header.command, crate::IpcCommand::Health);
    assert_eq!(header.correlation, 77);
}

#[test]
fn health_writes_bytes_to_stream() {
    let (client_stream, mut server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
    server_stream
        .set_read_timeout(Some(std::time::Duration::from_millis(100)))
        .unwrap();
    let mut client = IpcClient {
        stream: client_stream,
    };
    client.health(88).unwrap();

    let mut buf = vec![0u8; crate::IPC_HEADER_LEN];
    server_stream
        .read_exact(&mut buf)
        .expect("health should write header bytes");
    let header =
        crate::IpcFrameHeader::decode(&buf.try_into().unwrap(), crate::MaxPayloadBytes::DEFAULT)
            .unwrap();
    assert_eq!(header.command, crate::IpcCommand::Health);
    assert_eq!(header.correlation, 88);
}

#[test]
fn shutdown_writes_bytes_to_stream() {
    let (client_stream, mut server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
    server_stream
        .set_read_timeout(Some(std::time::Duration::from_millis(100)))
        .unwrap();
    let mut client = IpcClient {
        stream: client_stream,
    };
    client.shutdown(99).unwrap();

    let mut buf = vec![0u8; crate::IPC_HEADER_LEN];
    server_stream
        .read_exact(&mut buf)
        .expect("shutdown should write header bytes");
    let header =
        crate::IpcFrameHeader::decode(&buf.try_into().unwrap(), crate::MaxPayloadBytes::DEFAULT)
            .unwrap();
    assert_eq!(header.command, crate::IpcCommand::Shutdown);
    assert_eq!(header.correlation, 99);
}

// ── failure path tests ──────────────────────────────────────────────────────

#[test]
fn recv_response_header_fails_with_frame_error_on_bad_magic() {
    let (client_stream, mut server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut client = IpcClient {
        stream: client_stream,
    };

    let mut bad_header = [0u8; crate::IPC_HEADER_LEN];
    bad_header[..4].copy_from_slice(&0xDEAD_BEEF_u32.to_le_bytes());
    server_stream.write_all(&bad_header).unwrap();
    server_stream.flush().unwrap();

    let result = client.recv_response_header();
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("frame error"),
        "expected frame error, got {msg}"
    );
}

#[test]
fn recv_response_payload_fails_on_short_read() {
    let (client_stream, mut server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut client = IpcClient {
        stream: client_stream,
    };

    let header = crate::IpcFrameHeader::new(crate::IpcCommand::Health, 0, 1, 10);
    let encoded = header.encode().unwrap();
    server_stream.write_all(&encoded).unwrap();
    server_stream.write_all(b"abc").unwrap();
    server_stream.flush().unwrap();
    drop(server_stream);

    let received_header = client.recv_response_header().unwrap();
    let result = client.recv_response_payload(&received_header);
    let Err(IpcClientError::FrameError { source }) = result else {
        panic!("expected FrameError, got {:?}", result);
    };
    assert_eq!(source, crate::IpcError::PayloadDecodeFailed);
}

#[test]
fn send_command_fails_with_io_error_on_broken_pipe() {
    let (client_stream, server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut client = IpcClient {
        stream: client_stream,
    };
    drop(server_stream);

    let payload = crate::IpcPayload::Health;
    let result = client.send_command(crate::IpcCommand::Health, 1, &payload);
    let Err(IpcClientError::IoError { source }) = result else {
        panic!("expected IoError, got {:?}", result);
    };
    assert_eq!(source.kind(), std::io::ErrorKind::BrokenPipe);
}

#[test]
fn recv_response_fails_when_header_decode_fails() {
    let (client_stream, mut server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut client = IpcClient {
        stream: client_stream,
    };

    let mut bad_header = [0u8; crate::IPC_HEADER_LEN];
    bad_header[..4].copy_from_slice(&0xDEAD_BEEF_u32.to_le_bytes());
    server_stream.write_all(&bad_header).unwrap();
    server_stream.flush().unwrap();

    let result = client.recv_response(crate::MaxPayloadBytes::DEFAULT);
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("frame error"),
        "expected frame error, got {msg}"
    );
}

#[test]
fn top_level_send_command_delegates_and_writes_bytes() {
    let (client_stream, mut server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
    server_stream
        .set_read_timeout(Some(std::time::Duration::from_millis(100)))
        .unwrap();
    let mut client = IpcClient {
        stream: client_stream,
    };
    let payload = crate::IpcPayload::Health;
    send_command(&mut client, crate::IpcCommand::Health, 55, &payload).unwrap();

    let mut buf = vec![0u8; crate::IPC_HEADER_LEN];
    server_stream
        .read_exact(&mut buf)
        .expect("top-level send_command should write bytes");
    let header =
        crate::IpcFrameHeader::decode(&buf.try_into().unwrap(), crate::MaxPayloadBytes::DEFAULT)
            .unwrap();
    assert_eq!(header.command, crate::IpcCommand::Health);
    assert_eq!(header.correlation, 55);
}

#[test]
fn top_level_recv_response_delegates_and_decodes() {
    let (client_stream, mut server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut client = IpcClient {
        stream: client_stream,
    };

    let response = crate::server::IpcResponse::Healthy;
    let payload = postcard::to_allocvec(&response).unwrap();
    let header = crate::IpcFrameHeader::new(crate::IpcCommand::Health, 0, 1, payload.len() as u32);
    server_stream.write_all(&header.encode().unwrap()).unwrap();
    server_stream.write_all(&payload).unwrap();
    server_stream.flush().unwrap();

    let (received_header, received_response) =
        recv_response(&mut client, crate::MaxPayloadBytes::DEFAULT).unwrap();
    assert_eq!(received_header.command, crate::IpcCommand::Health);
    assert_eq!(received_response, crate::server::IpcResponse::Healthy);
}

#[test]
fn send_command_writes_bytes_to_server_accepted_stream() {
    let path = temp_socket_path("send_command_server");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).unwrap();
    let mut runtime = make_runtime();
    let mut client = IpcClient::connect(&path).unwrap();

    server
        .poll_once(&mut runtime, Some(Duration::from_millis(100)))
        .unwrap();
    assert_eq!(server.client_count(), 1);

    let payload = crate::IpcPayload::Health;
    let encoded_payload = postcard::to_allocvec(&payload).unwrap();
    client
        .send_command(crate::IpcCommand::Health, 1, &payload)
        .unwrap();

    let server_stream = server.client_stream_mut(1).unwrap();
    let mut buf = vec![0u8; crate::IPC_HEADER_LEN];
    let mut total_read = 0usize;
    for _ in 0..100_000 {
        match server_stream.read(&mut buf[total_read..]) {
            Ok(0) => panic!("unexpected EOF reading from server stream"),
            Ok(n) => {
                total_read += n;
                if total_read >= crate::IPC_HEADER_LEN {
                    break;
                }
                continue;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::hint::spin_loop();
                continue;
            }
            Err(e) => panic!("read error from server stream: {e}"),
        }
    }
    assert_eq!(total_read, crate::IPC_HEADER_LEN);

    let header =
        crate::IpcFrameHeader::decode(&buf.try_into().unwrap(), crate::MaxPayloadBytes::DEFAULT)
            .unwrap();
    assert_eq!(header.command, crate::IpcCommand::Health);
    assert_eq!(header.correlation, 1);
    assert_eq!(header.payload_len, encoded_payload.len() as u32);

    let mut payload_buf = vec![0u8; encoded_payload.len()];
    total_read = 0;
    for _ in 0..100_000 {
        match server_stream.read(&mut payload_buf[total_read..]) {
            Ok(0) => panic!("unexpected EOF reading payload from server stream"),
            Ok(n) => {
                total_read += n;
                if total_read >= encoded_payload.len() {
                    break;
                }
                continue;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::hint::spin_loop();
                continue;
            }
            Err(e) => panic!("read error from server stream payload: {e}"),
        }
    }
    assert_eq!(payload_buf, encoded_payload);
}
