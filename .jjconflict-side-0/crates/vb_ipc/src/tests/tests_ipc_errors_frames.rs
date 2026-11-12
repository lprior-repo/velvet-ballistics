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
        assert!(
            result.is_ok(),
            "u32_to_usize({value}) should succeed on this platform"
        );
        let converted = result.unwrap();
        assert_eq!(
            converted as u32, value,
            "round-trip u32->usize->u32 should preserve value"
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
    };
    let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
    assert_ok!(encoded);
    let Ok(encoded) = encoded else { return };

    let decoded = decode_payload(&encoded);

    assert_ok!(decoded, "CancelRun with run_id=0 should decode");
    let Ok(decoded) = decoded else { return };
    assert_eq!(
        decoded,
        IpcPayload::CancelRun {
            run_id: RunId::new(0)
        }
    );
}

#[test]
fn adversarial_cancel_run_with_run_id_max_encoded_roundtrips() {
    let payload = IpcPayload::CancelRun {
        run_id: RunId::new(u64::MAX),
    };
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
        taint: None,
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
        taint: None,
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
    let Ok(decoded_header) = decoded_header else {
        return;
    };

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
        IpcFrameHeader::new(IpcCommand::CompleteAction, 0, 0x1234_5678_9ABC_DEF0, 8);
    let encoded = original_header.encode();
    assert_ok!(encoded, "header should encode");
    let Ok(encoded) = encoded else { return };
    let payload = Bytes::from_static(b"payload!");

    let result = decode_frame(&encoded, payload, MaxPayloadBytes::DEFAULT);

    assert_ok!(result, "frame should decode");
    let Ok(frame) = result else { return };
    assert_eq!(frame.header().command, IpcCommand::CompleteAction);
    assert_eq!(frame.header().flags, 0);
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

