#[test]
fn encode_decode_roundtrip_journal_event_run_accepted() -> Result<(), JournalError> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(42),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded_event) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(envelope.record_kind, RecordKind::RunAccepted.id());
    assert_eq!(envelope.sequence, 0);
    assert_eq!(decoded_event, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_tenant_admission() -> Result<(), JournalError> {
    let event = JournalEvent::TenantAdmissionEvent {
        run: RunId::new(42),
        seq: EventSeq::new(3),
        tenant: vb_core::ids::TenantId::new(7),
        cost_units: 10,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::TenantAdmission,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded_event) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(envelope.record_kind, RecordKind::TenantAdmission.id());
    assert_eq!(envelope.sequence, 3);
    assert_eq!(decoded_event, event);
    assert_eq!(decoded_event.record_kind(), RecordKind::TenantAdmission);
    assert!(decoded_event.is_valid());
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_step_started() -> Result<(), JournalError> {
    let event = JournalEvent::StepStarted {
        run: RunId::new(100),
        seq: EventSeq::new(1),
        step: StepIdx::new(5),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::StepStarted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_run_finished() -> Result<(), JournalError> {
    let event = JournalEvent::RunFinished {
        run: RunId::new(7),
        seq: EventSeq::new(99),
        result: SlotIdx::new(3),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunFinished,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_slot_written_with_value() -> Result<(), JournalError> {
    let slot_bytes =
        postcard::to_allocvec(&vb_core::SlotValue::Bool(true)).map_err(JournalError::Encode)?;
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(10),
        seq: EventSeq::new(3),
        slot: SlotIdx::new(0),
        value: Some(slot_bytes),
        extra: None,
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::SlotWritten,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_run_cancelled() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(55),
        seq: EventSeq::new(2),
        attempt: 1,
        reason: None,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_run_cancelled_with_reason() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(56),
        seq: EventSeq::new(3),
        attempt: 1,
        reason: Some("user request".to_string()),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_action_failed() -> Result<(), JournalError> {
    let event = JournalEvent::ActionFailedEvent {
        run: RunId::new(200),
        seq: EventSeq::new(3),
        step: StepIdx::new(2),
        action: vb_core::ActionId::new(99),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::ActionFailed,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn decode_journal_event_accepts_exact_ask_timed_out_kind() -> Result<(), JournalError> {
    let event = JournalEvent::AskTimedOutEvent {
        run: RunId::new(73),
        seq: EventSeq::new(8),
        step: StepIdx::new(3),
        attempt: 2,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskTimedOut,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    if decoded == event {
        Ok(())
    } else {
        Err(JournalError::InvalidEvent)
    }
}

#[test]
fn decode_journal_event_rejects_ask_timed_out_under_ask_answered_kind() -> Result<(), JournalError>
{
    let event = JournalEvent::AskTimedOutEvent {
        run: RunId::new(74),
        seq: EventSeq::new(9),
        step: StepIdx::new(4),
        attempt: 3,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskAnswered,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    match decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == RecordKind::AskAnswered.id()
            && payload_kind == RecordKind::AskTimedOut.id() =>
        {
            Ok(())
        }
        Ok(_) | Err(_) => Err(JournalError::InvalidEvent),
    }
}

// ============================================================================
// Adversarial parity tests: AskTimedOut(29) envelope with AskAnswered(18) payload.
// The parity gate must reject this combination at every public decode site —
// `decode_record::<JournalEvent>`, `decode_journal_event`, `parse_event`, and
// the replay path. These tests exercise the round trip through the generic
// `decode_record` entry point (which used to bypass parity) to prove that
// parity is now mandatory for JournalEvent decoding.
// ============================================================================

/// `decode_record::<JournalEvent>` is the public generic decode entry point.
/// Before the parity gate was mandatory, this function silently returned the
/// decoded payload even when the envelope kind disagreed with the payload
/// variant. The contract fix moves the parity check into
/// `EnforceKindParity::enforce_kind_parity`, so `decode_record::<JournalEvent>`
/// must now reject envelope kind 18 (AskAnswered) paired with an
/// `AskTimedOutEvent` payload.
#[test]
fn decode_record_journal_event_rejects_ask_answered_envelope_with_ask_timed_out_payload()
-> Result<(), JournalError> {
    let event = JournalEvent::AskTimedOutEvent {
        run: RunId::new(91),
        seq: EventSeq::new(11),
        step: StepIdx::new(5),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskAnswered,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    match decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == RecordKind::AskAnswered.id()
            && payload_kind == RecordKind::AskTimedOut.id() =>
        {
            Ok(())
        }
        other => panic!(
            "decode_record::<JournalEvent> must reject AskAnswered(18) envelope \
             carrying AskTimedOut payload, got {other:?}"
        ),
    }
}

/// Symmetric inverse: AskTimedOut(29) envelope carrying an `AskAnsweredEvent`
/// payload must also be rejected with `RecordKindPayloadMismatch` that names
/// both the envelope and payload kinds. This guards against the production
/// replay path accepting a timeout event whose payload is actually an answer.
#[test]
fn decode_record_journal_event_rejects_ask_timed_out_envelope_with_ask_answered_payload()
-> Result<(), JournalError> {
    let event = JournalEvent::AskAnsweredEvent {
        run: RunId::new(92),
        seq: EventSeq::new(12),
        step: StepIdx::new(6),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskTimedOut,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    match decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == RecordKind::AskTimedOut.id()
            && payload_kind == RecordKind::AskAnswered.id() =>
        {
            Ok(())
        }
        other => panic!(
            "decode_record::<JournalEvent> must reject AskTimedOut(29) envelope \
             carrying AskAnswered payload, got {other:?}"
        ),
    }
}

/// `decode_journal_event` is the explicit journal-event decode function. It
/// must reject the same envelope/payload swap with the same diagnostic, but
/// the test lives here to confirm parity is enforced even on the dedicated
/// entry point (not just the generic `decode_record`).
#[test]
fn decode_journal_event_rejects_ask_timed_out_envelope_with_ask_answered_payload()
-> Result<(), JournalError> {
    let event = JournalEvent::AskAnsweredEvent {
        run: RunId::new(93),
        seq: EventSeq::new(13),
        step: StepIdx::new(7),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskTimedOut,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    match decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == RecordKind::AskTimedOut.id()
            && payload_kind == RecordKind::AskAnswered.id() =>
        {
            Ok(())
        }
        other => panic!(
            "decode_journal_event must reject AskTimedOut(29) envelope \
             carrying AskAnswered payload, got {other:?}"
        ),
    }
}

/// When the envelope and payload kinds agree, parity is satisfied and the
/// decode must succeed. This is the positive control for the AskTimedOut
/// parity gate — without it the rejection tests above could trivially pass
/// by rejecting everything.
#[test]
fn decode_record_journal_event_accepts_ask_timed_out_with_matching_kind() -> Result<(), JournalError>
{
    let event = JournalEvent::AskTimedOutEvent {
        run: RunId::new(94),
        seq: EventSeq::new(14),
        step: StepIdx::new(8),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskTimedOut,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.record_kind, RecordKind::AskTimedOut.id());
    assert_eq!(decoded, event);
    Ok(())
}
