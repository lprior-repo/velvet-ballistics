#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::map_clone,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
#![forbid(unsafe_code)]
//! IpcServer implementation.

#![allow(unused_imports)]

use arrayvec::ArrayVec;
use mio::net::UnixListener;
use mio::{Events, Interest, Poll, Token};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
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
    ClientConnection, IpcResponse, IpcServer, MAX_CLIENTS, WorkflowResolutionError,
    WorkflowResolver, append_read_bytes, borrow_workflow_resolver, extract_payload,
    frame_error_response, frame_total_len, read_buffer_header, send_response,
};

const SERVER_TOKEN: Token = Token(0);
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

        // SEC-03 (master §23 + RED-QUEEN-MASTER-ISSUE-REPORT.md): restrict
        // the bound Unix socket to owner-only read/write. Without this,
        // any local user can connect to the socket and submit commands
        // against the running shard.
        #[cfg(unix)]
        {
            let permissions = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(socket_path, permissions)
                .map_err(|source| IpcServerError::BindFailed { source })?;
        }

        let poll = Poll::new().map_err(|source| IpcServerError::PollFailed { source })?;

        poll.registry()
            .register(&mut listener, SERVER_TOKEN, Interest::READABLE)
            .map_err(|source| IpcServerError::PollFailed { source })?;

        let events = Events::with_capacity(MAX_CLIENTS);

        const NONE_CONN: Option<ClientConnection> = None;

        Ok(Self {
            poll,
            listener,
            events,
            clients: [NONE_CONN; MAX_CLIENTS],
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
        let (stream, _addr) = self
            .listener
            .accept()
            .map_err(|source| IpcServerError::AcceptFailed { source })?;

        let free_slot = self.clients.iter().position(|c| c.is_none());

        if let Some(index) = free_slot {
            let token = Token(index.saturating_add(1));

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

            if let Some(c) = self.clients.get_mut(index) {
                *c = Some(client);
            }
        } else {
            // Drop connection to enforce concurrent client limit.
            drop(stream);
        }

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

        let Some(index) = token_index.checked_sub(1) else {
            return Ok(true);
        };
        let Some(client) = self.clients.get_mut(index).and_then(|c| c.as_mut()) else {
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
            let read_slice = temp_buf
                .get(..bytes_read)
                .ok_or(IpcServerError::FrameInvalid {
                    source: IpcError::PayloadLengthMismatch {
                        header: READ_CHUNK_BYTES,
                        actual: bytes_read,
                    },
                })?;
            match validate_magic_early(read_slice) {
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
        let Some(index) = token_index.checked_sub(1) else {
            return Ok(true);
        };
        let Some(client) = self.clients.get_mut(index).and_then(|c| c.as_mut()) else {
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
        if let Some(index) = token_index.checked_sub(1) {
            if let Some(mut client) = self.clients.get_mut(index).and_then(|c| c.take()) {
                drop(self.poll.registry().deregister(&mut client.stream));
            }
        }
    }
}
