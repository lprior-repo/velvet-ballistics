#![forbid(unsafe_code)]
//! Single-flight framing and bounded partial I/O.
use super::dispatch::dispatch_command_with_resolver;
use super::helpers::{MagicValidationState, validate_magic_early};
use super::{
    ClientConnection, ClosePolicy, IpcResponse, IpcServer, IpcServerError, WorkflowResolver,
};
use crate::{IPC_HEADER_LEN, IpcError, IpcFrameHeader, MaxPayloadBytes};
use mio::{Interest, Token};
use std::io::{Read, Write};
use vb_runtime::runtime::Runtime;

const IO_OPERATIONS_PER_TURN: usize = 64;

enum ReadProgress {
    Incomplete,
    Closed,
    Request(IpcFrameHeader),
}

struct ReadyRequest {
    token: usize,
    header: IpcFrameHeader,
}

impl IpcServer {
    pub(crate) fn handle_readable(
        &mut self,
        token: usize,
        runtime: &mut Runtime,
        resolver: Option<&mut dyn WorkflowResolver>,
    ) -> Result<bool, IpcServerError> {
        let registry = clone_registry(&self.poll)?;
        match self.read_client_request(token) {
            Ok(ReadProgress::Closed) => Ok(true),
            Ok(ReadProgress::Incomplete) => Ok(false),
            Ok(ReadProgress::Request(header)) => {
                self.dispatch_request(ReadyRequest { token, header }, runtime, resolver, &registry)
            }
            Err(IpcServerError::FrameInvalid { source }) => {
                self.reject_frame(token, source, &registry)
            }
            Err(error) => Err(error),
        }
    }

    fn read_client_request(&mut self, token: usize) -> Result<ReadProgress, IpcServerError> {
        let Some(client) = self.clients.get_mut(&token) else {
            return Ok(ReadProgress::Closed);
        };
        if !client.write_buffer.is_empty() {
            return Ok(ReadProgress::Incomplete);
        }
        read_request(client)
    }

    fn dispatch_request(
        &mut self,
        request: ReadyRequest,
        runtime: &mut Runtime,
        resolver: Option<&mut dyn WorkflowResolver>,
        registry: &mio::Registry,
    ) -> Result<bool, IpcServerError> {
        let Some(client) = self.clients.get_mut(&request.token) else {
            return Ok(true);
        };
        let payload = super::extract_payload(
            &mut client.read_buffer,
            super::frame_total_len(&request.header)?,
        )?;
        let response = dispatch_command_with_resolver(&request.header, &payload, runtime, resolver);
        let close = queue_response(client, registry, request.token, &request.header, &response)?;
        self.shutdown_requested = matches!(response, IpcResponse::ShuttingDown);
        Ok(close)
    }

    fn reject_frame(
        &mut self,
        token: usize,
        source: IpcError,
        registry: &mio::Registry,
    ) -> Result<bool, IpcServerError> {
        if matches!(source, IpcError::InvalidMagic { .. }) {
            return Ok(true);
        }
        let Some(client) = self.clients.get_mut(&token) else {
            return Ok(true);
        };
        client.close_policy = ClosePolicy::AfterResponse;
        let header = IpcFrameHeader::new(crate::IpcCommand::Health, 0, 0, 0);
        queue_response(
            client,
            registry,
            token,
            &header,
            &super::frame_error_response(source),
        )
    }

    pub(crate) fn handle_writable(&mut self, token: usize) -> Result<bool, IpcServerError> {
        let Some(client) = self.clients.get_mut(&token) else {
            return Ok(true);
        };
        for _ in 0..IO_OPERATIONS_PER_TURN {
            if client.write_buffer.is_empty() {
                return finish_response(client, self.poll.registry(), token);
            }
            if let Some(close) = write_pending_chunk(client)? {
                return Ok(close);
            }
        }
        client.write_ready = true;
        Ok(false)
    }
}

fn clone_registry(poll: &mio::Poll) -> Result<mio::Registry, IpcServerError> {
    poll.registry()
        .try_clone()
        .map_err(|source| IpcServerError::PollFailed { source })
}

