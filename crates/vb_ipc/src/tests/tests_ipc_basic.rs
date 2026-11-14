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
    assert_eq!(IpcCommand::from_u16(12), Ok(IpcCommand::UnknownCommand(12)));
    assert_eq!(IpcCommand::from_u16(13), Ok(IpcCommand::UnknownCommand(13)));
    assert_eq!(IpcCommand::from_u16(14), Ok(IpcCommand::UnknownCommand(14)));
    assert_eq!(IpcCommand::from_u16(15), Ok(IpcCommand::UnknownCommand(15)));
    assert_eq!(IpcCommand::from_u16(16), Ok(IpcCommand::UnknownCommand(16)));
    assert_eq!(IpcCommand::from_u16(17), Ok(IpcCommand::UnknownCommand(17)));
    assert_eq!(
        IpcCommand::from_u16(18),
        Ok(IpcCommand::SubmitBoundGateDecision)
    );
}

#[test]
fn header_roundtrips_little_endian_fields() {
    let header = IpcFrameHeader::new(IpcCommand::CompleteAction, 0, 42, 3);
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
    assert_ok!(encoded, "payload should encode under default bound");
    let Ok(encoded) = encoded else { return };

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
    let capacity =
        QueueCapacity::new(std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN));
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
    let capacity =
        QueueCapacity::new(std::num::NonZeroUsize::new(4).unwrap_or(std::num::NonZeroUsize::MIN));
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
    let payload = IpcPayload::CancelRun {
        run_id: RunId::new(42),
    };

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
        taint: None,
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
    let payload = IpcPayload::InspectRun {
        run_id: RunId::new(333),
    };

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
fn header_decode_rejects_unsupported_version_zero() {
    let encoded = header_bytes(IPC_MAGIC, 0, IpcCommand::Health.as_u16(), 0, 0, 1, 0);
    assert_ok!(encoded, "test header should encode");
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_eq!(decoded, Err(IpcError::UnsupportedVersion { actual: 0 }));
}

#[test]
fn header_decode_accepts_unknown_command_id() {
    let encoded = header_bytes(IPC_MAGIC, IPC_VERSION, 200, 0, 0, 1, 0);
    assert_ok!(encoded, "test header should encode");
    let Ok(encoded) = encoded else { return };

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
fn ut_header_reserved_field_is_zero() {
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 1, 0);
    let encoded = header.encode().expect("header must encode");
    let reserved_bytes: [u8; 2] = [encoded[10], encoded[11]];
    assert_eq!(
        reserved_bytes,
        [0x00, 0x00],
        "reserved bytes at offset 10-11 must be zero"
    );
}

#[test]
fn header_encode_decode_roundtrip_preserves_flags() {
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0x00CD, 999, 10);
    let encoded = header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

    assert_ok!(decoded, "header should decode");
    let Ok(decoded) = decoded else { return };
    assert_eq!(decoded.flags, 0x00CD);
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
    let max =
        MaxPayloadBytes::new(std::num::NonZeroUsize::new(1).unwrap_or(std::num::NonZeroUsize::MIN));
    let data = Bytes::new();

    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0; 32]),
        data,
        max,
    );

    assert_ok!(result, "empty payload should fit within any non-zero max");
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
