#![forbid(unsafe_code)]
//! IPC server errors.

use crate::IpcError;

/// IPC server errors.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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
    /// A previous response still owns the bounded output buffer.
    #[error("previous IPC response is still pending")]
    ResponsePending,
    /// Failed to read a request from a peer.
    #[error("request read failed: {source}")]
    RequestReadFailed {
        /// Underlying IO error.
        source: std::io::Error,
    },
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

impl PartialEq for IpcServerError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::BindFailed { source: a }, Self::BindFailed { source: b }) => {
                a.kind() == b.kind() && a.to_string() == b.to_string()
            }
            (Self::PollFailed { source: a }, Self::PollFailed { source: b }) => {
                a.kind() == b.kind() && a.to_string() == b.to_string()
            }
            (Self::AcceptFailed { source: a }, Self::AcceptFailed { source: b }) => {
                a.kind() == b.kind() && a.to_string() == b.to_string()
            }
            (Self::TooManyClients, Self::TooManyClients) => true,
            (Self::ResponseEncodeFailed, Self::ResponseEncodeFailed) => true,
            (Self::ResponsePending, Self::ResponsePending) => true,
            (Self::RequestReadFailed { source: a }, Self::RequestReadFailed { source: b }) => {
                a.kind() == b.kind() && a.to_string() == b.to_string()
            }
            (Self::ResponseWriteFailed { source: a }, Self::ResponseWriteFailed { source: b }) => {
                a.kind() == b.kind() && a.to_string() == b.to_string()
            }
            (Self::IncompleteFrame, Self::IncompleteFrame) => true,
            (Self::ReadBufferTooLarge, Self::ReadBufferTooLarge) => true,
            (Self::FrameInvalid { source: a }, Self::FrameInvalid { source: b }) => a == b,
            _ => false,
        }
    }
}

impl Eq for IpcServerError {}

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
            | Self::ResponsePending
            | Self::RequestReadFailed { .. }
            | Self::ResponseWriteFailed { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests;
