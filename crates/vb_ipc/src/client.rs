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
