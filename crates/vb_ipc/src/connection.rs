//! Connection handling for the IPC server.

use arrayvec::ArrayVec;
use mio::net::UnixStream;
use mio::{Interest, Poll, Registry, Token};
use std::io::{Read, Write};
use vb_core::ids::WorkflowDigest;
use vb_core::workflow::CompiledWorkflow;
use vb_runtime::runtime::Runtime;

use crate::{
    IpcCommand, IpcError, IpcFrameHeader, IpcResponse, IpcServerError, MaxPayloadBytes,
    IPC_HEADER_LEN,
};

use crate::session::{WorkflowResolutionError, WorkflowResolver};

const MAX_CLIENTS: usize = 256;
const READ_CHUNK_BYTES: usize = 4096;

/// Client connection state for the IPC server.
pub struct ClientConnection {
    /// Unix stream for this client.
    pub stream: mio::net::UnixStream,
    /// Pending read bytes.
    pub read_buffer: Vec<u8>,
    /// Pending write bytes.
    pub write_buffer: Vec<u8>,
}

impl ClientConnection {
    /// Creates a new client connection.
    pub fn new(stream: mio::net::UnixStream) -> Self {
        Self {
            stream,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
        }
    }
}

/// Constants for server token management.
pub const SERVER_TOKEN: Token = Token(0);
pub const FIRST_CLIENT_TOKEN: usize = 1;

/// Accepts a new client connection.
pub fn accept_client(
    poll: &Poll,
    listener: &mio::net::UnixListener,
    clients: &mut std::collections::HashMap<usize, ClientConnection>,
    next_token: &mut usize,
) -> Result<Token, IpcServerError> {
    if clients.len() >= MAX_CLIENTS {
        return Err(IpcServerError::TooManyClients);
    }

    let (stream, _addr) = listener
        .accept()
        .map_err(|source| IpcServerError::AcceptFailed { source })?;

    let token_val = next_token
        .checked_add(1)
        .ok_or(IpcServerError::TooManyClients)?;
    let token = Token(*next_token);
    *next_token = token_val;

    let mut client = ClientConnection::new(stream);

    poll.registry()
        .register(&mut client.stream, token, Interest::READABLE)
        .map_err(|source| IpcServerError::PollFailed { source })?;

    clients.insert(token.0, client);
    Ok(token)
}

/// Handles a writable event for a client.
pub fn handle_writable(
    poll: &Poll,
    token_index: usize,
    client: &mut ClientConnection,
) -> Result<bool, IpcServerError> {
    if client.write_buffer.is_empty() {
        return Ok(false);
    }

    let written = match client.stream.write(&client.write_buffer) {
        Ok(n) => n,
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
        Err(_) => return Ok(true),
    };

    client.write_buffer.drain(..written);

    if client.write_buffer.is_empty() {
        let token = Token(token_index);
        poll.registry()
            .reregister(&mut client.stream, token, Interest::READABLE)
            .map_err(|source| IpcServerError::PollFailed { source })?;
    }

    Ok(false)
}

/// Reads bytes from a client connection.
pub fn read_client_bytes(
    client: &mut ClientConnection,
    poll: &Poll,
    token: Token,
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> Result<bool, IpcServerError> {
    let registry = poll
        .registry()
        .try_clone()
        .map_err(|source| IpcServerError::PollFailed { source })?;

    let mut temp_buf = [0u8; READ_CHUNK_BYTES];
    let bytes_read = match client.stream.read(&mut temp_buf) {
        Ok(0) => return Ok(true),
        Ok(n) => n,
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
        Err(_) => return Ok(true),
    };

    append_read_bytes(&mut client.read_buffer, &temp_buf, bytes_read)?;

    while client.read_buffer.len() >= IPC_HEADER_LEN {
        let header_bytes = read_buffer_header(&client.read_buffer)?;
        let header = match IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT) {
            Ok(h) => h,
            Err(error) => {
                let response = frame_error_response(error);
                let fallback_header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0);
                send_response(
                    &mut client.stream,
                    &mut client.write_buffer,
                    &registry,
                    token,
                    &fallback_header,
                    &response,
                )?;
                return Ok(true);
            }
        };

        let total_len = frame_total_len(&header)?;
        if client.read_buffer.len() < total_len {
            return Ok(false);
        }

        let payload_bytes = extract_payload(&mut client.read_buffer, total_len)?;

        let response = dispatch_command_with_resolver(
            &header,
            &payload_bytes,
            runtime,
            borrow_workflow_resolver(&mut resolver),
        );
        send_response(
            &mut client.stream,
            &mut client.write_buffer,
            &registry,
            token,
            &header,
            &response,
        )?;
    }

    Ok(false)
}

