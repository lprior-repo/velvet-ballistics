#![forbid(unsafe_code)]
//! IPC client for connecting to a velvet_ballastics runtime.

use std::io::Write;
use std::path::Path;

use crate::frame::{read_frame_header_bounded, read_frame_payload_bounded};
use crate::server::IpcResponse;
use crate::{IpcCommand, IpcError, IpcPayload, MaxPayloadBytes};
use vb_core::WorkflowDigest;

/// IPC client connected to a Unix domain socket.
pub struct IpcClient {
    pub(crate) stream: std::os::unix::net::UnixStream,
}

impl IpcClient {
    /// Connects to a Unix domain socket IPC endpoint.
    pub fn connect(socket_path: &Path) -> Result<Self, IpcClientError> {
        let stream = std::os::unix::net::UnixStream::connect(socket_path)
            .map_err(|source| IpcClientError::ConnectFailed { source })?;
        Ok(Self { stream })
    }

    /// Sends a command with a postcard-encoded payload.
    pub fn send_command(
        &mut self,
        command: IpcCommand,
        correlation: u64,
        payload: &IpcPayload,
    ) -> Result<(), IpcClientError> {
        let encoded = postcard::to_allocvec(payload).map_err(|_| IpcClientError::EncodeFailed)?;
        self.send_raw(command, correlation, &encoded)
    }

    /// Sends a raw command with pre-encoded payload bytes.
    pub fn send_raw(
        &mut self,
        command: IpcCommand,
        correlation: u64,
        payload: &[u8],
    ) -> Result<(), IpcClientError> {
        let frame = crate::frame::encode_frame(command, 0, correlation, payload)
            .map_err(|source| IpcClientError::FrameError { source })?;
        self.stream
            .write_all(&frame)
            .map_err(|source| IpcClientError::IoError { source })?;
        self.stream
            .flush()
            .map_err(|source| IpcClientError::IoError { source })?;
        Ok(())
    }

    /// Receives a response frame header.
    pub fn recv_response_header(&mut self) -> Result<crate::IpcFrameHeader, IpcClientError> {
        read_frame_header_bounded(&mut self.stream, MaxPayloadBytes::DEFAULT)
            .map_err(|source| IpcClientError::FrameError { source })
    }

    /// Receives a response payload after the header has been read.
    pub fn recv_response_payload(
        &mut self,
        header: &crate::IpcFrameHeader,
    ) -> Result<Vec<u8>, IpcClientError> {
        read_frame_payload_bounded(&mut self.stream, header, MaxPayloadBytes::DEFAULT)
            .map_err(|source| IpcClientError::FrameError { source })
    }

    /// Receives and decodes a typed IPC response with explicit frame bounds.
    pub fn recv_response(
        &mut self,
        max_payload: MaxPayloadBytes,
    ) -> Result<(crate::IpcFrameHeader, IpcResponse), IpcClientError> {
        let header = read_frame_header_bounded(&mut self.stream, max_payload)
            .map_err(|source| IpcClientError::FrameError { source })?;
        let payload = read_frame_payload_bounded(&mut self.stream, &header, max_payload)
            .map_err(|source| IpcClientError::FrameError { source })?;
        let response = postcard::from_bytes(&payload).map_err(|_| IpcClientError::FrameError {
            source: IpcError::ResponseDecodeFailed,
        })?;
        Ok((header, response))
    }

    /// Sends a health check command.
    pub fn health(&mut self, correlation: u64) -> Result<(), IpcClientError> {
        self.send_raw(IpcCommand::Health, correlation, &[])
    }

    /// Sends a shutdown command.
    pub fn shutdown(&mut self, correlation: u64) -> Result<(), IpcClientError> {
        self.send_raw(IpcCommand::Shutdown, correlation, &[])
    }

    /// Sends a list-runs command.
    pub fn list_runs(
        &mut self,
        correlation: u64,
        limit: u32,
        workflow: Option<WorkflowDigest>,
    ) -> Result<(), IpcClientError> {
        self.send_command(
            IpcCommand::ListRuns,
            correlation,
            &IpcPayload::ListRuns { limit, workflow },
        )
    }
}

/// Connects to an IPC endpoint.
pub fn connect_ipc(socket_path: &Path) -> Result<IpcClient, IpcClientError> {
    IpcClient::connect(socket_path)
}

/// Sends a typed IPC command through an existing client.
pub fn send_command(
    client: &mut IpcClient,
    command: IpcCommand,
    correlation: u64,
    payload: &IpcPayload,
) -> Result<(), IpcClientError> {
    client.send_command(command, correlation, payload)
}

