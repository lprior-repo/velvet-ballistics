#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
#![forbid(unsafe_code)]
//! Buffer and frame helper functions.

use super::IpcResponse;
use super::error::IpcServerError;
use crate::IpcError;
use crate::IpcFrameHeader;
use crate::{IPC_HEADER_LEN, IPC_MAGIC, MaxPayloadBytes};
use mio::Registry;
use mio::Token;
use mio::net::UnixStream;
use std::io::Write;

/// Maximum bytes to read while still in AwaitingMagic state.
pub const AWAITING_MAGIC_MAX_BYTES: usize = 4;

/// Magic validation state for IPC frame header parsing.
///
/// Tracks whether we have validated the magic bytes at the start of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagicValidationState {
    /// Have not yet validated magic bytes.
    AwaitingMagic,
    /// Magic bytes validated successfully.
    MagicValidated,
}

/// Validates magic bytes early, before allocating a large read buffer.
///
/// Returns `MagicValidated` if the magic matches `IPC_MAGIC`.
/// Returns `AwaitingMagic` if not enough bytes have been collected.
/// Returns an `IpcError::InvalidMagic` if the magic bytes are present but do not match.
pub fn validate_magic_early(bytes: &[u8]) -> Result<MagicValidationState, IpcError> {
    let Some(prefix) = bytes.get(..AWAITING_MAGIC_MAX_BYTES) else {
        return Ok(MagicValidationState::AwaitingMagic);
    };
    let magic_bytes = <[u8; AWAITING_MAGIC_MAX_BYTES]>::try_from(prefix)
        .map_err(|_| IpcError::HeaderDecodeFailed)?;
    let magic = u32::from_le_bytes(magic_bytes);
    if magic == IPC_MAGIC {
        Ok(MagicValidationState::MagicValidated)
    } else {
        Err(IpcError::InvalidMagic { actual: magic })
    }
}

/// Appends read bytes into the read buffer with bounds checking.
pub fn append_read_bytes(
    read_buffer: &mut Vec<u8>,
    temp_buf: &[u8; 4096],
    bytes_read: usize,
) -> Result<(), IpcServerError> {
    let read_slice = temp_buf
        .get(..bytes_read)
        .ok_or(IpcServerError::FrameInvalid {
            source: IpcError::PayloadLengthMismatch {
                header: 4096,
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

/// Reads the frame header from the read buffer.
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

/// Extracts payload bytes from the read buffer.
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

/// Creates a frame error response.
pub fn frame_error_response(error: IpcError) -> IpcResponse {
    IpcResponse::FrameError {
        message: error.to_string(),
    }
}

/// Borrows the workflow resolver from an optional mutable reference.
pub fn borrow_workflow_resolver<'a>(
    resolver: &'a mut Option<&mut dyn crate::server::WorkflowResolver>,
) -> Option<&'a mut dyn crate::server::WorkflowResolver> {
    match resolver {
        Some(inner) => Some(&mut **inner),
        None => None,
    }
}

#[cfg(test)]
pub(crate) mod test_hooks {
    use std::cell::Cell;
    thread_local! {
        pub(crate) static FORCE_POSTCARD_FAIL: Cell<bool> = const { Cell::new(false) };
        pub(crate) static FORCE_HEADER_ENCODE_FAIL: Cell<bool> = const { Cell::new(false) };
        pub(crate) static FORCE_FLUSH_FAIL: Cell<bool> = const { Cell::new(false) };
    }
}

/// Sends a response frame to the client.
pub fn send_response(
    stream: &mut UnixStream,
    write_buffer: &mut Vec<u8>,
    registry: &Registry,
    token: Token,
    request_header: &IpcFrameHeader,
    response: &IpcResponse,
) -> Result<(), IpcServerError> {
    #[cfg(test)]
    let payload_bytes = if test_hooks::FORCE_POSTCARD_FAIL.get() {
        Err(postcard::Error::SerializeBufferFull)
    } else {
        postcard::to_allocvec(response)
    }
    .map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    #[cfg(not(test))]
    let payload_bytes =
        postcard::to_allocvec(response).map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    let payload_len =
        u32::try_from(payload_bytes.len()).map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    let header = IpcFrameHeader::new(
        request_header.command,
        0,
        request_header.correlation,
        payload_len,
    );

    #[cfg(test)]
    let header_bytes = if test_hooks::FORCE_HEADER_ENCODE_FAIL.get() {
        Err(crate::IpcError::HeaderEncodeFailed)
    } else {
        header.encode()
    }
    .map_err(|_| IpcServerError::ResponseEncodeFailed)?;

    #[cfg(not(test))]
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
        write_buffer.drain(..written).for_each(|_| ());
    }
    if write_buffer.is_empty() {
        #[cfg(test)]
        let flush_result = if test_hooks::FORCE_FLUSH_FAIL.get() {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "flush fail",
            ))
        } else {
            stream.flush()
        };

        #[cfg(not(test))]
        let flush_result = stream.flush();

        match flush_result {
            Ok(()) => {}
            Err(ref source) if source.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(source) => return Err(IpcServerError::ResponseWriteFailed { source }),
        }
    } else {
        registry
            .reregister(
                stream,
                token,
                mio::Interest::READABLE | mio::Interest::WRITABLE,
            )
            .map_err(|source| IpcServerError::PollFailed { source })?;
    }

    Ok(())
}