/// Appends read bytes to a read buffer.
pub fn append_read_bytes(
    read_buffer: &mut Vec<u8>,
    temp_buf: &[u8; READ_CHUNK_BYTES],
    bytes_read: usize,
) -> Result<(), IpcServerError> {
    let read_slice = temp_buf
        .get(..bytes_read)
        .ok_or(IpcServerError::FrameInvalid {
            source: IpcError::PayloadLengthMismatch {
                header: READ_CHUNK_BYTES,
                actual: bytes_read,
            },
        })?;
    let next_len = read_buffer
        .len()
        .checked_add(read_slice.len())
        .ok_or(IpcServerError::ReadBufferTooLarge)?;
    let max_buffer = IPC_HEADER_LEN
        .checked_add(MaxPayloadBytes::DEFAULT.get())
        .ok_or(IpcServerError::ReadBufferTooLarge)?;
    if next_len > max_buffer {
        return Err(IpcServerError::ReadBufferTooLarge);
    }
    read_buffer.extend_from_slice(read_slice);
    Ok(())
}

/// Extracts the header bytes from a read buffer.
pub fn read_buffer_header(read_buffer: &[u8]) -> Result<[u8; IPC_HEADER_LEN], IpcServerError> {
    let header_slice = read_buffer
        .get(..IPC_HEADER_LEN)
        .ok_or(IpcServerError::IncompleteFrame)?;
    <[u8; IPC_HEADER_LEN]>::try_from(header_slice).map_err(|_| IpcServerError::IncompleteFrame)
}

/// Computes total frame length from header.
pub fn frame_total_len(header: &IpcFrameHeader) -> Result<usize, IpcServerError> {
    let payload_len =
        usize::try_from(header.payload_len).map_err(|_| IpcServerError::FrameInvalid {
            source: IpcError::PayloadLengthOutOfRange {
                actual: header.payload_len,
            },
        })?;
    let total_len = IPC_HEADER_LEN
        .checked_add(payload_len)
        .ok_or(IpcServerError::ReadBufferTooLarge)?;
    Ok(total_len)
}

/// Extracts payload bytes from a read buffer.
pub fn extract_payload(
    read_buffer: &mut Vec<u8>,
    total_len: usize,
) -> Result<Vec<u8>, IpcServerError> {
    if read_buffer.len() < total_len {
        return Err(IpcServerError::IncompleteFrame);
    }
    // Keep unread bytes in the connection buffer and return only the consumed payload.
    let remaining = read_buffer.split_off(total_len);
    let mut frame = std::mem::replace(read_buffer, remaining);
    Ok(frame.split_off(IPC_HEADER_LEN))
}

/// Converts an IpcError to a frame error response.
pub fn frame_error_response(error: IpcError) -> IpcResponse {
    IpcResponse::FrameError {
        message: error.to_string(),
    }
}

/// Removes a client connection.
pub fn remove_client(
    poll: &Poll,
    token_index: usize,
    clients: &mut std::collections::HashMap<usize, ClientConnection>,
) {
    if let Some(mut client) = clients.remove(&token_index) {
        let _ = poll.registry().deregister(&mut client.stream);
    }
}

/// Sends a response to a client.
pub fn send_response(
    stream: &mut mio::net::UnixStream,
    write_buffer: &mut Vec<u8>,
    registry: &Registry,
    token: Token,
    request_header: &IpcFrameHeader,
    response: &IpcResponse,
) -> Result<(), IpcServerError> {
    let payload_bytes =
        postcard::to_allocvec(response).map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    let payload_len =
        u32::try_from(payload_bytes.len()).map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    // Build the response frame header via IpcFrameHeader::encode (uses byteorder internally).
    let header = IpcFrameHeader::new(
        request_header.command,
        0,
        request_header.correlation,
        payload_len,
    );
    let header_bytes = header
        .encode()
        .map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    write_buffer.clear();
    write_buffer.extend_from_slice(&header_bytes);
    write_buffer.extend_from_slice(&payload_bytes);

    let written = match stream.write(write_buffer) {
        Ok(count) => count,
        Err(ref source) if source.kind() == std::io::ErrorKind::WouldBlock => 0,
        Err(source) => return Err(IpcServerError::ResponseWriteFailed { source }),
    };
    if written > 0 {
        write_buffer.drain(..written);
    }
    if write_buffer.is_empty() {
        match stream.flush() {
            Ok(()) => {}
            Err(ref source) if source.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(source) => return Err(IpcServerError::ResponseWriteFailed { source }),
        }
    } else {
        registry
            .reregister(stream, token, Interest::READABLE | Interest::WRITABLE)
            .map_err(|source| IpcServerError::PollFailed { source })?;
    }

    Ok(())
}

// Re-export dispatch functions for use by connection handling
pub use crate::dispatch::{
    borrow_workflow_resolver, dispatch_command, dispatch_command_with_resolver,
};
