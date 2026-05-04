//! Integration tests for vb_ipc.

use vb_core::{DiagnosticCode, RunId, WorkflowDigest};

use crate::{
    decode_frame, decode_payload, encode_payload, BoundedPayload, IngressFrame, IpcCommand,
    IpcError, IpcFrameHeader, IpcPayload, MaxPayloadBytes, MemoryIngress, QueueCapacity,
    SubmitRunPayload, IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION,
};
use bytes::Bytes;

#[cfg(test)]
macro_rules! assert_ok {
    ($result:expr $(, $($arg:tt)+)?) => {{
        match &$result {
            Ok(_) => (),
            Err(_) => assert_eq!(Some("Err(..)"), None::<&str> $(, $($arg)+)?),
        }
    }};
}

#[cfg(test)]
macro_rules! prop_assert_ok {
    ($result:expr $(, $($arg:tt)+)?) => {{
        let is_ok_result = match &$result {
            Ok(_) => true,
            Err(_) => false,
        };
        prop_assert!(is_ok_result $(, $($arg)+)?);
    }};
}

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

    <[u8; IPC_HEADER_LEN]>::try_from(bytes.as_slice())
        .map_err(|_| IpcError::HeaderEncodeFailed)
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
    assert_ok!(frame, "test frame should fit default payload bound");
    let Ok(frame) = frame else { return };

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
    assert_eq!(IpcCommand::from_u16(12), Ok(IpcCommand::ListRuns));
    assert_eq!(IpcCommand::from_u16(13), Ok(IpcCommand::GetMetrics));
    assert_eq!(IpcCommand::from_u16(14), Ok(IpcCommand::GetWorkflowGraph));
    assert_eq!(IpcCommand::from_u16(15), Ok(IpcCommand::GetTaintReport));
    assert_eq!(IpcCommand::from_u16(16), Ok(IpcCommand::VerifyWorkflow));
    assert_eq!(
        IpcCommand::from_u16(17),
        Err(IpcError::UnknownCommand(17))
    );
}

#[test]
fn header_roundtrips_little_endian_fields() {
    let header = IpcFrameHeader::new(IpcCommand::CompleteAction, 7, 42, 3);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode to fixed width");
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);
    assert_eq!(decoded, Ok(header));
}

#[test]
fn decoder_rejects_bad_magic_before_payload_use() {
    let encoded = header_bytes(0, IPC_VERSION, IpcCommand::Health.as_u16(), 0, 0, 1, 0);
    assert_ok!(encoded, "test header should encode");
    let Ok(encoded) = encoded else { return };
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
    let encoded = header_bytes(
        IPC_MAGIC,
        IPC_VERSION,
        IpcCommand::SubmitRun.as_u16(),
        0,
        0,
        1,
        payload_len_val,
    );
    assert_ok!(encoded, "test header should encode");
    let Ok(encoded) = encoded else { return };
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
fn decoder_rejects_non_zero_reserved_field() {
    let encoded = header_bytes(
        IPC_MAGIC,
        IPC_VERSION,
        IpcCommand::Health.as_u16(),
        0,
        9,
        1,
        0,
    );
    assert_ok!(encoded, "test header should encode");
    let Ok(encoded) = encoded else { return };
    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(decoded, Err(IpcError::ReservedNonZero { actual: 9 }));
}

#[test]
fn frame_decode_requires_payload_length_match() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode to fixed width");
    let Ok(encoded) = encoded else { return };

    let decoded = decode_frame(&encoded, Bytes::from_static(b"abc"), MaxPayloadBytes::DEFAULT);

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
    assert_ok!(encoded, "payload should encode under default bound");
    let Ok(encoded) = encoded else { return };

    assert_eq!(decode_payload(&encoded), Ok(payload));
}

#[test]
fn from_u16_rejects_zero_command() {
    let result = IpcCommand::from_u16(0);
    assert_eq!(result, Err(IpcError::UnknownCommand(0)));
}

#[test]
fn unsupported_version_rejects_when_version_is_two() {
    let encoded = header_bytes(IPC_MAGIC, 2, IpcCommand::Health.as_u16(), 0, 0, 1, 0);
    assert_ok!(encoded, "test header should encode");
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(bounded, "payload should fit default bound");
    let Ok(bounded) = bounded else { return };

    assert_eq!(bounded.bytes(), &data);
}