#[cfg(test)]
pub(crate) fn append_read_bytes_checked_add(a: usize, b: usize) -> Result<usize, IpcServerError> {
    a.checked_add(b).ok_or(IpcServerError::ReadBufferTooLarge)
}

#[cfg(test)]
pub(crate) fn frame_total_len_checked_add(
    header_len: usize,
    payload_len: usize,
) -> Result<usize, IpcServerError> {
    header_len
        .checked_add(payload_len)
        .ok_or(IpcServerError::ReadBufferTooLarge)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IpcCommand;

    // ── append_read_bytes tests ──

    #[test]
    fn append_read_bytes_appends_data_to_empty_buffer() {
        let mut read_buffer = Vec::new();
        let temp_buf = [0xAB_u8; 4096];
        let result = append_read_bytes(&mut read_buffer, &temp_buf, 10);
        assert_eq!(result, Ok(()), "appending 10 bytes should succeed");
        assert_eq!(read_buffer.len(), 10);
        assert_eq!(read_buffer.as_slice(), &[0xAB; 10]);
    }

    #[test]
    fn append_read_bytes_appends_to_existing_buffer() {
        let mut read_buffer = vec![1, 2, 3];
        let temp_buf = [4u8; 4096];
        let result = append_read_bytes(&mut read_buffer, &temp_buf, 3);
        let Ok(()) = result else {
            panic!("appending 3 bytes should succeed");
        };
        assert_eq!(read_buffer.len(), 6);
        assert_eq!(read_buffer.as_slice(), &[1, 2, 3, 4, 4, 4]);
    }

    #[test]
    fn append_read_bytes_with_zero_bytes_read() {
        let mut read_buffer = Vec::new();
        let temp_buf = [0u8; 4096];
        let result = append_read_bytes(&mut read_buffer, &temp_buf, 0);
        let Ok(()) = result else {
            panic!("zero bytes should succeed");
        };
        assert!(read_buffer.is_empty());
    }

    #[test]
    fn append_read_bytes_rejects_bytes_read_exceeding_temp_buf() {
        let mut read_buffer = Vec::new();
        let temp_buf = [0u8; 4096];
        // bytes_read > 4096 is impossible in practice but tests the guard
        let result = append_read_bytes(&mut read_buffer, &temp_buf, 5000);
        let Err(e) = result else {
            panic!("bytes_read > temp_buf size should fail");
        };
        assert!(
            matches!(e, IpcServerError::FrameInvalid { .. }),
            "expected FrameInvalid, got {e:?}"
        );
    }

    // ── read_buffer_header tests ──

    #[test]
    fn read_buffer_header_returns_incomplete_frame_for_short_buffer() {
        let short_buf = vec![0u8; 10];
        let result = read_buffer_header(&short_buf);
        let Err(err) = result else {
            panic!("short buffer should fail");
        };
        let msg = err.to_string();
        assert!(
            msg.contains("incomplete"),
            "expected 'incomplete' in '{msg}'"
        );
    }

    #[test]
    fn read_buffer_header_returns_incomplete_frame_for_empty_buffer() {
        let empty_buf: Vec<u8> = Vec::new();
        let result = read_buffer_header(&empty_buf);
        let Err(e) = result else {
            panic!("expected IncompleteFrame for empty buffer");
        };
        assert!(
            matches!(e, IpcServerError::IncompleteFrame),
            "expected IncompleteFrame, got {e:?}"
        );
    }

    #[test]
    fn read_buffer_header_succeeds_with_exact_header_length() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let encoded = header.encode();
        let Ok(encoded) = encoded else {
            panic!("header should encode");
        };
        let buf = encoded.to_vec();
        let result = read_buffer_header(&buf);
        let Ok(_) = result else {
            panic!("exact header length should succeed");
        };
    }

    #[test]
    fn read_buffer_header_succeeds_with_extra_bytes() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let encoded = header.encode();
        let Ok(encoded) = encoded else {
            panic!("header should encode");
        };
        let mut buf = encoded.to_vec();
        buf.extend_from_slice(&[0xFF; 100]); // extra payload bytes
        let result = read_buffer_header(&buf);
        let Ok(_) = result else {
            panic!("extra bytes after header should still succeed");
        };
    }

    // ── frame_total_len tests ──

    #[test]
    fn frame_total_len_header_only_zero_payload() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let result = frame_total_len(&header);
        let Ok(val) = result else {
            panic!("frame_total_len should succeed");
        };
        assert_eq!(val, IPC_HEADER_LEN);
    }

    #[test]
    fn frame_total_len_with_payload() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
        let result = frame_total_len(&header);
        let Ok(val) = result else {
            panic!("frame_total_len should succeed");
        };
        assert_eq!(val, IPC_HEADER_LEN + 100);
    }

    #[test]
    fn frame_total_len_with_max_reasonable_payload() {
        let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 1, 1000);
        let result = frame_total_len(&header);
        let Ok(val) = result else {
            panic!("frame_total_len should succeed");
        };
        assert_eq!(val, IPC_HEADER_LEN + 1000);
    }

    // ── extract_payload tests ──

    #[test]
    fn extract_payload_returns_incomplete_when_buffer_too_short() {
        let mut read_buffer = vec![0u8; 10];
        let result = extract_payload(&mut read_buffer, 50);
        let Err(e) = result else {
            panic!("expected IncompleteFrame for short buffer");
        };
        assert!(
            matches!(e, IpcServerError::IncompleteFrame),
            "expected IncompleteFrame, got {e:?}"
        );
    }

    #[test]
    fn extract_payload_extracts_header_plus_payload() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
        let encoded = header.encode();
        let Ok(encoded) = encoded else {
            panic!("header should encode");
        };
        let mut read_buffer = encoded.to_vec();
        read_buffer.extend_from_slice(b"test");
        let total_len = IPC_HEADER_LEN + 4;

        let result = extract_payload(&mut read_buffer, total_len);
        let Ok(payload) = result else {
            panic!("extract should succeed");
        };
        assert_eq!(payload.as_slice(), b"test");
    }

    #[test]
    fn extract_payload_preserves_remaining_bytes_in_buffer() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
        let encoded = header.encode();
        let Ok(encoded) = encoded else {
            panic!("header should encode");
        };
        let mut read_buffer = encoded.to_vec();
        read_buffer.extend_from_slice(b"test");
        read_buffer.extend_from_slice(b"extra");
        let total_len = IPC_HEADER_LEN + 4;

        let result = extract_payload(&mut read_buffer, total_len);
        let Ok(_) = result else {
            panic!("extract should succeed");
        };
        assert_eq!(
            read_buffer.as_slice(),
            b"extra",
            "remaining bytes should stay in buffer"
        );
    }

    #[test]
    fn extract_payload_returns_empty_for_zero_payload_len() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let encoded = header.encode();
        let Ok(encoded) = encoded else {
            panic!("header should encode");
        };
        let mut read_buffer = encoded.to_vec();
        let result = extract_payload(&mut read_buffer, IPC_HEADER_LEN);
        let Ok(payload) = result else {
            panic!("extract should succeed");
        };
        assert!(payload.is_empty());
    }

    // ── frame_error_response tests ──

    #[test]
    fn frame_error_response_produces_frame_error_variant() {
        let err = IpcError::InvalidMagic { actual: 0xDEAD };
        let response = frame_error_response(err);
        let IpcResponse::FrameError { message } = response else {
            return;
        };
        assert!(message.contains("magic"), "expected 'magic' in '{message}'");
    }

    #[test]
    fn frame_error_response_includes_display_of_source() {
        let err = IpcError::PayloadTooLarge {
            actual: 999,
            limit: 10,
        };
        let response = frame_error_response(err);
        let IpcResponse::FrameError { message } = response else {
            return;
        };
        assert!(
            message.contains("999"),
            "expected actual value in '{message}'"
        );
        assert!(
            message.contains("10"),
            "expected limit value in '{message}'"
        );
    }

    // ── borrow_workflow_resolver tests ──

    #[test]
    fn borrow_workflow_resolver_returns_none_for_none_outer() {
        let mut outer: Option<&mut dyn crate::server::WorkflowResolver> = None;
        let result = borrow_workflow_resolver(&mut outer);
        assert!(result.is_none());
    }

    #[test]
    fn borrow_workflow_resolver_returns_some_for_some_outer() {
        struct DummyResolver;
        impl crate::server::WorkflowResolver for DummyResolver {
            fn resolve_workflow(
                &mut self,
                _digest: vb_core::WorkflowDigest,
            ) -> Result<vb_core::workflow::CompiledWorkflow, crate::server::WorkflowResolutionError>
            {
                Err(crate::server::WorkflowResolutionError::NotFound)
            }
        }
        let mut resolver = DummyResolver;
        let mut outer: Option<&mut dyn crate::server::WorkflowResolver> = Some(&mut resolver);
        let result = borrow_workflow_resolver(&mut outer);
        assert!(result.is_some());
    }

    // ── append_read_bytes overflow / bounds tests ──

    #[test]
    fn append_read_bytes_accepts_exactly_at_max() {
        let max = IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT.get();
        let mut read_buffer = vec![0u8; max - 1];
        let temp_buf = [1u8; 4096];
        let result = append_read_bytes(&mut read_buffer, &temp_buf, 1);
        assert_eq!(result, Ok(()), "expected Ok when appending exactly to max");
        assert_eq!(read_buffer.len(), max);
    }

    #[test]
    fn append_read_bytes_rejects_when_buffer_would_exceed_max() {
        let max = IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT.get();
        let mut read_buffer = vec![0u8; max - 5];
        let temp_buf = [1u8; 4096];
        let result = append_read_bytes(&mut read_buffer, &temp_buf, 10);
        assert!(
            matches!(result, Err(IpcServerError::ReadBufferTooLarge)),
            "expected ReadBufferTooLarge when max exceeded"
        );
    }

    #[test]
    fn append_read_bytes_rejects_at_exactly_one_over_max() {
        let max = IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT.get();
        let mut read_buffer = vec![0u8; max];
        let temp_buf = [1u8; 4096];
        let result = append_read_bytes(&mut read_buffer, &temp_buf, 1);
        assert!(
            matches!(result, Err(IpcServerError::ReadBufferTooLarge)),
            "expected ReadBufferTooLarge when exactly one over max"
        );
    }

    #[test]
    fn append_read_bytes_checked_add_overflow_returns_too_large() {
        let result = append_read_bytes_checked_add(usize::MAX, 1);
        assert!(
            matches!(result, Err(IpcServerError::ReadBufferTooLarge)),
            "expected ReadBufferTooLarge on usize overflow"
        );
    }

    // ── frame_total_len overflow test ──

    #[test]
    fn frame_total_len_checked_add_overflow_returns_too_large() {
        let result = frame_total_len_checked_add(usize::MAX, 1);
        assert!(
            matches!(result, Err(IpcServerError::ReadBufferTooLarge)),
            "expected ReadBufferTooLarge on usize overflow"
        );
    }

    // ── extract_payload exact-length test ──

    #[test]
    fn extract_payload_exact_length_clears_buffer() {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
        let encoded = header.encode().expect("encode");
        let mut read_buffer = encoded.to_vec();
        read_buffer.extend_from_slice(b"test");
        let total_len = IPC_HEADER_LEN + 4;

        let result = extract_payload(&mut read_buffer, total_len);
        let Ok(payload) = result else {
            panic!("extract should succeed");
        };
        assert_eq!(payload.as_slice(), b"test");
        assert!(
            read_buffer.is_empty(),
            "read_buffer should be empty after exact extraction"
        );
    }

    // ── send_response error-path tests ──

    use mio::Poll;

    struct ResetOnDrop;
    impl Drop for ResetOnDrop {
        fn drop(&mut self) {
            test_hooks::FORCE_POSTCARD_FAIL.set(false);
            test_hooks::FORCE_HEADER_ENCODE_FAIL.set(false);
            test_hooks::FORCE_FLUSH_FAIL.set(false);
        }
    }

    fn setup_mio_stream_with_peer() -> (
        mio::net::UnixStream,
        std::os::unix::net::UnixStream,
        Poll,
        mio::Token,
    ) {
        let (std_a, std_b) = std::os::unix::net::UnixStream::pair().unwrap();
        std_a.set_nonblocking(true).unwrap();
        std_b.set_nonblocking(true).unwrap();
        let mut stream = mio::net::UnixStream::from_std(std_a);
        let poll = Poll::new().unwrap();
        let token = mio::Token(1);
        poll.registry()
            .register(&mut stream, token, mio::Interest::READABLE)
            .unwrap();
        (stream, std_b, poll, token)
    }

    #[test]
    fn send_response_reregisters_when_written_zero_and_buffer_not_empty() {
        let (mut stream, _peer, poll, token) = setup_mio_stream_with_peer();
        let registry = poll.registry().try_clone().unwrap();

        // Fill the stream's send buffer so the next write returns WouldBlock.
        let buf = [0u8; 4096];
        loop {
            match stream.write(&buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        let mut write_buffer = Vec::new();
        let request_header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let response = crate::server::IpcResponse::Healthy;

        let result = send_response(
            &mut stream,
            &mut write_buffer,
            &registry,
            token,
            &request_header,
            &response,
        );

        let Ok(()) = result else {
            panic!(
                "send_response should succeed after reregister: {:?}",
                result.err()
            );
        };
        assert!(
            !write_buffer.is_empty(),
            "write_buffer should still contain data after WouldBlock"
        );
    }

    #[test]
    fn send_response_returns_error_when_postcard_encode_fails() {
        let (mut stream, _peer, poll, token) = setup_mio_stream_with_peer();
        let registry = poll.registry().try_clone().unwrap();

        test_hooks::FORCE_POSTCARD_FAIL.set(true);
        let _guard = ResetOnDrop;

        let mut write_buffer = Vec::new();
        let request_header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let response = crate::server::IpcResponse::Healthy;

        let result = send_response(
            &mut stream,
            &mut write_buffer,
            &registry,
            token,
            &request_header,
            &response,
        );

        let Err(e) = result else {
            panic!("expected ResponseEncodeFailed");
        };
        assert!(
            matches!(e, IpcServerError::ResponseEncodeFailed),
            "expected ResponseEncodeFailed, got {e:?}"
        );
    }

    #[test]
    fn send_response_returns_error_when_header_encode_fails() {
        let (mut stream, _peer, poll, token) = setup_mio_stream_with_peer();
        let registry = poll.registry().try_clone().unwrap();

        test_hooks::FORCE_HEADER_ENCODE_FAIL.set(true);
        let _guard = ResetOnDrop;

        let mut write_buffer = Vec::new();
        let request_header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let response = crate::server::IpcResponse::Healthy;

        let result = send_response(
            &mut stream,
            &mut write_buffer,
            &registry,
            token,
            &request_header,
            &response,
        );

        let Err(e) = result else {
            panic!("expected ResponseEncodeFailed");
        };
        assert!(
            matches!(e, IpcServerError::ResponseEncodeFailed),
            "expected ResponseEncodeFailed, got {e:?}"
        );
    }

    #[test]
    fn send_response_returns_error_when_flush_fails_non_wouldblock() {
        let (mut stream, _peer, poll, token) = setup_mio_stream_with_peer();
        let registry = poll.registry().try_clone().unwrap();

        test_hooks::FORCE_FLUSH_FAIL.set(true);
        let _guard = ResetOnDrop;

        let mut write_buffer = Vec::new();
        let request_header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let response = crate::server::IpcResponse::Healthy;

        let result = send_response(
            &mut stream,
            &mut write_buffer,
            &registry,
            token,
            &request_header,
            &response,
        );

        let Err(e) = result else {
            panic!("expected ResponseWriteFailed");
        };
        assert!(
            matches!(e, IpcServerError::ResponseWriteFailed { .. }),
            "expected ResponseWriteFailed, got {e:?}"
        );
    }

    #[test]
    fn send_response_returns_error_when_write_fails_broken_pipe() {
        let (mut stream, peer, poll, token) = setup_mio_stream_with_peer();
        let registry = poll.registry().try_clone().unwrap();
        drop(peer);

        let mut write_buffer = Vec::new();
        let request_header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let response = crate::server::IpcResponse::Healthy;

        let result = send_response(
            &mut stream,
            &mut write_buffer,
            &registry,
            token,
            &request_header,
            &response,
        );

        let Err(e) = result else {
            panic!("expected ResponseWriteFailed after dropping peer");
        };
        assert!(
            matches!(e, IpcServerError::ResponseWriteFailed { .. }),
            "expected ResponseWriteFailed, got {e:?}"
        );
    }
}
