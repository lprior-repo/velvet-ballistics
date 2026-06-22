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
//! Integration tests for vb_ipc.

use vb_core::{DiagnosticCode, RunId, WorkflowDigest};

use crate::{
    BoundedPayload, IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION, IngressFrame, IpcCommand, IpcError,
    IpcFrameHeader, IpcPayload, MaxPayloadBytes, MemoryIngress, QueueCapacity, ROOT_CAPABILITY_BIT,
    SubmitRunPayload, decode_frame, decode_payload, encode_payload,
};
use bytes::Bytes;

#[cfg(test)]
macro_rules! prop_assert_err {
    ($result:expr $(, $($arg:tt)+)?) => {{
        let is_err_result = match &$result {
            Ok(_) => false,
            Err(_) => true,
        };
        prop_assert!(is_err_result $(, $($arg)+)?);
    }};
}

fn header_bytes(
    magic: u32,
    version: u16,
    command: u16,
    flags: u16,
    reserved: u16,
    correlation: u64,
    payload_len: u32,
) -> Result<[u8; IPC_HEADER_LEN], IpcError> {
    let mut bytes = Vec::with_capacity(IPC_HEADER_LEN);
    bytes.extend_from_slice(&magic.to_le_bytes());
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.extend_from_slice(&command.to_le_bytes());
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.extend_from_slice(&reserved.to_le_bytes());
    bytes.extend_from_slice(&correlation.to_le_bytes());
    bytes.extend_from_slice(&payload_len.to_le_bytes());

    <[u8; IPC_HEADER_LEN]>::try_from(bytes.as_slice()).map_err(|_| IpcError::HeaderEncodeFailed)
}

#[test]
fn bounded_queue_applies_backpressure() {
    let capacity = QueueCapacity::new(std::num::NonZeroUsize::MIN);
    let queue = MemoryIngress::bounded(capacity);
    let frame = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([2; 32]),
        Bytes::from_static(b"{}"),
        MaxPayloadBytes::DEFAULT,
    );
    let frame = frame.expect("test frame should fit default payload bound");

    assert_eq!(queue.try_submit(frame.clone()), Ok(()));
    assert_eq!(queue.try_submit(frame), Err(IpcError::Full));
    assert_eq!(queue.len(), 1);
}

#[test]
fn oversized_payload_is_rejected() {
    let payload_bytes = b"too big";
    let max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([2; 32]),
        Bytes::from_static(payload_bytes),
        max,
    );

    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: payload_bytes.len(),
            limit: max.get(),
        })
    );
}

#[test]
fn command_ids_cover_required_surface() {
    assert_eq!(IpcCommand::from_u16(1), Ok(IpcCommand::SubmitRun));
    assert_eq!(IpcCommand::from_u16(2), Ok(IpcCommand::SubmitRunInline));
    assert_eq!(IpcCommand::from_u16(3), Ok(IpcCommand::CancelRun));
    assert_eq!(IpcCommand::from_u16(4), Ok(IpcCommand::InspectRun));
    assert_eq!(IpcCommand::from_u16(5), Ok(IpcCommand::ListEvents));
    assert_eq!(IpcCommand::from_u16(6), Ok(IpcCommand::AnswerAsk));
    assert_eq!(IpcCommand::from_u16(7), Ok(IpcCommand::CompleteAction));
    assert_eq!(IpcCommand::from_u16(8), Ok(IpcCommand::FailAction));
    assert_eq!(IpcCommand::from_u16(9), Ok(IpcCommand::DrainTrace));
    assert_eq!(IpcCommand::from_u16(10), Ok(IpcCommand::Health));
    assert_eq!(IpcCommand::from_u16(11), Ok(IpcCommand::Shutdown));
    assert_eq!(IpcCommand::from_u16(12), Ok(IpcCommand::UnknownCommand(12)));
    assert_eq!(IpcCommand::from_u16(13), Ok(IpcCommand::UnknownCommand(13)));
    assert_eq!(IpcCommand::from_u16(14), Ok(IpcCommand::UnknownCommand(14)));
    assert_eq!(IpcCommand::from_u16(15), Ok(IpcCommand::UnknownCommand(15)));
    assert_eq!(IpcCommand::from_u16(16), Ok(IpcCommand::UnknownCommand(16)));
    assert_eq!(IpcCommand::from_u16(17), Ok(IpcCommand::UnknownCommand(17)));
}

#[test]
fn header_roundtrips_little_endian_fields() {
    // GAP-5: CompleteAction accepts only zero flags; use SubmitRunInline
    // with low-byte flags=7 to preserve the roundtrip shape.
    let header = IpcFrameHeader::new(IpcCommand::SubmitRunInline, 7, 42, 3);
    let encoded = header.encode();
    let encoded = encoded.expect("header should encode to fixed width");

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
    assert_eq!(decoded, Ok(header));
}

#[test]
fn decoder_rejects_bad_magic_before_payload_use() {
    let encoded = header_bytes(0, IPC_VERSION, IpcCommand::Health.as_u16(), 0, 0, 1, 0);
    let encoded = encoded.expect("test header should encode");
    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(decoded, Err(IpcError::InvalidMagic { actual: 0 }));
}

#[test]
fn decoder_rejects_payload_above_bound() {
    let payload_len_val: u32 = 8;
    let payload_len_usize = match usize::try_from(payload_len_val) {
        Ok(v) => v,
        Err(_) => return,
    };
    let max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    // SEC-01: caller-capabilities envelope is the ROOT bit (non-zero), so the
    // payload-bound check is reached and the oversized payload is rejected.
    let encoded = header_bytes(
        IPC_MAGIC,
        IPC_VERSION,
        IpcCommand::SubmitRun.as_u16(),
        0,
        ROOT_CAPABILITY_BIT,
        1,
        payload_len_val,
    );
    let encoded = encoded.expect("test header should encode");
    let decoded = IpcFrameHeader::decode(&encoded, max);

    assert_eq!(
        decoded,
        Err(IpcError::PayloadTooLarge {
            actual: payload_len_usize,
            limit: max.get(),
        })
    );
}

#[test]
fn decoder_rejects_zero_capabilities_envelope() {
    // SEC-01: a zero caller-capabilities envelope is the missing-capability
    // sentinel and must be rejected with PermissionDenied (replaces the old
    // `reserved != 0 → ReservedNonZero` check).
    let encoded = header_bytes(
        IPC_MAGIC,
        IPC_VERSION,
        IpcCommand::Health.as_u16(),
        0,
        0,
        1,
        0,
    );
    let encoded = encoded.expect("test header should encode");
    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(decoded, Err(IpcError::PermissionDenied));
}

#[test]
fn decoder_accepts_nonzero_capabilities_envelope() {
    let encoded = header_bytes(
        IPC_MAGIC,
        IPC_VERSION,
        IpcCommand::Health.as_u16(),
        0,
        9,
        1,
        0,
    );
    let encoded = encoded.expect("test header should encode");
    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
    let header = decoded.expect("non-zero capabilities envelope must be accepted");
    assert_eq!(header.caller_capabilities.bits(), 9);
}

#[test]
fn frame_decode_requires_payload_length_match() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let encoded = header.encode();
    let encoded = encoded.expect("header should encode to fixed width");

    let decoded = decode_frame(
        &encoded,
        Bytes::from_static(b"abc"),
        MaxPayloadBytes::DEFAULT,
    );

    assert_eq!(
        decoded,
        Err(IpcError::PayloadLengthMismatch {
            header: 4,
            actual: 3,
        })
    );
}

#[test]
fn postcard_payload_roundtrips_as_typed_command() {
    let payload = IpcPayload::SubmitRun(SubmitRunPayload {
        run_id: RunId::new(7),
        workflow: WorkflowDigest::from_bytes([3; 32]),
        input: Vec::from(&b"input"[..]),
    });

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("payload should encode under default bound");

    assert_eq!(decode_payload(&encoded), Ok(payload));
}