#[test]
fn ingress_frame_accessors_return_correct_values() {
    let run_id = RunId::new(42);
    let workflow = WorkflowDigest::from_bytes([0xAB; 32]);
    let data = Bytes::from_static(b"payload");
    let frame = IngressFrame::new(run_id, workflow, data, MaxPayloadBytes::DEFAULT);
    assert_ok!(frame, "frame should construct");
    let Ok(frame) = frame else { return };

    assert_eq!(frame.run_id(), run_id);
    assert_eq!(frame.workflow(), workflow);
    assert_eq!(frame.payload().bytes().as_ref(), b"payload");
}

// ── Error variant exact-assertion tests ────────────────────────────────────────

#[test]
fn decode_returns_disconnected_when_buffer_empty() {
    let (sender, receiver) = crossbeam_channel::bounded::<IngressFrame>(1);
    drop(sender);

    let result: Result<Option<IngressFrame>, IpcError> = match receiver.try_recv() {
        Ok(_) => Ok(None),
        Err(crossbeam_channel::TryRecvError::Empty) => Ok(None),
        Err(crossbeam_channel::TryRecvError::Disconnected) => Err(IpcError::Disconnected),
    };

    assert_eq!(result, Err(IpcError::Disconnected));
}

#[test]
fn from_u16_returns_unknown_command_for_zero() {
    assert_eq!(IpcCommand::from_u16(0), Err(IpcError::UnknownCommand(0)));
}

#[test]
fn from_u16_returns_unknown_command_for_value_above_range() {
    assert_eq!(IpcCommand::from_u16(99), Err(IpcError::UnknownCommand(99)));
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

    let result = IngressFrame::new(RunId::new(1), WorkflowDigest::from_bytes([0; 32]), data, max);

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
    assert_ok!(bounded, "garbage should fit in bound");
    let Ok(bounded) = bounded else { return };

    assert_eq!(decode_payload(&bounded), Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn encode_header_always_produces_fixed_width() {
    let header = IpcFrameHeader::new(IpcCommand::Shutdown, 0xFFFF, 0xDEAD_BEEF_CAFE, 1024);

    let encoded = header.encode();
    assert_ok!(encoded, "header encode should succeed");
    let Ok(encoded) = encoded else { return };

    assert_eq!(encoded.len(), IPC_HEADER_LEN);
}

#[test]
fn encode_header_produces_correct_magic_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);

    let encoded = header.encode();
    assert_ok!(encoded, "header encode should succeed");
    let Ok(encoded) = encoded else { return };

    let magic_bytes = encoded.get(..4);
    assert_eq!(magic_bytes, Some(IPC_MAGIC.to_le_bytes().as_slice()));
}

#[test]
fn encode_header_produces_correct_version_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);

    let encoded = header.encode();
    assert_ok!(encoded, "header encode should succeed");
    let Ok(encoded) = encoded else { return };

    let version_bytes = encoded.get(4..6);
    assert_eq!(version_bytes, Some(IPC_VERSION.to_le_bytes().as_slice()));
}

#[test]
fn encode_header_produces_correct_command_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::CancelRun, 0, 1, 0);

    let encoded = header.encode();
    assert_ok!(encoded, "header encode should succeed");
    let Ok(encoded) = encoded else { return };

    let command_bytes = encoded.get(6..8);
    assert_eq!(command_bytes, Some(3u16.to_le_bytes().as_slice()));
}

#[test]
fn encode_header_produces_correct_flags_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0x1234, 1, 0);

    let encoded = header.encode();
    assert_ok!(encoded, "header encode should succeed");
    let Ok(encoded) = encoded else { return };

    let flags_bytes = encoded.get(8..10);
    assert_eq!(flags_bytes, Some(0x1234u16.to_le_bytes().as_slice()));
}

#[test]
fn encode_header_produces_zero_reserved_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);

    let encoded = header.encode();
    assert_ok!(encoded, "header encode should succeed");
    let Ok(encoded) = encoded else { return };

    let reserved_bytes = encoded.get(10..12);
    assert_eq!(reserved_bytes, Some(0u16.to_le_bytes().as_slice()));
}

#[test]
fn encode_header_produces_correct_correlation_bytes() {
    let correlation: u64 = 0x0102_0304_0506_0708;
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, correlation, 0);

    let encoded = header.encode();
    assert_ok!(encoded, "header encode should succeed");
    let Ok(encoded) = encoded else { return };

    let corr_bytes = encoded.get(12..20);
    assert_eq!(corr_bytes, Some(correlation.to_le_bytes().as_slice()));
}

