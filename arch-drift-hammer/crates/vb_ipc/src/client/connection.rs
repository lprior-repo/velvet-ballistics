#![forbid(unsafe_code)]
//! IPC client for connecting to a velvet_ballistics runtime.

use std::io::Write;
use std::path::Path;

use super::error::IpcClientError;
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