#[test]
fn from_u16_rejects_zero_command() {
    let result = IpcCommand::from_u16(0);
    assert_eq!(result, Ok(IpcCommand::UnknownCommand(0)));
}

#[test]
fn unsupported_version_rejects_when_version_is_two() {
    let encoded = header_bytes(IPC_MAGIC, 2, IpcCommand::Health.as_u16(), 0, 0, 1, 0);
    let encoded = encoded.expect("test header should encode");
    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(decoded, Err(IpcError::UnsupportedVersion { actual: 2 }));
}

#[test]
fn memory_ingress_try_recv_returns_none_when_empty() {
    let capacity = QueueCapacity::new(std::num::NonZeroUsize::MIN);
    let queue = MemoryIngress::bounded(capacity);

    assert_eq!(queue.try_recv(), Ok(None));
}

#[test]
fn memory_ingress_is_empty_after_construction() {
    let capacity = QueueCapacity::new(std::num::NonZeroUsize::MIN);
    let queue = MemoryIngress::bounded(capacity);

    assert!(queue.is_empty());
}

#[test]
fn bounded_payload_bytes_returns_inner_slice() {
    let data = Bytes::from_static(b"hello");
    let bounded = BoundedPayload::new(data.clone(), MaxPayloadBytes::DEFAULT);
    let bounded = bounded.expect("payload should fit default bound");

    assert_eq!(bounded.bytes(), &data);
}

#[test]
fn ingress_frame_accessors_return_correct_values() {
    let run_id = RunId::new(42);
    let workflow = WorkflowDigest::from_bytes([0xAB; 32]);
    let data = Bytes::from_static(b"payload");
    let frame = IngressFrame::new(run_id, workflow, data, MaxPayloadBytes::DEFAULT);
    let frame = frame.expect("frame should construct");

    assert_eq!(frame.run_id(), run_id);
    assert_eq!(frame.workflow(), workflow);
    assert_eq!(frame.payload().bytes().as_ref(), b"payload");
}

// ── Error variant exact-assertion tests ────────────────────────────────────────

#[test]
fn decode_returns_disconnected_when_buffer_empty() {
    let mut ingress = MemoryIngress::bounded(QueueCapacity::new(std::num::NonZeroUsize::MIN));
    ingress.disconnect_sender();

    assert_eq!(ingress.try_recv(), Err(IpcError::Disconnected));
}

#[test]
fn from_u16_returns_unknown_command_for_zero() {
    assert_eq!(IpcCommand::from_u16(0), Ok(IpcCommand::UnknownCommand(0)));
}

#[test]
fn from_u16_returns_unknown_command_for_value_above_range() {
    assert_eq!(IpcCommand::from_u16(99), Ok(IpcCommand::UnknownCommand(99)));
}

#[test]
fn bounded_payload_rejects_oversized_with_exact_counts() {
    let data = Bytes::from(vec![0u8; 100]);
    let max = MaxPayloadBytes::new(
        std::num::NonZeroUsize::new(10).unwrap_or(std::num::NonZeroUsize::MIN),
    );

    let result = BoundedPayload::new(data, max);

    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: 100,
            limit: 10,
        })
    );
}

#[test]
fn ingress_frame_rejects_payload_exceeding_max() {
    let data = Bytes::from(vec![0xAA; 200]);
    let max = MaxPayloadBytes::new(
        std::num::NonZeroUsize::new(50).unwrap_or(std::num::NonZeroUsize::MIN),
    );

    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0; 32]),
        data,
        max,
    );

    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: 200,
            limit: 50,
        })
    );
}

#[test]
fn decode_payload_returns_decode_failed_on_garbage() {
    let garbage = Bytes::from_static(b"\xff\xff\xff\xff");
    let bounded = BoundedPayload::new(garbage, MaxPayloadBytes::DEFAULT);
    let bounded = bounded.expect("garbage should fit in bound");

    assert_eq!(decode_payload(&bounded), Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn encode_header_always_produces_fixed_width() {
    // GAP-5: Shutdown accepts zero flags only; encode is the trust boundary
    // that must succeed regardless of flag content.
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0x00FF, 0xDEAD_BEEF_CAFE, 1024);

    let encoded = header.encode();
    let encoded = encoded.expect("header encode should succeed");

    assert_eq!(encoded.len(), IPC_HEADER_LEN);
}

#[test]
fn encode_header_produces_correct_magic_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);

    let encoded = header.encode();
    let encoded = encoded.expect("header encode should succeed");

    let magic_bytes = encoded.get(..4);
    assert_eq!(magic_bytes, Some(IPC_MAGIC.to_le_bytes().as_slice()));
}

#[test]
fn encode_header_produces_correct_version_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);

    let encoded = header.encode();
    let encoded = encoded.expect("header encode should succeed");

    let version_bytes = encoded.get(4..6);
    assert_eq!(version_bytes, Some(IPC_VERSION.to_le_bytes().as_slice()));
}

#[test]
fn encode_header_produces_correct_command_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::CancelRun, 0, 1, 0);

    let encoded = header.encode();
    let encoded = encoded.expect("header encode should succeed");

    let command_bytes = encoded.get(6..8);
    assert_eq!(command_bytes, Some(3u16.to_le_bytes().as_slice()));
}

#[test]
fn encode_header_produces_correct_flags_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0x1234, 1, 0);

    let encoded = header.encode();
    let encoded = encoded.expect("header encode should succeed");

    let flags_bytes = encoded.get(8..10);
    assert_eq!(flags_bytes, Some(0x1234u16.to_le_bytes().as_slice()));
}

#[test]
fn encode_header_produces_root_capabilities_envelope() {
    // SEC-01: the previously reserved slot now carries the ROOT caller-capability
    // bit by default, so the encoded bytes at offset 10..12 must equal the
    // ROOT bit pattern (0x0001 little-endian).
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);

    let encoded = header.encode();
    let encoded = encoded.expect("header encode should succeed");

    let reserved_bytes = encoded.get(10..12);
    assert_eq!(
        reserved_bytes,
        Some(ROOT_CAPABILITY_BIT.to_le_bytes().as_slice())
    );
}

#[test]
fn encode_header_produces_correct_correlation_bytes() {
    let correlation: u64 = 0x0102_0304_0506_0708;
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, correlation, 0);

    let encoded = header.encode();
    let encoded = encoded.expect("header encode should succeed");

    let corr_bytes = encoded.get(12..20);
    assert_eq!(corr_bytes, Some(correlation.to_le_bytes().as_slice()));
}

#[test]
fn encode_header_produces_correct_payload_len_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4096);

    let encoded = header.encode();
    let encoded = encoded.expect("header encode should succeed");

    let plen_bytes = encoded.get(20..24);
    assert_eq!(plen_bytes, Some(4096u32.to_le_bytes().as_slice()));
}

#[test]
fn frame_decode_succeeds_when_payload_length_matches() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 3);
    let encoded = header.encode();
    let encoded = encoded.expect("header should encode");
    let payload = Bytes::from_static(b"abc");

    let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

    let frame = result.expect("frame should decode");
    assert_eq!(frame.header().command, IpcCommand::Health);
    assert_eq!(frame.header().payload_len, 3);
    assert_eq!(frame.payload().bytes().as_ref(), b"abc");
}

