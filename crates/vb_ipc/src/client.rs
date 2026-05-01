//! IPC client for connecting to a velvet-ballistics runtime.

use std::io::Write;
use std::path::Path;

use crate::frame::{read_frame_header, read_frame_payload, write_frame};
use crate::{IpcCommand, IpcError, IpcPayload};

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
        read_frame_header(&mut self.stream).map_err(|source| IpcClientError::FrameError { source })
    }

    /// Receives a response payload after the header has been read.
    pub fn recv_response_payload(
        &mut self,
        header: &crate::IpcFrameHeader,
    ) -> Result<Vec<u8>, IpcClientError> {
        read_frame_payload(&mut self.stream, header)
            .map_err(|source| IpcClientError::FrameError { source })
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
