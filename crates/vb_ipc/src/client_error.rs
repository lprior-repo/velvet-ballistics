//! IPC client errors.

use crate::IpcError;

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