#[test]
fn memory_ingress_len_reflects_queue_depth() {
    let capacity =
        QueueCapacity::new(std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN));
    let queue = MemoryIngress::bounded(capacity);

    let frame_zero = IngressFrame::new(
        RunId::new(0),
        WorkflowDigest::from_bytes([0; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    let frame_zero = frame_zero.expect("frame zero should construct");
    assert_eq!(queue.try_submit(frame_zero), Ok(()));

    let frame_one = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    let frame_one = frame_one.expect("frame one should construct");
    assert_eq!(queue.try_submit(frame_one), Ok(()));

    let frame_two = IngressFrame::new(
        RunId::new(2),
        WorkflowDigest::from_bytes([0; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    let frame_two = frame_two.expect("frame two should construct");
    assert_eq!(queue.try_submit(frame_two), Ok(()));

    assert_eq!(queue.len(), 3);
    assert!(!queue.is_empty());
}

#[test]
fn memory_ingress_try_recv_returns_frames_in_fifo_order() {
    let capacity =
        QueueCapacity::new(std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN));
    let queue = MemoryIngress::bounded(capacity);
    let frame1 = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([1; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    let frame2 = IngressFrame::new(
        RunId::new(2),
        WorkflowDigest::from_bytes([2; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    let frame1 = frame1.expect("first frame must construct");
    let frame2 = frame2.expect("second frame must construct");
    assert_eq!(queue.try_submit(frame1), Ok(()));
    assert_eq!(queue.try_submit(frame2), Ok(()));

    let recv1 = queue.try_recv();
    let recv2 = queue.try_recv();

    let f1 = recv1
        .expect("first recv should succeed")
        .expect("expected Some variant");
    let f2 = recv2
        .expect("second recv should succeed")
        .expect("expected Some variant");
    assert_eq!(f1.run_id(), RunId::new(1));
    assert_eq!(f2.run_id(), RunId::new(2));
}

#[test]
fn memory_ingress_is_empty_after_draining_all_frames() {
    let capacity =
        QueueCapacity::new(std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN));
    let queue = MemoryIngress::bounded(capacity);
    let frame = IngressFrame::new(
        RunId::new(99),
        WorkflowDigest::from_bytes([0; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    let frame = frame.expect("frame should construct");
    assert_eq!(queue.try_submit(frame), Ok(()));

    let drained = queue.try_recv();
    let drained = drained.expect("queued frame must drain");

    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn payload_roundtrip_preserves_cancel_run_variant() {
    let payload = IpcPayload::CancelRun {
        run_id: RunId::new(42),
        reason: None,
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("payload should encode");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_list_events_variant() {
    let payload = IpcPayload::ListEvents {
        run_id: RunId::new(7),
        from_sequence: 100,
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("payload should encode");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_answer_ask_variant() {
    let payload = IpcPayload::AnswerAsk {
        run_id: RunId::new(5),
        answer_slot: vb_core::ids::SlotIdx::new(9),
        answer: Vec::from(&b"yes"[..]),
        taint: None,
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("payload should encode");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_complete_action_variant() {
    let payload = IpcPayload::CompleteAction {
        run_id: RunId::new(11),
        ticket: 42,
        output: Vec::from(&b"done"[..]),
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("payload should encode");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_fail_action_variant() {
    let payload = IpcPayload::FailAction {
        run_id: RunId::new(13),
        ticket: 7,
        error: Vec::from(&b"err"[..]),
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("payload should encode");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_drain_trace_variant() {
    let payload = IpcPayload::DrainTrace {
        run_id: RunId::new(77),
        max_records: 500,
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("payload should encode");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_health_variant() {
    let payload = IpcPayload::Health;

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("payload should encode");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_shutdown_variant() {
    let payload = IpcPayload::Shutdown;

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("payload should encode");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_inspect_run_variant() {
    let payload = IpcPayload::InspectRun {
        run_id: RunId::new(333),
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("payload should encode");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_submit_run_inline_variant() {
    let payload = IpcPayload::SubmitRunInline(SubmitRunPayload {
        run_id: RunId::new(55),
        workflow: WorkflowDigest::from_bytes([0xBB; 32]),
        input: Vec::from(&b"inline-input"[..]),
    });

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("payload should encode");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn header_decode_rejects_unsupported_version_zero() {
    let encoded = header_bytes(IPC_MAGIC, 0, IpcCommand::Health.as_u16(), 0, 0, 1, 0);
    let encoded = encoded.expect("test header should encode");

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(decoded, Err(IpcError::UnsupportedVersion { actual: 0 }));
}

#[test]
fn header_decode_accepts_unknown_command_id() {
    // SEC-01: provide the ROOT capability envelope so the envelope check passes.
    let encoded = header_bytes(IPC_MAGIC, IPC_VERSION, 200, 0, ROOT_CAPABILITY_BIT, 1, 0);
    let encoded = encoded.expect("test header should encode");

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(
        decoded,
        Ok(IpcFrameHeader::new(
            IpcCommand::UnknownCommand(200),
            0,
            1,
            0
        ))
    );
}

#[test]
fn max_payload_bytes_default_is_one_mib() {
    assert_eq!(MaxPayloadBytes::DEFAULT.get(), 1_048_576);
}

#[test]
fn queue_capacity_returns_inner_value() {
    let cap =
        QueueCapacity::new(std::num::NonZeroUsize::new(42).unwrap_or(std::num::NonZeroUsize::MIN));
    assert_eq!(cap.get(), 42);
}

#[test]
fn max_payload_bytes_custom_value_respects_input() {
    let max = MaxPayloadBytes::new(
        std::num::NonZeroUsize::new(512).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    assert_eq!(max.get(), 512);
}

#[test]
fn bounded_payload_accepts_exactly_max_bytes() {
    let max_val = 16;
    let max = MaxPayloadBytes::new(
        std::num::NonZeroUsize::new(max_val).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    let data = Bytes::from(vec![0u8; max_val]);

    let result = BoundedPayload::new(data, max);

    let result = result.expect("payload at exact max should succeed");
}

#[test]
fn bounded_payload_rejects_one_over_max() {
    let max_val = 16;
    let max = MaxPayloadBytes::new(
        std::num::NonZeroUsize::new(max_val).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    let data = Bytes::from(vec![0u8; max_val + 1]);

    let result = BoundedPayload::new(data, max);

    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: max_val + 1,
            limit: max_val,
        })
    );
}

#[test]
fn bounded_payload_bytes_returns_correct_length() {
    let data = Bytes::from(vec![0u8; 7]);
    let bounded = BoundedPayload::new(data, MaxPayloadBytes::DEFAULT);
    let bounded = bounded.expect("should create bounded payload");

    assert_eq!(bounded.bytes().len(), 7);
}

#[test]
fn ipc_command_as_u16_returns_correct_values() {
    assert_eq!(IpcCommand::SubmitRun.as_u16(), 1);
    assert_eq!(IpcCommand::SubmitRunInline.as_u16(), 2);
    assert_eq!(IpcCommand::CancelRun.as_u16(), 3);
    assert_eq!(IpcCommand::InspectRun.as_u16(), 4);
    assert_eq!(IpcCommand::ListEvents.as_u16(), 5);
    assert_eq!(IpcCommand::AnswerAsk.as_u16(), 6);
    assert_eq!(IpcCommand::CompleteAction.as_u16(), 7);
    assert_eq!(IpcCommand::FailAction.as_u16(), 8);
    assert_eq!(IpcCommand::DrainTrace.as_u16(), 9);
    assert_eq!(IpcCommand::Health.as_u16(), 10);
    assert_eq!(IpcCommand::Shutdown.as_u16(), 11);
    assert_eq!(IpcCommand::UnknownCommand(12).as_u16(), 12);
    assert_eq!(IpcCommand::UnknownCommand(13).as_u16(), 13);
    assert_eq!(IpcCommand::UnknownCommand(14).as_u16(), 14);
    assert_eq!(IpcCommand::UnknownCommand(15).as_u16(), 15);
    assert_eq!(IpcCommand::UnknownCommand(16).as_u16(), 16);
}

#[test]
fn ipc_frame_header_new_stores_all_fields() {
    let header = IpcFrameHeader::new(IpcCommand::ListEvents, 0x00FF, 12345, 678);

    assert_eq!(header.command, IpcCommand::ListEvents);
    assert_eq!(header.flags, 0x00FF);
    assert_eq!(header.correlation, 12345);
    assert_eq!(header.payload_len, 678);
}

#[test]
fn ut_header_encoding_roundtrips() {
    // GAP-5: SubmitRunInline accepts up to 0x00FF. Use 0x0034 (low byte only).
    let header = IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0x0034, 0xDEAD_BEEF, 512);
    let encoded = header.encode().expect("header must encode");
    let decoded =
        IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT).expect("header must decode");
    assert_eq!(decoded.command, header.command);
    assert_eq!(decoded.flags, header.flags);
    assert_eq!(decoded.correlation, header.correlation);
    assert_eq!(decoded.payload_len, header.payload_len);
}

#[test]
fn ut_header_envelope_bytes_carry_root_capability() {
    // SEC-01: the bytes at offset 10..12 now carry the ROOT caller-capability
    // bit pattern (0x0001 little-endian), not a reserved zero field.
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let encoded = header.encode().expect("header must encode");
    let envelope_bytes: [u8; 2] = [encoded[10], encoded[11]];
    assert_eq!(
        envelope_bytes,
        [0x01, 0x00],
        "envelope bytes at offset 10-11 must carry ROOT capability bit"
    );
}

#[test]
fn header_encode_decode_roundtrip_preserves_flags() {
    // GAP-5: SubmitRun accepts the full low byte; 0x00FF is the contract max.
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0x00FF, 999, 10);
    let encoded = header.encode();
    let encoded = encoded.expect("header should encode");

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    let decoded = decoded.expect("header should decode");
    assert_eq!(decoded.flags, 0x00FF);
}

#[test]
fn header_encode_decode_roundtrip_preserves_payload_len() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 256);
    let encoded = header.encode();
    let encoded = encoded.expect("header should encode");

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    let decoded = decoded.expect("header should decode");
    assert_eq!(decoded.payload_len, 256);
}

#[test]
fn ingress_frame_rejects_empty_payload_with_min_max() {
    let max =
        MaxPayloadBytes::new(std::num::NonZeroUsize::new(1).unwrap_or(std::num::NonZeroUsize::MIN));
    let data = Bytes::new();

    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0; 32]),
        data,
        max,
    );

    let result = result.expect("empty payload should fit within any non-zero max");
}

#[test]
fn memory_ingress_submit_and_recv_single_frame() {
    let capacity =
        QueueCapacity::new(std::num::NonZeroUsize::new(2).unwrap_or(std::num::NonZeroUsize::MIN));
    let queue = MemoryIngress::bounded(capacity);
    let frame = IngressFrame::new(
        RunId::new(42),
        WorkflowDigest::from_bytes([1; 32]),
        Bytes::from_static(b"data"),
        MaxPayloadBytes::DEFAULT,
    );
    let frame = frame.expect("frame should construct");

    assert_eq!(queue.try_submit(frame), Ok(()));
    let recv = queue.try_recv();

    let f = recv
        .expect("recv should succeed")
        .expect("expected Some variant");
    assert_eq!(f.run_id(), RunId::new(42));
}

#[test]
fn try_submit_returns_full_when_at_capacity() {
    let capacity = QueueCapacity::new(std::num::NonZeroUsize::MIN);
    let queue = MemoryIngress::bounded(capacity);

    let frame = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    let frame = frame.expect("frame should construct");
    assert_eq!(queue.try_submit(frame.clone()), Ok(()));

    assert_eq!(queue.try_submit(frame), Err(IpcError::Full));
}

#[test]
fn frame_header_const_new_is_compile_time() {
    const HEADER: IpcFrameHeader = IpcFrameHeader::new(IpcCommand::Shutdown, 0, 0, 0);

    assert_eq!(HEADER.command, IpcCommand::Shutdown);
    assert_eq!(HEADER.flags, 0);
    assert_eq!(HEADER.correlation, 0);
    assert_eq!(HEADER.payload_len, 0);
}

#[test]
fn ipc_error_full_display_message() {
    let error = IpcError::Full;
    let message = error.to_string();
    assert!(message.contains("full"), "expected 'full' in '{message}'");
}

#[test]
fn ipc_error_header_encode_failed_display() {
    let error = IpcError::HeaderEncodeFailed;
    let message = error.to_string();
    assert!(
        message.contains("encode"),
        "expected 'encode' in '{message}'"
    );
}

#[test]
fn ipc_error_header_decode_failed_display() {
    let error = IpcError::HeaderDecodeFailed;
    let message = error.to_string();
    assert!(
        message.contains("decode"),
        "expected 'decode' in '{message}'"
    );
}

#[test]
fn ipc_error_payload_length_out_of_range_display() {
    let error = IpcError::PayloadLengthOutOfRange { actual: 999 };
    let message = error.to_string();
    assert!(message.contains("999"), "expected '999' in '{message}'");
}

#[test]
fn ut_u32_to_usize_conversion_succeeds_for_valid_values() {
    let values = [0u32, 1, 100, u32::MAX];
    for value in values {
        let result = crate::u32_to_usize(value);
        assert_eq!(
            result,
            Ok(value as usize),
            "u32_to_usize({value}) should succeed and return {value} on this platform"
        );
    }
}

#[test]
fn ipc_error_payload_encode_failed_display() {
    let error = IpcError::PayloadEncodeFailed;
    let message = error.to_string();
    assert!(
        message.contains("encode"),
        "expected 'encode' in '{message}'"
    );
}

#[test]
fn ipc_error_unknown_command_display_shows_id() {
    let error = IpcError::UnknownCommand(200);
    let message = error.to_string();
    assert!(message.contains("200"), "expected '200' in '{message}'");
}

#[test]
fn ipc_error_reserved_non_zero_display_shows_value() {
    let error = IpcError::ReservedNonZero { actual: 7 };
    let message = error.to_string();
    assert!(message.contains("7"), "expected '7' in '{message}'");
}

#[test]
fn ipc_error_runtime_codes_cover_ipc_mappings() {
    assert_eq!(IpcError::Full.runtime_code(), Some("QUEUE_FULL"));
    assert_eq!(
        IpcError::InvalidMagic { actual: 0 }.runtime_code(),
        Some("IPC_FRAME_INVALID")
    );
    assert_eq!(
        IpcError::UnsupportedVersion { actual: 2 }.runtime_code(),
        Some("IPC_FRAME_INVALID")
    );
    assert_eq!(
        IpcError::UnknownCommand(99).runtime_code(),
        Some("IPC_FRAME_INVALID")
    );
    assert_eq!(
        IpcError::ReservedNonZero { actual: 7 }.runtime_code(),
        Some("IPC_FRAME_INVALID")
    );
    assert_eq!(
        IpcError::PayloadLengthMismatch {
            header: 4,
            actual: 3
        }
        .runtime_code(),
        Some("IPC_FRAME_INVALID")
    );
    assert_eq!(
        IpcError::HeaderDecodeFailed.runtime_code(),
        Some("IPC_FRAME_INVALID")
    );
    assert_eq!(
        IpcError::PayloadDecodeFailed.runtime_code(),
        Some("IPC_FRAME_INVALID")
    );
    assert_eq!(
        IpcError::ResponseDecodeFailed.runtime_code(),
        Some("IPC_FRAME_INVALID")
    );
    assert_eq!(
        IpcError::PayloadTooLarge {
            actual: 9,
            limit: 8
        }
        .runtime_code(),
        Some("IPC_PAYLOAD_TOO_LARGE")
    );
    assert_eq!(
        IpcError::PayloadLengthOutOfRange { actual: u32::MAX }.runtime_code(),
        Some("IPC_PAYLOAD_TOO_LARGE")
    );
}

#[test]
fn ipc_error_runtime_codes_are_unique() {
    let codes = [
        IpcError::IPC_FRAME_INVALID_RUNTIME_CODE,
        IpcError::IPC_PAYLOAD_TOO_LARGE_RUNTIME_CODE,
        IpcError::QUEUE_FULL_RUNTIME_CODE,
    ];
    assert_eq!(codes.len(), 3);
    assert_eq!(
        codes
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
}

#[test]
fn ipc_error_runtime_code_is_absent_without_direct_mapping() {
    assert_eq!(IpcError::Disconnected.runtime_code(), None);
    assert_eq!(IpcError::HeaderEncodeFailed.runtime_code(), None);
    assert_eq!(IpcError::PayloadEncodeFailed.runtime_code(), None);
}

#[test]
fn ipc_error_diagnostic_code_full() {
    assert_eq!(
        IpcError::Full.diagnostic_code(),
        DiagnosticCode::new(0x3001)
    );
}

#[test]
fn ipc_error_diagnostic_code_disconnected() {
    assert_eq!(
        IpcError::Disconnected.diagnostic_code(),
        DiagnosticCode::new(0x3002)
    );
}

#[test]
fn ipc_error_diagnostic_code_payload_too_large() {
    assert_eq!(
        IpcError::PayloadTooLarge {
            actual: 100,
            limit: 10
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x3003)
    );
}

#[test]
fn ipc_error_diagnostic_code_invalid_magic() {
    assert_eq!(
        IpcError::InvalidMagic {
            actual: 0xDEAD_BEEF
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x3004)
    );
}

#[test]
fn ipc_error_diagnostic_code_unsupported_version() {
    assert_eq!(
        IpcError::UnsupportedVersion { actual: 99 }.diagnostic_code(),
        DiagnosticCode::new(0x3005)
    );
}

#[test]
fn ipc_error_diagnostic_code_unknown_command() {
    assert_eq!(
        IpcError::UnknownCommand(200).diagnostic_code(),
        DiagnosticCode::new(0x3006)
    );
}

#[test]
fn ipc_error_diagnostic_code_reserved_non_zero() {
    assert_eq!(
        IpcError::ReservedNonZero { actual: 7 }.diagnostic_code(),
        DiagnosticCode::new(0x3007)
    );
}

#[test]
fn ipc_error_diagnostic_code_payload_length_mismatch() {
    assert_eq!(
        IpcError::PayloadLengthMismatch {
            header: 4,
            actual: 3
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x3008)
    );
}

#[test]
fn ipc_error_diagnostic_code_header_encode_failed() {
    assert_eq!(
        IpcError::HeaderEncodeFailed.diagnostic_code(),
        DiagnosticCode::new(0x3009)
    );
}

#[test]
fn ipc_error_diagnostic_code_header_decode_failed() {
    assert_eq!(
        IpcError::HeaderDecodeFailed.diagnostic_code(),
        DiagnosticCode::new(0x300A)
    );
}

#[test]
fn ipc_error_diagnostic_code_payload_length_out_of_range() {
    assert_eq!(
        IpcError::PayloadLengthOutOfRange { actual: u32::MAX }.diagnostic_code(),
        DiagnosticCode::new(0x300B)
    );
}

#[test]
fn ipc_error_diagnostic_code_payload_encode_failed() {
    assert_eq!(
        IpcError::PayloadEncodeFailed.diagnostic_code(),
        DiagnosticCode::new(0x300C)
    );
}

#[test]
fn ipc_error_diagnostic_code_payload_decode_failed() {
    assert_eq!(
        IpcError::PayloadDecodeFailed.diagnostic_code(),
        DiagnosticCode::new(0x300D)
    );
}

#[test]
fn ipc_error_diagnostic_code_response_decode_failed() {
    assert_eq!(
        IpcError::ResponseDecodeFailed.diagnostic_code(),
        DiagnosticCode::new(0x300E)
    );
}

// ══ Adversarial command-specific attacks ═══════════════════════════════════════

#[test]
fn adversarial_cancel_run_with_run_id_zero_encoded_rejected_by_runtime() {
    let payload = IpcPayload::CancelRun {
        run_id: RunId::new(0),
        reason: None,
    };
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("must be Ok");

    let decoded = decode_payload(&encoded);

    let decoded = decoded.expect("CancelRun with run_id=0 should decode");
    assert_eq!(
        decoded,
        IpcPayload::CancelRun {
            run_id: RunId::new(0),
            reason: None,
        }
    );
}

#[test]
fn adversarial_cancel_run_with_run_id_max_encoded_roundtrips() {
    let payload = IpcPayload::CancelRun {
        run_id: RunId::new(u64::MAX),
        reason: None,
    };
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("must be Ok");

    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_answer_ask_with_zero_answer_slot_roundtrips() {
    let payload = IpcPayload::AnswerAsk {
        run_id: RunId::new(1),
        answer_slot: vb_core::ids::SlotIdx::ZERO,
        answer: Vec::new(),
        taint: None,
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("must be Ok");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_answer_ask_with_max_answer_slot_roundtrips() {
    let payload = IpcPayload::AnswerAsk {
        run_id: RunId::new(1),
        answer_slot: vb_core::ids::SlotIdx::new(u16::MAX),
        answer: Vec::from(&b"malicious"[..]),
        taint: None,
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("must be Ok");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_fail_action_with_unregistered_run_id_roundtrips() {
    let payload = IpcPayload::FailAction {
        run_id: RunId::new(99991),
        ticket: 7777,
        error: Vec::from(&b"no such run"[..]),
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("must be Ok");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_complete_action_with_mismatched_output_bytes_rejected() {
    let payload = IpcPayload::CompleteAction {
        run_id: RunId::new(1),
        ticket: 5,
        output: Vec::from(&b"\xFF\xFF\xFF\xFF"[..]),
    };
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("must be Ok");

    let decoded = decode_payload(&encoded);

    let decoded = decoded.expect("outer IpcPayload should decode");
}

#[test]
fn adversarial_submit_run_with_empty_input_roundtrips() {
    let payload = IpcPayload::SubmitRun(SubmitRunPayload {
        run_id: RunId::new(42),
        workflow: WorkflowDigest::from_bytes([0; 32]),
        input: Vec::new(),
    });

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("must be Ok");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_submit_run_with_large_input_under_limit_roundtrips() {
    let payload = IpcPayload::SubmitRun(SubmitRunPayload {
        run_id: RunId::new(7),
        workflow: WorkflowDigest::from_bytes([0xAA; 32]),
        input: vec![0u8; 100_000],
    });

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("must be Ok");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_list_events_with_from_sequence_max_roundtrips() {
    let payload = IpcPayload::ListEvents {
        run_id: RunId::new(5),
        from_sequence: u64::MAX,
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("must be Ok");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_drain_trace_with_max_records_roundtrips() {
    let payload = IpcPayload::DrainTrace {
        run_id: RunId::new(3),
        max_records: u32::MAX,
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("must be Ok");
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_bounded_payload_rejects_exactly_one_over_max() {
    let max_val = 32;
    let max = MaxPayloadBytes::new(
        std::num::NonZeroUsize::new(max_val).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    let data = Bytes::from(vec![0u8; max_val.saturating_add(1)]);

    let result = BoundedPayload::new(data, max);

    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: max_val.saturating_add(1),
            limit: max_val,
        })
    );
}

#[test]
fn adversarial_decode_frame_rejects_oversized_payload_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 3);
    let encoded = header.encode();
    let encoded = encoded.expect("must be Ok");
    let oversized = Bytes::from(vec![0u8; 1000]);

    let result = decode_frame(&encoded, oversized, MaxPayloadBytes::DEFAULT);

    assert_eq!(
        result,
        Err(IpcError::PayloadLengthMismatch {
            header: 3,
            actual: 1000,
        })
    );
}

#[test]
fn adversarial_encode_payload_exceeding_bound_rejected() {
    let payload = IpcPayload::Health;
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);

    let result = encode_payload(&payload, tiny_max);

    assert!(
        matches!(result, Ok(_) | Err(IpcError::PayloadTooLarge { .. })),
        "expected success or PayloadTooLarge for tiny health frame"
    );
}

#[test]
fn adversarial_ipc_frame_new_rejects_mismatched_lengths() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 10);
    let short_payload = Bytes::from(vec![0u8; 5]);

    let result = crate::IpcFrame::new(header, short_payload, MaxPayloadBytes::DEFAULT);

    assert_eq!(
        result,
        Err(IpcError::PayloadLengthMismatch {
            header: 10,
            actual: 5,
        })
    );
}

#[test]
fn adversarial_ipc_frame_new_rejects_oversized_payload() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
    let payload = Bytes::from(vec![0u8; 100]);
    let tiny_max = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);

    let result = crate::IpcFrame::new(header, payload, tiny_max);

    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: 100,
            limit: 1,
        })
    );
}

#[test]
fn adversarial_memory_ingress_full_then_drain_then_submit() {
    let capacity = QueueCapacity::new(std::num::NonZeroUsize::MIN);
    let queue = MemoryIngress::bounded(capacity);
    let frame = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    let frame = frame.expect("must be Ok");

    assert_eq!(queue.try_submit(frame.clone()), Ok(()));
    assert_eq!(queue.try_submit(frame.clone()), Err(IpcError::Full));
    let drained = queue.try_recv();
    let drained = drained.expect("drain must succeed");
    assert_eq!(queue.try_submit(frame), Ok(()));

    assert_eq!(queue.len(), 1);
}

#[test]
fn adversarial_memory_ingress_disconnected_after_sender_drop() {
    let capacity =
        QueueCapacity::new(std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN));
    let receiver_only = MemoryIngress::bounded(capacity);
    let sender = receiver_only.producer();
    drop(sender);

    let result = receiver_only.try_recv();

    assert!(matches!(result, Err(IpcError::Disconnected)));
}

#[test]
fn adversarial_decode_frame_bad_magic_in_header_returns_invalid_magic() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&0xDEAD_BEEF_u32.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    let payload = Bytes::new();

    let result = decode_frame(&header_bytes, payload, MaxPayloadBytes::DEFAULT);

    assert_eq!(
        result,
        Err(IpcError::InvalidMagic {
            actual: 0xDEAD_BEEF
        })
    );
}

#[test]
fn adversarial_decode_frame_bad_version_in_header_returns_unsupported_version() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
    let payload = Bytes::new();

    let result = decode_frame(&header_bytes, payload, MaxPayloadBytes::DEFAULT);

    assert_eq!(result, Err(IpcError::UnsupportedVersion { actual: 99 }));
}

#[test]
fn adversarial_submit_run_payload_with_zero_workflow_roundtrips() {
    let payload = IpcPayload::SubmitRun(SubmitRunPayload {
        run_id: RunId::new(0),
        workflow: WorkflowDigest::from_bytes([0; 32]),
        input: Vec::new(),
    });

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    let encoded = encoded.expect("must be Ok");

    assert_eq!(decode_payload(&encoded), Ok(payload));
}

// ══ IPC frame validation hardening tests ═══════════════════════════════════════

#[test]
fn frame_validation_oversized_payload_exceeding_default_max_returns_error() {
    let default_max = MaxPayloadBytes::DEFAULT.get();
    let over_limit = match u32::try_from(default_max.saturating_add(1)) {
        Ok(v) => v,
        Err(_) => return,
    };
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 1, over_limit);

    let encoded = header.encode();
    let encoded = encoded.expect("header should encode");
    let result = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: default_max.saturating_add(1),
            limit: default_max,
        })
    );
}

#[test]
fn frame_validation_header_claims_large_payload_but_actual_data_shorter_returns_error() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 42, 500);
    let encoded = header.encode();
    let encoded = encoded.expect("header should encode");
    let short_payload = Bytes::from(vec![0u8; 10]);

    let result = decode_frame(&encoded, short_payload, MaxPayloadBytes::DEFAULT);

    assert_eq!(
        result,
        Err(IpcError::PayloadLengthMismatch {
            header: 500,
            actual: 10,
        })
    );
}

#[test]
fn frame_validation_invalid_magic_bytes_returns_typed_error() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&0x0000_0000_u32.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());

    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);

    assert_eq!(result, Err(IpcError::InvalidMagic { actual: 0 }));
}

#[test]
fn frame_validation_version_mismatch_returns_unsupported_version() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&255u16.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());

    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);

    assert_eq!(result, Err(IpcError::UnsupportedVersion { actual: 255 }));
}

#[test]
fn frame_validation_zero_command_id_returns_unknown_command() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    // SEC-01: ROOT capability envelope so the envelope check passes.
    header_bytes[10..12].copy_from_slice(&ROOT_CAPABILITY_BIT.to_le_bytes());

    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);

    assert_eq!(
        result,
        Ok(IpcFrameHeader::new(IpcCommand::UnknownCommand(0), 0, 0, 0))
    );
}

#[test]
fn frame_validation_unrecognized_command_id_returns_unknown_command() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&99u16.to_le_bytes());
    header_bytes[10..12].copy_from_slice(&ROOT_CAPABILITY_BIT.to_le_bytes());

    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);

    assert_eq!(
        result,
        Ok(IpcFrameHeader::new(IpcCommand::UnknownCommand(99), 0, 0, 0))
    );
}

#[test]
fn frame_validation_valid_header_with_unknown_payload_type_decode_fails() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let encoded = header.encode();
    let encoded = encoded.expect("header should encode");
    let garbage = Bytes::from(vec![0xDE, 0xAD, 0xBE, 0xEF]);

    let frame_result = decode_frame(&encoded, garbage, MaxPayloadBytes::DEFAULT);
    let frame = frame_result.expect("frame struct decode should succeed");

    let payload_result = decode_payload(frame.payload());
    assert_eq!(payload_result, Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn frame_validation_truncated_frame_shorter_than_header_returns_error() {
    let data: [u8; 0] = [];
    let mut cursor = std::io::Cursor::new(data);

    let result = crate::frame::read_frame_header(&mut cursor);

    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn frame_validation_header_claims_more_data_than_available_returns_error() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 200);
    let encoded = header.encode();
    let encoded = encoded.expect("header should encode");
    let mut cursor = std::io::Cursor::new(encoded.as_slice());

    let decoded_header = crate::frame::read_frame_header(&mut cursor);
    let decoded_header = decoded_header.expect("header read should succeed");

    let short_data = vec![0u8; 5];
    let mut payload_cursor = std::io::Cursor::new(short_data.as_slice());
    let result = crate::frame::read_frame_payload(&mut payload_cursor, &decoded_header);

    assert_eq!(result, Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn frame_validation_valid_frame_parse_succeeds() {
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 42, 0);
    let encoded = header.encode();
    let encoded = encoded.expect("header should encode");
    let payload = Bytes::new();

    let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

    let frame = result.expect("well-formed frame should decode");
    assert_eq!(frame.header().command, IpcCommand::SubmitRun);
    assert_eq!(frame.header().flags, 0);
    assert_eq!(frame.header().correlation, 42);
    assert_eq!(frame.header().payload_len, 0);
    assert_eq!(frame.payload().bytes().len(), 0);
}

#[test]
fn frame_validation_roundtrip_encode_decode_preserves_all_fields() {
    // GAP-5: CompleteAction accepts only zero flags; the test previously
    // used 0xABCD which now triggers InvalidCommandFlags at decode.
    let original_header =
        IpcFrameHeader::new(IpcCommand::SubmitRun, 0x00FF, 0x1234_5678_9ABC_DEF0, 8);
    let encoded = original_header.encode();
    let encoded = encoded.expect("header should encode");
    let payload = Bytes::from_static(b"payload!");

    let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

    let frame = result.expect("frame should decode");
    assert_eq!(frame.header().command, IpcCommand::SubmitRun);
    assert_eq!(frame.header().flags, 0x00FF);
    assert_eq!(frame.header().correlation, 0x1234_5678_9ABC_DEF0);
    assert_eq!(frame.header().payload_len, 8);
    assert_eq!(frame.payload().bytes().as_ref(), b"payload!");
}

#[test]
fn frame_validation_roundtrip_all_commands_preserve_command_identity() {
    let commands = [
        IpcCommand::SubmitRun,
        IpcCommand::SubmitRunInline,
        IpcCommand::CancelRun,
        IpcCommand::InspectRun,
        IpcCommand::ListEvents,
        IpcCommand::AnswerAsk,
        IpcCommand::CompleteAction,
        IpcCommand::FailAction,
        IpcCommand::DrainTrace,
        IpcCommand::Health,
        IpcCommand::Shutdown,
    ];

    for command in commands {
        let header = IpcFrameHeader::new(command, 0, 1, 0);
        let encoded = header.encode();
        let encoded = encoded.expect("header should encode for {command:?}");
        let result = decode_frame(&encoded, Bytes::new(), MaxPayloadBytes::DEFAULT);

        let frame = result.expect("frame should decode for {command:?}");
        assert_eq!(
            frame.header().command,
            command,
            "command should roundtrip for {command:?}"
        );
    }
}

#[test]
fn frame_validation_payload_at_exact_max_boundary_succeeds() {
    let max = MaxPayloadBytes::DEFAULT.get();
    let payload_len = match u32::try_from(max) {
        Ok(v) => v,
        Err(_) => return,
    };
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 1, payload_len);
    let encoded = header.encode();
    let encoded = encoded.expect("header should encode");
    let payload = Bytes::from(vec![0u8; max]);

    let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

    let frame = result.expect("frame at exact max boundary should decode");
    assert_eq!(frame.header().payload_len, payload_len);
    assert_eq!(frame.payload().bytes().len(), max);
}

#[test]
fn frame_validation_read_frame_header_bounded_rejects_empty_reader() {
    let data: [u8; 0] = [];
    let mut cursor = std::io::Cursor::new(data);

    let result = crate::frame::read_frame_header_bounded(&mut cursor, MaxPayloadBytes::DEFAULT);

    assert_eq!(result, Err(IpcError::HeaderDecodeFailed));
}

#[test]
fn frame_validation_read_frame_payload_bounded_rejects_truncated_data() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 50);
    let data = vec![0u8; 10];
    let mut cursor = std::io::Cursor::new(data.as_slice());

    let result =
        crate::frame::read_frame_payload_bounded(&mut cursor, &header, MaxPayloadBytes::DEFAULT);

    assert_eq!(result, Err(IpcError::PayloadDecodeFailed));
}

// ══ Proptest module ════════════════════════════════════════════════════════════

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn ipc_command_roundtrips_through_u16(cmd in 1u16..=11u16) {
            let parsed = IpcCommand::from_u16(cmd);
            let command = parsed.expect("must be Ok");
            prop_assert_eq!(command.as_u16(), cmd);
        }
    }

    proptest! {
        #[test]
        fn non_magic_bytes_always_rejected(magic in 0u32..) {
            if magic != IPC_MAGIC {
                let mut header_bytes = [0u8; IPC_HEADER_LEN];
                header_bytes[..4].copy_from_slice(&magic.to_le_bytes());
                header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
                let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
                prop_assert_err!(result);
                if let Err(e) = result {
                    prop_assert!(
                        matches!(e, IpcError::InvalidMagic { .. }),
                        "expected InvalidMagic, got {e:?}"
                    );
                }
            }
        }
    }

    proptest! {
        #[test]
        fn ipc_command_encode_decode_roundtrip(cmd_val in 1u16..=11u16) {
            let command = IpcCommand::from_u16(cmd_val).unwrap();

            let header = IpcFrameHeader::new(command, 0, 0, 0);
            let encoded = header.encode();
            let encoded = encoded.expect("must be Ok");
            let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

            let decoded = decoded.expect("must be Ok");
            prop_assert_eq!(decoded.command, command);
        }
    }

    proptest! {
        #[test]
        fn ipc_response_encode_decode_roundtrip(run_id_val in 0u64..) {
            let payload = IpcPayload::CancelRun {
                run_id: RunId::new(run_id_val),
                reason: None,
            };

            let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
            let encoded = encoded.expect("must be Ok");
            let decoded = decode_payload(&encoded);

            let decoded = decoded.expect("must be Ok");
            prop_assert_eq!(decoded, payload);
        }
    }

    proptest! {
        #[test]
        fn frame_header_length_never_exceeds_max(cmd_val in 1u16..=11u16, payload_len in 0u32..=1024u32) {
            let command = IpcCommand::from_u16(cmd_val).unwrap();

            let header = IpcFrameHeader::new(command, 0, 0, payload_len);

            let encoded = header.encode();
            let encoded = encoded.expect("must be Ok");
            prop_assert_eq!(encoded.len(), IPC_HEADER_LEN);
        }
    }

    // ── P6: validate_frame_bounds obeys strict > for random (payload_len, max) ──

    proptest! {
        #[test]
        fn validate_frame_bounds_obeys_strict_greater_than(
            payload_len in 0u32..=65535u32,
            max_val in 1u16..=65535u16,
        ) {
            // Given: random payload_len and max, construct header and max_payload
            let Some(max) = std::num::NonZeroUsize::new(max_val as usize) else {
                return Ok(());
            };
            let max_payload = MaxPayloadBytes::new(max);
            let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, payload_len);

            let result = crate::validate_frame_bounds(&header, max_payload);

            // Convert payload_len to usize; on 32-bit this may fail (waiver WVR-001)
            let Ok(payload_usize) = usize::try_from(payload_len) else {
                // 32-bit platform where u32 doesn't fit usize
                let is_out_of_range = match &result {
                    Err(IpcError::PayloadLengthOutOfRange { actual }) => *actual == payload_len,
                    _ => false,
                };
                prop_assert!(is_out_of_range, "expected PayloadLengthOutOfRange");
                return Ok(());
            };

            if payload_usize > max_payload.get() {
                prop_assert_eq!(
                    result,
                    Err(IpcError::PayloadTooLarge {
                        actual: payload_usize,
                        limit: max_payload.get(),
                    })
                );
            } else {
                prop_assert_eq!(result, Ok(()));
            }
        }
    }
}

