#![forbid(unsafe_code)]
//! IPC server errors.

use crate::IpcError;

/// IPC server errors.
#[derive(Debug, thiserror::Error)]
pub enum IpcServerError {
    /// Failed to bind to the socket path.
    #[error("bind failed: {source}")]
    BindFailed {
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Poll operation failed.
    #[error("poll failed: {source}")]
    PollFailed {
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Accept operation failed.
    #[error("accept failed: {source}")]
    AcceptFailed {
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Too many concurrent clients.
    #[error("too many clients")]
    TooManyClients,
    /// Failed to encode response payload.
    #[error("response encode failed")]
    ResponseEncodeFailed,
    /// Failed to write response to client.
    #[error("response write failed: {source}")]
    ResponseWriteFailed {
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Client frame did not contain enough bytes for the declared frame.
    #[error("incomplete IPC frame")]
    IncompleteFrame,
    /// Client read buffer exceeded the configured single-frame bound.
    #[error("IPC read buffer exceeded configured frame bound")]
    ReadBufferTooLarge,
    /// Client frame failed typed validation.
    #[error("invalid IPC frame: {source}")]
    FrameInvalid {
        /// Typed IPC frame error.
        source: IpcError,
    },
}

impl IpcServerError {
    /// Returns the stable section 17 runtime code when this server error has a direct mapping.
    #[must_use]
    pub fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::IncompleteFrame => Some(IpcError::IPC_FRAME_INVALID_RUNTIME_CODE),
            Self::ReadBufferTooLarge => Some(IpcError::IPC_PAYLOAD_TOO_LARGE_RUNTIME_CODE),
            Self::FrameInvalid { source } => source.runtime_code(),
            Self::TooManyClients => Some(IpcError::QUEUE_FULL_RUNTIME_CODE),
            Self::BindFailed { .. }
            | Self::PollFailed { .. }
            | Self::AcceptFailed { .. }
            | Self::ResponseEncodeFailed
            | Self::ResponseWriteFailed { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IpcError, IpcServerError};
    use std::io;

    fn io_error(kind: io::ErrorKind, msg: &str) -> io::Error {
        io::Error::new(kind, msg)
    }

    // ── Display message tests ──

    #[test]
    fn bind_failed_display_mentions_bind() {
        let err = IpcServerError::BindFailed {
            source: io_error(io::ErrorKind::AddrInUse, "addr in use"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("bind failed"),
            "expected 'bind failed' in '{msg}'"
        );
    }

    #[test]
    fn poll_failed_display_mentions_poll() {
        let err = IpcServerError::PollFailed {
            source: io_error(io::ErrorKind::Interrupted, "interrupted"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("poll failed"),
            "expected 'poll failed' in '{msg}'"
        );
    }

    #[test]
    fn accept_failed_display_mentions_accept() {
        let err = IpcServerError::AcceptFailed {
            source: io_error(io::ErrorKind::ConnectionRefused, "refused"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("accept failed"),
            "expected 'accept failed' in '{msg}'"
        );
    }

    #[test]
    fn too_many_clients_display() {
        let err = IpcServerError::TooManyClients;
        let msg = err.to_string();
        assert!(
            msg.contains("too many clients"),
            "expected 'too many clients' in '{msg}'"
        );
    }

    #[test]
    fn response_encode_failed_display() {
        let err = IpcServerError::ResponseEncodeFailed;
        let msg = err.to_string();
        assert!(
            msg.contains("response encode failed"),
            "expected 'response encode failed' in '{msg}'"
        );
    }

    #[test]
    fn response_write_failed_display() {
        let err = IpcServerError::ResponseWriteFailed {
            source: io_error(io::ErrorKind::BrokenPipe, "pipe broke"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("response write failed"),
            "expected 'response write failed' in '{msg}'"
        );
    }

    #[test]
    fn incomplete_frame_display() {
        let err = IpcServerError::IncompleteFrame;
        let msg = err.to_string();
        assert!(
            msg.contains("incomplete IPC frame"),
            "expected 'incomplete IPC frame' in '{msg}'"
        );
    }

    #[test]
    fn read_buffer_too_large_display() {
        let err = IpcServerError::ReadBufferTooLarge;
        let msg = err.to_string();
        assert!(
            msg.contains("read buffer exceeded"),
            "expected 'read buffer exceeded' in '{msg}'"
        );
    }

    #[test]
    fn frame_invalid_display_wraps_source() {
        let err = IpcServerError::FrameInvalid {
            source: IpcError::InvalidMagic { actual: 0xDEAD },
        };
        let msg = err.to_string();
        assert!(
            msg.contains("invalid IPC frame"),
            "expected 'invalid IPC frame' in '{msg}'"
        );
        assert!(
            msg.contains("magic"),
            "expected inner error detail in '{msg}'"
        );
    }

    // ── Runtime code mapping tests ──

    #[test]
    fn runtime_code_incomplete_frame_maps_to_frame_invalid() {
        let err = IpcServerError::IncompleteFrame;
        assert_eq!(err.runtime_code(), Some("IPC_FRAME_INVALID"));
    }

    #[test]
    fn runtime_code_read_buffer_too_large_maps_to_payload_too_large() {
        let err = IpcServerError::ReadBufferTooLarge;
        assert_eq!(err.runtime_code(), Some("IPC_PAYLOAD_TOO_LARGE"));
    }

    #[test]
    fn runtime_code_frame_invalid_delegates_to_source() {
        let err = IpcServerError::FrameInvalid {
            source: IpcError::InvalidMagic { actual: 0 },
        };
        assert_eq!(err.runtime_code(), Some("IPC_FRAME_INVALID"));
    }

    #[test]
    fn runtime_code_frame_invalid_delegates_to_payload_too_large_source() {
        let err = IpcServerError::FrameInvalid {
            source: IpcError::PayloadTooLarge {
                actual: 99,
                limit: 10,
            },
        };
        assert_eq!(err.runtime_code(), Some("IPC_PAYLOAD_TOO_LARGE"));
    }

    #[test]
    fn runtime_code_too_many_clients_maps_to_queue_full() {
        let err = IpcServerError::TooManyClients;
        assert_eq!(err.runtime_code(), Some("QUEUE_FULL"));
    }

    #[test]
    fn runtime_code_bind_failed_is_none() {
        let err = IpcServerError::BindFailed {
            source: io_error(io::ErrorKind::AddrInUse, "addr"),
        };
        assert_eq!(err.runtime_code(), None);
    }

    #[test]
    fn runtime_code_poll_failed_is_none() {
        let err = IpcServerError::PollFailed {
            source: io_error(io::ErrorKind::Interrupted, "intr"),
        };
        assert_eq!(err.runtime_code(), None);
    }

    #[test]
    fn runtime_code_accept_failed_is_none() {
        let err = IpcServerError::AcceptFailed {
            source: io_error(io::ErrorKind::ConnectionRefused, "ref"),
        };
        assert_eq!(err.runtime_code(), None);
    }

    #[test]
    fn runtime_code_response_encode_failed_is_none() {
        let err = IpcServerError::ResponseEncodeFailed;
        assert_eq!(err.runtime_code(), None);
    }

    #[test]
    fn runtime_code_response_write_failed_is_none() {
        let err = IpcServerError::ResponseWriteFailed {
            source: io_error(io::ErrorKind::BrokenPipe, "pipe"),
        };
        assert_eq!(err.runtime_code(), None);
    }

    #[test]
    fn runtime_code_frame_invalid_with_disconnected_source_is_none() {
        let err = IpcServerError::FrameInvalid {
            source: IpcError::Disconnected,
        };
        assert_eq!(err.runtime_code(), None);
    }
}