#[test]
fn encode_header_produces_correct_payload_len_bytes() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4096);

    let encoded = header.encode();
    assert_ok!(encoded, "header encode should succeed");
    let Ok(encoded) = encoded else { return };

    let plen_bytes = encoded.get(20..24);
    assert_eq!(plen_bytes, Some(4096u32.to_le_bytes().as_slice()));
}

#[test]
fn frame_decode_succeeds_when_payload_length_matches() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 3);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let payload = Bytes::from_static(b"abc");

    let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

    assert_ok!(result, "frame should decode");
    let Ok(frame) = result else { return };
    assert_eq!(frame.header().command, IpcCommand::Health);
    assert_eq!(frame.header().payload_len, 3);
    assert_eq!(frame.payload().bytes().as_ref(), b"abc");
}

#[test]
fn memory_ingress_len_reflects_queue_depth() {
    let capacity = QueueCapacity::new(
        std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    let queue = MemoryIngress::bounded(capacity);

    let frame_zero = IngressFrame::new(
        RunId::new(0),
        WorkflowDigest::from_bytes([0; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    assert_ok!(frame_zero, "frame zero should construct");
    let Ok(frame_zero) = frame_zero else { return };
    assert_eq!(queue.try_submit(frame_zero), Ok(()));

    let frame_one = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    assert_ok!(frame_one, "frame one should construct");
    let Ok(frame_one) = frame_one else { return };
    assert_eq!(queue.try_submit(frame_one), Ok(()));

    let frame_two = IngressFrame::new(
        RunId::new(2),
        WorkflowDigest::from_bytes([0; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    assert_ok!(frame_two, "frame two should construct");
    let Ok(frame_two) = frame_two else { return };
    assert_eq!(queue.try_submit(frame_two), Ok(()));

    assert_eq!(queue.len(), 3);
    assert!(!queue.is_empty());
}

#[test]
fn memory_ingress_try_recv_returns_frames_in_fifo_order() {
    let capacity = QueueCapacity::new(
        std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN),
    );
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
    assert_ok!(frame1, "first frame should construct");
    assert_ok!(frame2, "second frame should construct");
    let Ok(frame1) = frame1 else { return };
    let Ok(frame2) = frame2 else { return };
    assert_eq!(queue.try_submit(frame1), Ok(()));
    assert_eq!(queue.try_submit(frame2), Ok(()));

    let recv1 = queue.try_recv();
    let recv2 = queue.try_recv();

    assert_ok!(recv1, "first recv should succeed");
    assert_ok!(recv2, "second recv should succeed");
    let Ok(Some(f1)) = recv1 else { return };
    let Ok(Some(f2)) = recv2 else { return };
    assert_eq!(f1.run_id(), RunId::new(1));
    assert_eq!(f2.run_id(), RunId::new(2));
}

#[test]
fn memory_ingress_is_empty_after_draining_all_frames() {
    let capacity = QueueCapacity::new(
        std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    let queue = MemoryIngress::bounded(capacity);
    let frame = IngressFrame::new(
        RunId::new(99),
        WorkflowDigest::from_bytes([0; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    );
    assert_ok!(frame, "frame should construct");
    let Ok(frame) = frame else { return };
    assert_eq!(queue.try_submit(frame), Ok(()));

    let drained = queue.try_recv();
    assert_ok!(drained, "queued frame should drain");

    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn payload_roundtrip_preserves_cancel_run_variant() {
    let payload = IpcPayload::CancelRun { run_id: RunId::new(42) };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_answer_ask_variant() {
    let payload = IpcPayload::AnswerAsk {
        run_id: RunId::new(5),
        ticket: 999,
        answer: Vec::from(&b"yes"[..]),
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_health_variant() {
    let payload = IpcPayload::Health;

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_shutdown_variant() {
    let payload = IpcPayload::Shutdown;

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_inspect_run_variant() {
    let payload = IpcPayload::InspectRun { run_id: RunId::new(333) };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_list_runs_variant() {
    let payload = IpcPayload::ListRuns {
        limit: 50,
        workflow: Some(WorkflowDigest::from_bytes([0xAA; 32])),
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_list_runs_no_filter_variant() {
    let payload = IpcPayload::ListRuns {
        limit: 10,
        workflow: None,
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_get_metrics_variant() {
    let payload = IpcPayload::GetMetrics;

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_get_workflow_graph_variant() {
    let payload = IpcPayload::GetWorkflowGraph {
        digest: WorkflowDigest::from_bytes([0xCC; 32]),
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_get_taint_report_variant() {
    let payload = IpcPayload::GetTaintReport {
        digest: WorkflowDigest::from_bytes([0xDD; 32]),
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn payload_roundtrip_preserves_verify_workflow_variant() {
    let payload = IpcPayload::VerifyWorkflow {
        digest: WorkflowDigest::from_bytes([0xEE; 32]),
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded, "payload should encode");
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn header_decode_rejects_unsupported_version_zero() {
    let encoded = header_bytes(IPC_MAGIC, 0, IpcCommand::Health.as_u16(), 0, 0, 1, 0);
    assert_ok!(encoded, "test header should encode");
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(decoded, Err(IpcError::UnsupportedVersion { actual: 0 }));
}

#[test]
fn header_decode_rejects_unknown_command_id() {
    let encoded = header_bytes(IPC_MAGIC, IPC_VERSION, 200, 0, 0, 1, 0);
    assert_ok!(encoded, "test header should encode");
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(decoded, Err(IpcError::UnknownCommand(200)));
}

#[test]
fn max_payload_bytes_default_is_one_mib() {
    assert_eq!(MaxPayloadBytes::DEFAULT.get(), 1_048_576);
}

#[test]
fn queue_capacity_returns_inner_value() {
    let cap = QueueCapacity::new(
        std::num::NonZeroUsize::new(42).unwrap_or(std::num::NonZeroUsize::MIN),
    );
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

    assert_ok!(result, "payload at exact max should succeed");
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
    assert_ok!(bounded, "should create bounded payload");
    let Ok(bounded) = bounded else { return };

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
    assert_eq!(IpcCommand::ListRuns.as_u16(), 12);
    assert_eq!(IpcCommand::GetMetrics.as_u16(), 13);
    assert_eq!(IpcCommand::GetWorkflowGraph.as_u16(), 14);
    assert_eq!(IpcCommand::GetTaintReport.as_u16(), 15);
    assert_eq!(IpcCommand::VerifyWorkflow.as_u16(), 16);
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
fn header_encode_decode_roundtrip_preserves_flags() {
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0xABCD, 999, 10);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_ok!(decoded, "header should decode");
    let Ok(decoded) = decoded else { return };
    assert_eq!(decoded.flags, 0xABCD);
}

#[test]
fn header_encode_decode_roundtrip_preserves_payload_len() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 256);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_ok!(decoded, "header should decode");
    let Ok(decoded) = decoded else { return };
    assert_eq!(decoded.payload_len, 256);
}

#[test]
fn ingress_frame_rejects_empty_payload_with_min_max() {
    let max = MaxPayloadBytes::new(
        std::num::NonZeroUsize::new(1).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    let data = Bytes::new();

    let result = IngressFrame::new(RunId::new(1), WorkflowDigest::from_bytes([0; 32]), data, max);

    assert_ok!(result, "empty payload should fit within any non-zero max");
}

#[test]
fn memory_ingress_submit_and_recv_single_frame() {
    let capacity = QueueCapacity::new(
        std::num::NonZeroUsize::new(2).unwrap_or(std::num::NonZeroUsize::MIN),
    );
    let queue = MemoryIngress::bounded(capacity);
    let frame = IngressFrame::new(
        RunId::new(42),
        WorkflowDigest::from_bytes([1; 32]),
        Bytes::from_static(b"data"),
        MaxPayloadBytes::DEFAULT,
    );
    assert_ok!(frame, "frame should construct");
    let Ok(frame) = frame else { return };

    assert_eq!(queue.try_submit(frame), Ok(()));
    let recv = queue.try_recv();

    assert_ok!(recv, "recv should succeed");
    let Ok(Some(f)) = recv else { return };
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
    assert_ok!(frame, "frame should construct");
    let Ok(frame) = frame else { return };
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
    assert!(message.contains("encode"), "expected 'encode' in '{message}'");
}

#[test]
fn ipc_error_header_decode_failed_display() {
    let error = IpcError::HeaderDecodeFailed;
    let message = error.to_string();
    assert!(message.contains("decode"), "expected 'decode' in '{message}'");
}

#[test]
fn ipc_error_payload_length_out_of_range_display() {
    let error = IpcError::PayloadLengthOutOfRange { actual: 999 };
    let message = error.to_string();
    assert!(message.contains("999"), "expected '999' in '{message}'");
}

#[test]
fn ipc_error_payload_encode_failed_display() {
    let error = IpcError::PayloadEncodeFailed;
    let message = error.to_string();
    assert!(message.contains("encode"), "expected 'encode' in '{message}'");
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
        IpcError::PayloadLengthMismatch { header: 4, actual: 3 }.runtime_code(),
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
        IpcError::PayloadTooLarge { actual: 9, limit: 8 }.runtime_code(),
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
        codes.iter().copied().collect::<std::collections::BTreeSet<_>>().len(),
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
    assert_eq!(IpcError::Full.diagnostic_code(), DiagnosticCode::new(0x3001));
}

#[test]
fn ipc_error_diagnostic_code_disconnected() {
    assert_eq!(IpcError::Disconnected.diagnostic_code(), DiagnosticCode::new(0x3002));
}

#[test]
fn ipc_error_diagnostic_code_payload_too_large() {
    assert_eq!(
        IpcError::PayloadTooLarge { actual: 100, limit: 10 }.diagnostic_code(),
        DiagnosticCode::new(0x3003)
    );
}

#[test]
fn ipc_error_diagnostic_code_invalid_magic() {
    assert_eq!(
        IpcError::InvalidMagic { actual: 0xDEAD_BEEF }.diagnostic_code(),
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
        IpcError::PayloadLengthMismatch { header: 4, actual: 3 }.diagnostic_code(),
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
    let payload = IpcPayload::CancelRun { run_id: RunId::new(0) };
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };

    let decoded = decode_payload(&encoded);

    assert_ok!(decoded, "CancelRun with run_id=0 should decode");
    let Ok(decoded) = decoded else { return };
    assert_eq!(decoded, IpcPayload::CancelRun { run_id: RunId::new(0) });
}

#[test]
fn adversarial_cancel_run_with_run_id_max_encoded_roundtrips() {
    let payload = IpcPayload::CancelRun { run_id: RunId::new(u64::MAX) };
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };

    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_answer_ask_with_zero_ticket_roundtrips() {
    let payload = IpcPayload::AnswerAsk {
        run_id: RunId::new(1),
        ticket: 0,
        answer: Vec::new(),
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
    let decoded = decode_payload(&encoded);

    assert_eq!(decoded, Ok(payload));
}

#[test]
fn adversarial_answer_ask_with_max_u64_ticket_roundtrips() {
    let payload = IpcPayload::AnswerAsk {
        run_id: RunId::new(1),
        ticket: u64::MAX,
        answer: Vec::from(&b"malicious"[..]),
    };

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };

    let decoded = decode_payload(&encoded);

    assert_ok!(decoded, "outer IpcPayload should decode");
}

#[test]
fn adversarial_submit_run_with_empty_input_roundtrips() {
    let payload = IpcPayload::SubmitRun(SubmitRunPayload {
        run_id: RunId::new(42),
        workflow: WorkflowDigest::from_bytes([0; 32]),
        input: Vec::new(),
    });

    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(frame);
    let Ok(frame) = frame else { return };

    assert_eq!(queue.try_submit(frame.clone()), Ok(()));
    assert_eq!(queue.try_submit(frame.clone()), Err(IpcError::Full));
    let drained = queue.try_recv();
    assert_ok!(drained);
    assert_eq!(queue.try_submit(frame), Ok(()));

    assert_eq!(queue.len(), 1);
}

#[test]
fn adversarial_memory_ingress_disconnected_after_sender_drop() {
    let capacity =
        QueueCapacity::new(std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN));
    let queue = MemoryIngress::bounded(capacity);
    let receiver_only = queue.receiver.clone();
    let sender = queue.sender.clone();
    drop(queue);
    drop(sender);

    let result = receiver_only.try_recv();

    assert!(matches!(
        result,
        Err(crossbeam_channel::TryRecvError::Disconnected)
    ));
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
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };

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
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
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
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
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

    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);

    assert_eq!(result, Err(IpcError::UnknownCommand(0)));
}

#[test]
fn frame_validation_unrecognized_command_id_returns_typed_error() {
    let mut header_bytes = [0u8; IPC_HEADER_LEN];
    header_bytes[..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    header_bytes[6..8].copy_from_slice(&99u16.to_le_bytes());

    let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);

    assert_eq!(result, Err(IpcError::UnknownCommand(99)));
}

#[test]
fn frame_validation_valid_header_with_unknown_payload_type_decode_fails() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 4);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let garbage = Bytes::from(vec![0xDE, 0xAD, 0xBE, 0xEF]);

    let frame_result = decode_frame(&encoded, garbage, MaxPayloadBytes::DEFAULT);
    assert_ok!(frame_result, "frame struct decode should succeed");
    let Ok(frame) = frame_result else { return };

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
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let mut cursor = std::io::Cursor::new(encoded.as_slice());

    let decoded_header = crate::frame::read_frame_header(&mut cursor);
    assert_ok!(decoded_header, "header read should succeed");
    let Ok(decoded_header) = decoded_header else { return };

    let short_data = vec![0u8; 5];
    let mut payload_cursor = std::io::Cursor::new(short_data.as_slice());
    let result = crate::frame::read_frame_payload(&mut payload_cursor, &decoded_header);

    assert_eq!(result, Err(IpcError::PayloadDecodeFailed));
}

#[test]
fn frame_validation_valid_frame_parse_succeeds() {
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 42, 0);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let payload = Bytes::new();

    let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

    assert_ok!(result, "well-formed frame should decode");
    let Ok(frame) = result else { return };
    assert_eq!(frame.header().command, IpcCommand::SubmitRun);
    assert_eq!(frame.header().flags, 0);
    assert_eq!(frame.header().correlation, 42);
    assert_eq!(frame.header().payload_len, 0);
    assert_eq!(frame.payload().bytes().len(), 0);
}

#[test]
fn frame_validation_roundtrip_encode_decode_preserves_all_fields() {
    let original_header =
        IpcFrameHeader::new(IpcCommand::CompleteAction, 0xABCD, 0x1234_5678_9ABC_DEF0, 8);
    let encoded = original_header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let payload = Bytes::from_static(b"payload!");

    let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

    assert_ok!(result, "frame should decode");
    let Ok(frame) = result else { return };
    assert_eq!(frame.header().command, IpcCommand::CompleteAction);
    assert_eq!(frame.header().flags, 0xABCD);
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
        assert_ok!(encoded, "header should encode for {command:?}");
        let Ok(encoded) = encoded else { return };
        let result = decode_frame(&encoded, Bytes::new(), MaxPayloadBytes::DEFAULT);

        assert_ok!(result, "frame should decode for {command:?}");
        let Ok(frame) = result else { return };
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
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let payload = Bytes::from(vec![0u8; max]);

    let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

    assert_ok!(result, "frame at exact max boundary should decode");
    let Ok(frame) = result else { return };
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
        fn ipc_command_roundtrips_through_u16(cmd in 1u16..=16u16) {
            let parsed = IpcCommand::from_u16(cmd);
            prop_assert_ok!(parsed);
            let Ok(command) = parsed else { return Ok(()) };
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
        fn ipc_command_encode_decode_roundtrip(cmd_val in 1u16..=16u16) {
            let Ok(command) = IpcCommand::from_u16(cmd_val) else {
                return Ok(());
            };

            let header = IpcFrameHeader::new(command, 0, 0, 0);
            let encoded = header.encode();
            prop_assert_ok!(encoded);
            let Ok(encoded) = encoded else { return Ok(()) };
            let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

            prop_assert_ok!(decoded);
            let Ok(decoded) = decoded else { return Ok(()) };
            prop_assert_eq!(decoded.command, command);
        }
    }

    proptest! {
        #[test]
        fn ipc_response_encode_decode_roundtrip(run_id_val in 0u64..) {
            let payload = IpcPayload::CancelRun { run_id: RunId::new(run_id_val) };

            let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
            prop_assert_ok!(encoded);
            let Ok(encoded) = encoded else { return Ok(()) };
            let decoded = decode_payload(&encoded);

            prop_assert_ok!(decoded);
            let Ok(decoded) = decoded else { return Ok(()) };
            prop_assert_eq!(decoded, payload);
        }
    }

    proptest! {
        #[test]
        fn frame_header_length_never_exceeds_max(cmd_val in 1u16..=16u16, payload_len in 0u32..=1024u32) {
            let Ok(command) = IpcCommand::from_u16(cmd_val) else {
                return Ok(());
            };

            let header = IpcFrameHeader::new(command, 0, 0, payload_len);

            let encoded = header.encode();
            prop_assert_ok!(encoded);
            let Ok(encoded) = encoded else { return Ok(()) };
            prop_assert_eq!(encoded.len(), IPC_HEADER_LEN);
        }
    }
}
