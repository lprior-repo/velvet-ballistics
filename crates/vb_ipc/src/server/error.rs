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