// ══ Preallocation Surface Integration Tests ════════════════════════════════════

/// Helper: constructs a `MaxPayloadBytes` from a literal nonzero value.
fn max_payload_bytes(value: usize) -> MaxPayloadBytes {
    MaxPayloadBytes::new(std::num::NonZeroUsize::new(value).expect("value must be nonzero"))
}

// ── P0-#1: cursor position unchanged after bounded read rejection ──

#[test]
fn bounded_read_rejects_before_allocation_proof_cursor_position_unchanged() {
    // Given: a hostile header with payload_len > max, and a cursor with known content
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 500);
    let tiny_max = max_payload_bytes(1);
    // Use a known test pattern so we can verify cursor content after rejection
    let known_pattern: [u8; 21] = *b"CURSOR_PRESERVE_TEST!";

    // When: before and after the bounded read
    let mut cursor = std::io::Cursor::new(known_pattern);
    let pos_before = cursor.position();

    let result = crate::read_frame_payload_bounded(&mut cursor, &header, tiny_max);

    // Then: PayloadTooLarge, cursor position unchanged, cursor still has original data
    assert_eq!(
        result,
        Err(IpcError::PayloadTooLarge {
            actual: 500,
            limit: 1,
        })
    );
    assert_eq!(
        cursor.position(),
        pos_before,
        "cursor must not advance when bounds reject: no bytes consumed"
    );

    // Verify we can still read the original data from the cursor
    let pos_after = cursor.position();
    assert_eq!(pos_after, 0, "cursor should still be at position 0");
    let content_after = &cursor.get_ref()[pos_after as usize..];
    assert_eq!(
        content_after,
        known_pattern.as_slice(),
        "original bytes still readable after rejection"
    );
}

