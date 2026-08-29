#![forbid(unsafe_code)]
//! I/O operations for the IPC server — read and write handlers.

use arrayvec::ArrayVec;
use mio::{Interest, Token};
use std::io::{Read, Write};
use vb_runtime::runtime::Runtime;

use super::dispatch::dispatch_command_with_resolver;
use super::error::IpcServerError;
use super::helpers::{AWAITING_MAGIC_MAX_BYTES, MagicValidationState, validate_magic_early};
use super::{ClientConnection, IpcServer, WorkflowResolver};
use crate::IPC_HEADER_LEN;
use crate::MaxPayloadBytes;
use crate::{IpcCommand, IpcFrameHeader, IpcPayload};

use super::{
    append_read_bytes, borrow_workflow_resolver, extract_payload, frame_error_response,
    frame_total_len, read_buffer_header, send_response,
};

const READ_CHUNK_BYTES: usize = 4096;

impl IpcServer {
    pub(crate) fn handle_readable(
        &mut self,
        token_index: usize,
        runtime: &mut Runtime,
        resolver: Option<&mut dyn WorkflowResolver>,
    ) -> Result<bool, IpcServerError> {
        let registry = self
            .poll
            .registry()
            .try_clone()
            .map_err(|source| IpcServerError::PollFailed { source })?;
        let token = Token(token_index);

        let Some(client) = self.clients.get_mut(&token_index) else {
            return Ok(true);
        };
        let mut resolver = resolver;

        let mut temp_buf = [0u8; READ_CHUNK_BYTES];
        let bytes_read = match client.stream.read(&mut temp_buf) {
            Ok(0) => return Ok(true),
            Ok(n) => n,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(_) => return Ok(true),
        };

        append_read_bytes(&mut client.read_buffer, &temp_buf, bytes_read)?;

        // Early magic validation — reject immediately if magic bytes are invalid.
        // This prevents unbounded buffer growth from a malicious peer.
        if client.magic_state == MagicValidationState::AwaitingMagic
            && client.read_buffer.len() >= AWAITING_MAGIC_MAX_BYTES
        {
            match validate_magic_early(&client.read_buffer) {
                Ok(MagicValidationState::AwaitingMagic) => {
                    // Not enough bytes yet — wait for more.
                    return Ok(false);
                }
                Ok(MagicValidationState::MagicValidated) => {
                    client.magic_state = MagicValidationState::MagicValidated;
                }
                Err(_error) => {
                    // Invalid magic — close connection immediately.
                    return Ok(true);
                }
            }
        }

        // Only process frames after magic is validated.
        if client.magic_state != MagicValidationState::MagicValidated {
            return Ok(false);
        }

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

    pub(crate) fn handle_writable(&mut self, token_index: usize) -> Result<bool, IpcServerError> {
        let Some(client) = self.clients.get_mut(&token_index) else {
            return Ok(true);
        };

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
            self.poll
                .registry()
                .reregister(&mut client.stream, token, Interest::READABLE)
                .map_err(|source| IpcServerError::PollFailed { source })?;
        }

        Ok(false)
    }
}
