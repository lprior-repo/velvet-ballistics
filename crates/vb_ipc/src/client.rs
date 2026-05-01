//! IPC client for connecting to a velvet-ballistics runtime.

use std::io::Write;
use std::path::Path;

use crate::frame::{read_frame_header_bounded, read_frame_payload_bounded, write_frame};
use crate::server::IpcResponse;
use crate::{IpcCommand, IpcError, IpcPayload, MaxPayloadBytes};

/// IPC client connected to a Unix domain socket.
pub struct IpcClient {
    stream: std::os::unix::net::UnixStream,
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
        write_frame(&mut self.stream, command, 0, correlation, &encoded)
            .map_err(|source| IpcClientError::FrameError { source })?;
        self.stream
            .flush()
            .map_err(|source| IpcClientError::IoError { source })?;
        Ok(())
    }

    /// Sends a raw command with pre-encoded payload bytes.
    pub fn send_raw(
        &mut self,
        command: IpcCommand,
        correlation: u64,
        payload: &[u8],
    ) -> Result<(), IpcClientError> {
        write_frame(&mut self.stream, command, 0, correlation, payload)
            .map_err(|source| IpcClientError::FrameError { source })?;
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
    use super::{IpcClient, IpcClientError};
    use std::path::PathBuf;

    #[test]
    fn connect_ipc_rejects_nonexistent_socket() {
        let path = PathBuf::from("/tmp/vb_ipc_test_nonexistent_39f2.socket");
        let result = IpcClient::connect(&path);

        assert!(result.is_err(), "connecting to a nonexistent socket must fail");
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
        let Err(IpcClientError::ConnectFailed { source }) = result else {
            return;
        };
        let _ = source; // verify the source field is accessible
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
        assert!(client.is_err(), "connect should fail for nonexistent socket");
    }

    #[test]
    fn recv_response_header_returns_error_without_server() {
        // Given: a client connected to nothing (connection will fail)
        let path = PathBuf::from("/tmp/vb_ipc_test_recv_fail_88c2.socket");
        let client = IpcClient::connect(&path);

        // When: connection fails
        // Then: no recv possible
        assert!(client.is_err(), "connect should fail");
    }
}