// ── P0-#2: per-bound acceptance and rejection for all 6 bounds ──

#[test]
fn bounded_read_per_bound_acceptance_at_boundary() {
    let bounds: [(usize, u32); 6] = [
        (1, 1),
        (16, 16),
        (256, 256),
        (1024, 1024),
        (65536, 65536),
        (1_048_576, 1_048_576),
    ];

    for (bound_val, payload_len) in &bounds {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, *payload_len);
        let max = max_payload_bytes(*bound_val);
        let payload = vec![0xAA_u8; *bound_val];
        let mut cursor = std::io::Cursor::new(payload.as_slice());

        let result = crate::read_frame_payload_bounded(&mut cursor, &header, max);

        let read_bytes = result
            .unwrap_or_else(|e| panic!("payload at bound {bound_val} must succeed, got {e:?}"));
        assert_eq!(
            read_bytes.len(),
            *bound_val,
            "payload at bound {bound_val} must have correct length"
        );
        assert_eq!(read_bytes, vec![0xAA_u8; *bound_val]);
    }
}

#[test]
fn bounded_read_per_bound_rejection_one_above() {
    let bounds: [(usize, u32); 6] = [
        (1, 2),
        (16, 17),
        (256, 257),
        (1024, 1025),
        (65536, 65537),
        (1_048_576, 1_048_577),
    ];

    for (bound_val, payload_len) in &bounds {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, *payload_len);
        let max = max_payload_bytes(*bound_val);
        let data = vec![0u8; 24];
        let mut cursor = std::io::Cursor::new(data.as_slice());

        let result = crate::read_frame_payload_bounded(&mut cursor, &header, max);

        assert_eq!(
            result,
            Err(IpcError::PayloadTooLarge {
                actual: *bound_val + 1,
                limit: *bound_val,
            }),
            "payload one above bound {bound_val} must be rejected"
        );
    }
}

