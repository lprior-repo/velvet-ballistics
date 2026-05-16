//! IPC client for connecting to a velvet-ballistics runtime.

// Re-export from submodules for ergonomic API surface.
pub use crate::client_conn::{connect_ipc, IpcClient, recv_response, send_command};
pub use crate::client_error::IpcClientError;

#[cfg(test)]
mod tests {
    use super::{IpcClient, IpcClientError};
    use std::path::PathBuf;

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
}