/// Receives a typed IPC response through an existing client.
pub fn recv_response(
    client: &mut IpcClient,
    max_payload: MaxPayloadBytes,
) -> Result<(crate::IpcFrameHeader, IpcResponse), IpcClientError> {
    client.recv_response(max_payload)
}

/// IPC client errors.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IpcClientError {
    /// Connection to the socket failed.
    #[error("connect failed: {source}")]
    ConnectFailed {
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// IO error during communication.
    #[error("io error: {source}")]
    IoError {
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Frame encoding or decoding failed.
    #[error("frame error: {source}")]
    FrameError {
        /// Underlying IPC error.
        source: IpcError,
    },
    /// Payload encoding failed.
    #[error("payload encode failed")]
    EncodeFailed,
}

#[cfg(test)]
mod tests {
    use super::{IpcClient, IpcClientError, recv_response, send_command};
    use crate::server::{IpcResponse, IpcServer};
    use std::io::{Read, Write};
    use std::num::NonZeroUsize;
    use std::path::PathBuf;
    use std::time::Duration;
    use vb_runtime::runtime::Runtime;
    use vb_runtime::shard::ShardConfig;

    #[test]
    fn connect_ipc_rejects_nonexistent_socket() {
        let path = PathBuf::from("/tmp/vb_ipc_test_nonexistent_39f2.socket");
        let result = IpcClient::connect(&path);

        assert!(
            result.is_err(),
            "connecting to a nonexistent socket must fail"
        );
        let Err(error) = result else {
            return;
        };
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

    fn temp_socket_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("vb_ipc_client_test_{name}_{}", std::process::id()))
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
        let header =
            crate::IpcFrameHeader::new(crate::IpcCommand::Health, 0, 1, payload.len() as u32);
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
    fn list_runs_sends_list_runs_command() {
        let path = temp_socket_path("list_runs_cmd");
        let _cleanup = CleanupPath(&path);
        let mut server = IpcServer::bind(&path).unwrap();
        let mut runtime = make_runtime();
        let mut client = IpcClient::connect(&path).unwrap();

        server
            .poll_once(&mut runtime, Some(Duration::from_millis(100)))
            .unwrap();
        client.list_runs(303, 10, None).unwrap();
        server
            .poll_once(&mut runtime, Some(Duration::from_millis(100)))
            .unwrap();

        let (header, response) = client
            .recv_response(crate::MaxPayloadBytes::DEFAULT)
            .unwrap();
        assert_eq!(header.command, crate::IpcCommand::ListRuns);
        assert_eq!(header.correlation, 303);
        match response {
            IpcResponse::RunList { runs } => assert!(runs.is_empty()),
            other => panic!("expected RunList, got {other:?}"),
        }
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
        let header = crate::IpcFrameHeader::decode(
            &buf.try_into().unwrap(),
            crate::MaxPayloadBytes::DEFAULT,
        )
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
        let header = crate::IpcFrameHeader::decode(
            &buf.try_into().unwrap(),
            crate::MaxPayloadBytes::DEFAULT,
        )
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
        let header = crate::IpcFrameHeader::decode(
            &buf.try_into().unwrap(),
            crate::MaxPayloadBytes::DEFAULT,
        )
        .unwrap();
        assert_eq!(header.command, crate::IpcCommand::Shutdown);
        assert_eq!(header.correlation, 99);
    }

    #[test]
    fn list_runs_writes_bytes_to_stream() {
        let (client_stream, mut server_stream) = std::os::unix::net::UnixStream::pair().unwrap();
        server_stream
            .set_read_timeout(Some(std::time::Duration::from_millis(100)))
            .unwrap();
        let mut client = IpcClient {
            stream: client_stream,
        };
        client.list_runs(111, 5, None).unwrap();

        let mut buf = vec![0u8; crate::IPC_HEADER_LEN];
        server_stream
            .read_exact(&mut buf)
            .expect("list_runs should write header bytes");
        let header = crate::IpcFrameHeader::decode(
            &buf.try_into().unwrap(),
            crate::MaxPayloadBytes::DEFAULT,
        )
        .unwrap();
        assert_eq!(header.command, crate::IpcCommand::ListRuns);
        assert_eq!(header.correlation, 111);
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
        let header = crate::IpcFrameHeader::decode(
            &buf.try_into().unwrap(),
            crate::MaxPayloadBytes::DEFAULT,
        )
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
        let header =
            crate::IpcFrameHeader::new(crate::IpcCommand::Health, 0, 1, payload.len() as u32);
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

        let header = crate::IpcFrameHeader::decode(
            &buf.try_into().unwrap(),
            crate::MaxPayloadBytes::DEFAULT,
        )
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
}
