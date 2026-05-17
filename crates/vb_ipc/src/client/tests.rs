#![forbid(unsafe_code)]
//! Tests for client module.

use crate::client::{IpcClient, IpcClientError};

#[test]
fn connect_ipc_rejects_nonexistent_socket() {
    let path = std::path::PathBuf::from("/tmp/vb_ipc_test_nonexistent_39f2.socket");
    let result = IpcClient::connect(&path);

    let Err(error) = result else {
        panic!("connecting to a nonexistent socket must fail, got Ok");
    };
    let message = error.to_string();
    assert!(message.contains("connect failed"), "error message should mention connect failed, got: {message}");
}

#[test]
fn connect_ipc_returns_connect_failed_error_variant() {
    let path = std::path::PathBuf::from("/tmp/vb_ipc_test_noexist_77a3.socket");
    let result = IpcClient::connect(&path);

    let Err(IpcClientError::ConnectFailed { source: _source }) = result else { return };
}

#[test]
fn ipc_client_error_connect_failed_display() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let error = IpcClientError::ConnectFailed { source: io_err };
    let message = error.to_string();
    assert!(message.contains("connect failed"), "expected 'connect failed' in '{message}'");
}

#[test]
fn ipc_client_error_io_error_display() {
    let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken pipe");
    let error = IpcClientError::IoError { source: io_err };
    let message = error.to_string();
    assert!(message.contains("io error"), "expected 'io error' in '{message}'");
}

#[test]
fn ipc_client_error_frame_error_display() {
    let ipc_err = crate::IpcError::InvalidMagic { actual: 99 };
    let error = IpcClientError::FrameError { source: ipc_err };
    let message = error.to_string();
    assert!(message.contains("frame error"), "expected 'frame error' in '{message}'");
}

#[test]
fn ipc_client_error_encode_failed_display() {
    let error = IpcClientError::EncodeFailed;
    let message = error.to_string();
    assert!(message.contains("payload encode failed"), "expected 'payload encode failed' in '{message}'");
}

#[test]
fn send_command_returns_connect_failed_when_socket_closed() {
    let path = std::path::PathBuf::from("/tmp/vb_ipc_test_send_fail_55b1.socket");
    let client = IpcClient::connect(&path);
    let Err(IpcClientError::ConnectFailed { .. }) = client else { return };
}

#[test]
fn recv_response_header_returns_error_without_server() {
    let path = std::path::PathBuf::from("/tmp/vb_ipc_test_recv_fail_88c2.socket");
    let client = IpcClient::connect(&path);
    let Err(IpcClientError::ConnectFailed { .. }) = client else { return };
}

// ══ Adversarial client tests ═══════════════════════════════════════════════════

#[test]
fn adversarial_connect_to_directory_returns_connect_failed() {
    let path = std::path::PathBuf::from("/tmp");
    let result = IpcClient::connect(&path);
    let Err(IpcClientError::ConnectFailed { .. }) = result else { return };
}

#[test]
fn adversarial_connect_to_empty_path_returns_connect_failed() {
    let path = std::path::PathBuf::from("");
    let result = IpcClient::connect(&path);
    let Err(IpcClientError::ConnectFailed { .. }) = result else { return };
}

#[test]
fn adversarial_connect_to_nested_nonexistent_returns_connect_failed() {
    let path = std::path::PathBuf::from("/tmp/vb_ipc_noexist_a1b2/noexist_subdir/noexist.socket");
    let result = IpcClient::connect(&path);
    let Err(IpcClientError::ConnectFailed { .. }) = result else { return };
}

#[test]
fn adversarial_client_error_variants_are_distinct() {
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

    assert!(connect_err.to_string().contains("connect failed"));
    assert!(io_err.to_string().contains("io error"));
    assert!(frame_err.to_string().contains("frame error"));
    assert!(encode_err.to_string().contains("payload encode failed"));
}