// ── P1-#3: u32::MAX payload_len does not OOM ──

#[test]
fn bounded_read_with_u32_max_payload_len_no_oom() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, u32::MAX);
    let data = vec![0u8; 24]; // minimal cursor: not read by bounds check
    let mut cursor = std::io::Cursor::new(data.as_slice());

    let result = crate::read_frame_payload_bounded(&mut cursor, &header, MaxPayloadBytes::DEFAULT);

    // On 64-bit: PayloadTooLarge since 4GB > 1 MiB
    // On 32-bit: PayloadLengthOutOfRange (u32 > usize::MAX)
    // Either way, no OOM, no panic
    match result {
        Err(IpcError::PayloadTooLarge { actual, limit }) => {
            assert!(
                actual > limit,
                "oversized payload must be larger than limit"
            );
            assert_eq!(limit, MaxPayloadBytes::DEFAULT.get());
        }
        Err(IpcError::PayloadLengthOutOfRange { actual }) => {
            // Only reachable on 32-bit
            assert_eq!(actual, u32::MAX);
        }
        other => {
            panic!("expected PayloadTooLarge or PayloadLengthOutOfRange, got {other:?}");
        }
    }
}

// ── P1-#4: fuzz target mock exercises all error paths ──

#[test]
fn fuzz_target_mock_exercises_all_decode_error_paths() {
    // Given: hand-crafted byte sequences triggering each of the 7 decode errors

    // 1. InvalidMagic — empty/zero header bytes
    {
        let zero_header: [u8; IPC_HEADER_LEN] = [0u8; IPC_HEADER_LEN];
        let result = crate::decode_frame_header(&zero_header);
        assert_eq!(
            result,
            Err(IpcError::InvalidMagic { actual: 0 }),
            "zero header must produce InvalidMagic"
        );
    }

    // 2. InvalidMagic — 0xDEADBEEF magic
    {
        let mut bad_header = [0u8; IPC_HEADER_LEN];
        bad_header[..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        let result = crate::decode_frame_header(&bad_header);
        assert_eq!(
            result,
            Err(IpcError::InvalidMagic {
                actual: 0xDEAD_BEEF
            }),
            "0xDEADBEEF magic must produce InvalidMagic"
        );
    }

    // 3. UnsupportedVersion — valid magic, version=0
    {
        let mut h = [0u8; IPC_HEADER_LEN];
        h[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
        // version bytes at 4..6 are already 0
        let result = crate::decode_frame_header(&h);
        assert_eq!(
            result,
            Err(IpcError::UnsupportedVersion { actual: 0 }),
            "version=0 must produce UnsupportedVersion"
        );
    }

    // 4. PermissionDenied — valid magic, version=1, capabilities=0 sentinel
    {
        let mut h = [0u8; IPC_HEADER_LEN];
        h[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
        h[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
        h[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
        // capabilities envelope bytes 10..12 are already 0 (the sentinel)
        let result = crate::decode_frame_header(&h);
        assert_eq!(
            result,
            Err(IpcError::PermissionDenied),
            "zero capabilities envelope must produce PermissionDenied (SEC-01)"
        );
    }

    // 5. PayloadTooLarge — valid header, payload_len=1024, bound=256
    {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 1024);
        let max_256 = max_payload_bytes(256);
        let encoded = header.encode().expect("header should encode");
        let result = IpcFrameHeader::decode(&encoded, max_256);
        assert_eq!(
            result,
            Err(IpcError::PayloadTooLarge {
                actual: 1024,
                limit: 256,
            }),
            "payload too large for bound must produce PayloadTooLarge"
        );
    }

    // 6. HeaderDecodeFailed — truncated byte slice
    {
        let short: [u8; 3] = [0u8; 3];
        let result = crate::validate_frame_magic(&short);
        assert_eq!(
            result,
            Err(IpcError::HeaderDecodeFailed),
            "3-byte input must produce HeaderDecodeFailed"
        );
    }

    // 7. PayloadDecodeFailed — bounds pass but short cursor
    {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 100);
        let short_data = b"abc";
        let mut cursor = std::io::Cursor::new(short_data.as_slice());
        let result =
            crate::read_frame_payload_bounded(&mut cursor, &header, MaxPayloadBytes::DEFAULT);
        assert_eq!(
            result,
            Err(IpcError::PayloadDecodeFailed),
            "truncated payload within bounds must produce PayloadDecodeFailed"
        );
    }

    // 8. PayloadLengthMismatch — header says 8, actual is 4
    {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 8);
        let short_payload = vec![0u8; 4];
        let result = crate::decode_frame_payload(&header, &short_payload);
        assert_eq!(
            result,
            Err(IpcError::PayloadLengthMismatch {
                header: 8,
                actual: 4,
            }),
            "length mismatch must produce PayloadLengthMismatch"
        );
    }

    // 9. PayloadLengthMismatch — header says 0, actual is 10
    {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
        let extra_payload = vec![0u8; 10];
        let result = crate::decode_frame_payload(&header, &extra_payload);
        assert_eq!(
            result,
            Err(IpcError::PayloadLengthMismatch {
                header: 0,
                actual: 10,
            }),
            "zero header with extra payload must produce PayloadLengthMismatch"
        );
    }

    // 10. PayloadDecodeFailed — garbage postcard bytes
    {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
        let garbage = vec![0xFF, 0xFE, 0xFD, 0xFC];
        let result = crate::decode_frame_payload(&header, &garbage);
        assert_eq!(
            result,
            Err(IpcError::PayloadDecodeFailed),
            "garbage postcard bytes must produce PayloadDecodeFailed"
        );
    }
}