fn write_pending_chunk(client: &mut ClientConnection) -> Result<Option<bool>, IpcServerError> {
    match client.stream.write(&client.write_buffer) {
        Ok(0) => Ok(Some(true)),
        Ok(count) => {
            client.write_buffer.drain(..count).for_each(|_| ());
            Ok(None)
        }
        Err(source) if source.kind() == std::io::ErrorKind::WouldBlock => {
            client.write_ready = false;
            Ok(Some(false))
        }
        Err(source) if source.kind() == std::io::ErrorKind::Interrupted => Ok(None),
        Err(source) => Err(IpcServerError::ResponseWriteFailed { source }),
    }
}

fn read_request(client: &mut ClientConnection) -> Result<ReadProgress, IpcServerError> {
    for _ in 0..IO_OPERATIONS_PER_TURN {
        let remaining = request_remaining(client)?;
        if remaining == 0 {
            return completed_request(client);
        }
        if let Some(progress) = read_request_chunk(client, remaining)? {
            return Ok(progress);
        }
    }
    client.read_ready = true;
    Ok(ReadProgress::Incomplete)
}

fn completed_request(client: &ClientConnection) -> Result<ReadProgress, IpcServerError> {
    let bytes = super::read_buffer_header(&client.read_buffer)?;
    IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT)
        .map(ReadProgress::Request)
        .map_err(|source| IpcServerError::FrameInvalid { source })
}

fn read_request_chunk(
    client: &mut ClientConnection,
    remaining: usize,
) -> Result<Option<ReadProgress>, IpcServerError> {
    let mut scratch = [0; 4096];
    let writable = scratch
        .get_mut(..remaining.min(4096))
        .ok_or(IpcServerError::ReadBufferTooLarge)?;
    match client.stream.read(writable) {
        Ok(0) => Ok(Some(ReadProgress::Closed)),
        Ok(count) => {
            super::append_read_bytes(&mut client.read_buffer, &scratch, count)?;
            Ok(None)
        }
        Err(source) if source.kind() == std::io::ErrorKind::WouldBlock => {
            client.read_ready = false;
            Ok(Some(ReadProgress::Incomplete))
        }
        Err(source) if source.kind() == std::io::ErrorKind::Interrupted => Ok(None),
        Err(source) => Err(IpcServerError::RequestReadFailed { source }),
    }
}

fn request_remaining(client: &mut ClientConnection) -> Result<usize, IpcServerError> {
    let length = client.read_buffer.len();
    if length < 4 {
        return Ok(4_usize.saturating_sub(length));
    }
    client.magic_state = validate_magic_early(&client.read_buffer)
        .map_err(|source| IpcServerError::FrameInvalid { source })?;
    if length < IPC_HEADER_LEN {
        return Ok(IPC_HEADER_LEN.saturating_sub(length));
    }
    let bytes = super::read_buffer_header(&client.read_buffer)?;
    let header = IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT)
        .map_err(|source| IpcServerError::FrameInvalid { source })?;
    Ok(super::frame_total_len(&header)?.saturating_sub(length))
}

fn queue_response(
    client: &mut ClientConnection,
    registry: &mio::Registry,
    token: usize,
    header: &IpcFrameHeader,
    response: &IpcResponse,
) -> Result<bool, IpcServerError> {
    super::send_response(
        &mut client.stream,
        &mut client.write_buffer,
        registry,
        Token(token),
        header,
        response,
    )?;
    client.magic_state = MagicValidationState::AwaitingMagic;
    client.read_ready = true;
    client.write_ready = !client.write_buffer.is_empty();
    Ok(client.close_policy == ClosePolicy::AfterResponse && client.write_buffer.is_empty())
}

fn finish_response(
    client: &mut ClientConnection,
    registry: &mio::Registry,
    token: usize,
) -> Result<bool, IpcServerError> {
    client.write_ready = false;
    if client.close_policy == ClosePolicy::AfterResponse {
        return Ok(true);
    }
    registry
        .reregister(&mut client.stream, Token(token), Interest::READABLE)
        .map_err(|source| IpcServerError::PollFailed { source })?;
    client.read_ready = true;
    Ok(false)
}
