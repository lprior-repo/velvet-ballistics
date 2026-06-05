#![forbid(unsafe_code)]
//! IpcServer implementation.

#![allow(unused_imports)]

use arrayvec::ArrayVec;
use mio::net::UnixListener;
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;

use vb_core::action::{ActionFailure, ActionFailureCode, ActionTicket};
use vb_core::ids::{ActionId, SeqNo, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::CompiledWorkflow;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{AskAnswer, AskTicket};
use vb_runtime::trace::TraceEvent;

use super::dispatch::{dispatch_command_with_resolver, serve_ipc_with_resolver};
use super::error::IpcServerError;
use super::handlers::{
    handle_answer_ask, handle_cancel_run, handle_complete_action, handle_fail_action,
    handle_health, handle_inspect_run, handle_list_events, handle_shutdown, handle_submit_run,
    submit_resolved_workflow,
};
use super::trace::handle_drain_trace;
use crate::IPC_HEADER_LEN;
use crate::IpcError;
use crate::MaxPayloadBytes;
use crate::{
    IpcActionOutputPayload, IpcCommand, IpcFrameHeader, IpcPayload, IpcTraceEvent,
    IpcTraceEventKind, SubmitRunPayload,
};

use super::helpers::{AWAITING_MAGIC_MAX_BYTES, MagicValidationState, validate_magic_early};
use super::{
    ClientConnection, IpcResponse, IpcServer, WorkflowResolutionError, WorkflowResolver,
    append_read_bytes, borrow_workflow_resolver, extract_payload, frame_error_response,
    frame_total_len, read_buffer_header, send_response,
};

const SERVER_TOKEN: Token = Token(0);
const FIRST_CLIENT_TOKEN: usize = 1;
pub(crate) const MAX_CLIENTS: usize = 256;
const READ_CHUNK_BYTES: usize = 4096;

impl IpcServer {
    /// Creates a new IPC server bound to the given Unix socket path.
    pub fn bind(socket_path: &Path) -> Result<Self, IpcServerError> {
        if socket_path.exists() {
            std::fs::remove_file(socket_path)
                .map_err(|source| IpcServerError::BindFailed { source })?;
        }

        let mut listener = UnixListener::bind(socket_path)
            .map_err(|source| IpcServerError::BindFailed { source })?;

        let poll = Poll::new().map_err(|source| IpcServerError::PollFailed { source })?;

        poll.registry()
            .register(&mut listener, SERVER_TOKEN, Interest::READABLE)
            .map_err(|source| IpcServerError::PollFailed { source })?;

        let events = Events::with_capacity(MAX_CLIENTS);

        Ok(Self {
            poll,
            listener,
            events,
            clients: HashMap::new(),
            next_token: FIRST_CLIENT_TOKEN,
            #[cfg(test)]
            test_poll_result: None,
        })
    }

    /// Polls for events once, dispatches commands, returns false when shutdown.
    pub fn poll_once(
        &mut self,
        runtime: &mut Runtime,
        timeout: Option<std::time::Duration>,
    ) -> Result<bool, IpcServerError> {
        self.poll_once_with_resolver(runtime, timeout, None)
    }

    /// Polls for events once with workflow resolution available for submit commands.
    pub fn poll_once_with_resolver(
        &mut self,
        runtime: &mut Runtime,
        timeout: Option<std::time::Duration>,
        resolver: Option<&mut dyn WorkflowResolver>,
    ) -> Result<bool, IpcServerError> {
        self.poll
            .poll(&mut self.events, timeout)
            .map_err(|source| IpcServerError::PollFailed { source })?;

        let mut pending: ArrayVec<(Token, bool, bool), MAX_CLIENTS> = ArrayVec::new();
        for event in &self.events {
            pending
                .try_push((event.token(), event.is_readable(), event.is_writable()))
                .map_err(|_| IpcServerError::TooManyClients)?;
        }
        let mut resolver = resolver;
        for (token, readable, writable) in pending {
            if token == SERVER_TOKEN {
                self.accept_client()?;
                continue;
            }

            let token_index = token.0;

            if writable {
                let should_remove = self.handle_writable(token_index)?;
                if should_remove {
                    self.remove_client(token_index);
                    continue;
                }
            }

            if readable {
                let resolver_ref = borrow_workflow_resolver(&mut resolver);
                let should_remove = self.handle_readable(token_index, runtime, resolver_ref)?;
                if should_remove {
                    self.remove_client(token_index);
                }
            }
        }

        #[cfg(test)]
        if let Some(result) = self.test_poll_result.take() {
            return result;
        }
        Ok(true)
    }

    pub(crate) fn accept_client(&mut self) -> Result<(), IpcServerError> {
        if self.clients.len() >= MAX_CLIENTS {
            return Err(IpcServerError::TooManyClients);
        }

        let (stream, _addr) = self
            .listener
            .accept()
            .map_err(|source| IpcServerError::AcceptFailed { source })?;

        let token_val = self
            .next_token
            .checked_add(1)
            .ok_or(IpcServerError::TooManyClients)?;
        let token = Token(self.next_token);
        self.next_token = token_val;

        let mut client = ClientConnection {
            stream,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
            magic_state: MagicValidationState::AwaitingMagic,
        };

        self.poll
            .registry()
            .register(&mut client.stream, token, Interest::READABLE)
            .map_err(|source| IpcServerError::PollFailed { source })?;

        drop(self.clients.insert(token.0, client));
        Ok(())
    }

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

        // Validate magic BEFORE appending when still awaiting magic.
        // This prevents unbounded buffer growth from a malicious peer
        // that sends garbage bytes instead of valid magic.
        if client.magic_state == MagicValidationState::AwaitingMagic
            && bytes_read >= AWAITING_MAGIC_MAX_BYTES
        {
            // SAFETY: bytes_read is return value of read() into fixed-size temp_buf,
            // guaranteed <= READ_CHUNK_BYTES. Slice indexing is therefore valid.
            #[allow(clippy::indexing_slicing)]
            let buf = &temp_buf[..bytes_read];
            match validate_magic_early(buf) {
                Ok(MagicValidationState::AwaitingMagic) => {
                    // Not enough bytes yet for full magic validation.
                    append_read_bytes(&mut client.read_buffer, &temp_buf, bytes_read)?;
                    return Ok(false);
                }
                Ok(MagicValidationState::MagicValidated) => {
                    // Magic validated — append bytes and continue.
                    client.magic_state = MagicValidationState::MagicValidated;
                    append_read_bytes(&mut client.read_buffer, &temp_buf, bytes_read)?;
                }
                Err(_error) => {
                    // Invalid magic — close connection immediately without buffering.
                    return Ok(true);
                }
            }
        } else {
            // Not yet enough bytes for magic check, or already validated.
            append_read_bytes(&mut client.read_buffer, &temp_buf, bytes_read)?;

            // Check if we now have enough bytes to validate magic.
            if client.magic_state == MagicValidationState::AwaitingMagic
                && client.read_buffer.len() >= AWAITING_MAGIC_MAX_BYTES
            {
                match validate_magic_early(&client.read_buffer) {
                    Ok(MagicValidationState::AwaitingMagic) => {
                        // Still waiting for more bytes.
                        return Ok(false);
                    }
                    Ok(MagicValidationState::MagicValidated) => {
                        client.magic_state = MagicValidationState::MagicValidated;
                    }
                    Err(_error) => {
                        // Invalid magic — close connection.
                        return Ok(true);
                    }
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

    pub(crate) fn remove_client(&mut self, token_index: usize) {
        if let Some(mut client) = self.clients.remove(&token_index) {
            drop(self.poll.registry().deregister(&mut client.stream));
        }
    }
}
