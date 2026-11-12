fn mismatch_step_started_under_slot_written_envelope()
-> (crate::types::RecordEnvelope, JournalEvent) {
    let event = JournalEvent::StepStarted {
        run: RunId::new(7),
        seq: EventSeq::new(9),
        step: StepIdx::new(3),
        attempt: 1,
    };
    let envelope = crate::types::RecordEnvelope {
        magic: MAGIC_JOURNAL_EVENT,
        schema_version: CURRENT_SCHEMA_VERSION,
        record_kind: RecordKind::SlotWritten.id(),
        sequence: 9,
    };
    (envelope, event)
}

#[test]
fn validate_journal_event_record_kind_rejects_mismatch_and_accepts_match()
-> Result<(), JournalError> {
    let (envelope, event) = mismatch_step_started_under_slot_written_envelope();
    match validate_journal_event_record_kind(&envelope, &event) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: observed_envelope,
            payload_kind: observed_payload,
        }) if observed_envelope == RecordKind::SlotWritten.id()
            && observed_payload == RecordKind::StepStarted.id() => {}
        other => panic!(
            "expected exact RecordKindPayloadMismatch for SlotWritten envelope with StepStarted payload, got {other:?}"
        ),
    }
    let matching = crate::types::RecordEnvelope {
        magic: MAGIC_JOURNAL_EVENT,
        schema_version: CURRENT_SCHEMA_VERSION,
        record_kind: RecordKind::StepStarted.id(),
        sequence: 9,
    };
    match validate_journal_event_record_kind(&matching, &event) {
        Ok(()) => Ok(()),
        other => panic!("expected Ok for matching StepStarted kind pair, got {other:?}"),
    }
}

#[test]
fn journal_event_parity_enforcement_rejects_noncompat_mismatch_with_exact_variant()
-> Result<(), JournalError> {
    let (envelope, event) = mismatch_step_started_under_slot_written_envelope();
    match validate_decoded_journal_event_record_kind(&envelope, &event) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: observed_envelope,
            payload_kind: observed_payload,
        }) if observed_envelope == RecordKind::SlotWritten.id()
            && observed_payload == RecordKind::StepStarted.id() => {}
        other => panic!(
            "expected exact RecordKindPayloadMismatch from decoded-kind validator, got {other:?}"
        ),
    }
    match <JournalEvent as EnforceKindParity>::enforce_kind_parity(&envelope, &event) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: observed_envelope,
            payload_kind: observed_payload,
        }) if observed_envelope == RecordKind::SlotWritten.id()
            && observed_payload == RecordKind::StepStarted.id() =>
        {
            Ok(())
        }
        other => panic!(
            "expected exact RecordKindPayloadMismatch from parity enforcement, got {other:?}"
        ),
    }
}

#[test]
fn schema_one_compat_pair_rejected_by_strict_accepted_by_decoded() -> Result<(), JournalError> {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(7),
        seq: EventSeq::new(9),
        step: StepIdx::new(3),
        output: SlotIdx::new(0),
    };
    let envelope = crate::types::RecordEnvelope {
        magic: MAGIC_JOURNAL_EVENT,
        schema_version: SCHEMA_ONE_VERSION_FOR_TEST,
        record_kind: RecordKind::SlotWritten.id(),
        sequence: 9,
    };
    match validate_journal_event_record_kind(&envelope, &event) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == RecordKind::SlotWritten.id()
            && payload_kind == RecordKind::StepSucceeded.id() => {}
        other => panic!(
            "expected exact RecordKindPayloadMismatch for schema-1 SlotWritten envelope with StepSucceeded payload, got {other:?}"
        ),
    }
    match validate_decoded_journal_event_record_kind(&envelope, &event) {
        Ok(()) => {}
        other => {
            panic!("expected Ok for schema-1 compat pair through decoded validator, got {other:?}")
        }
    }
    match <JournalEvent as EnforceKindParity>::enforce_kind_parity(&envelope, &event) {
        Ok(()) => Ok(()),
        other => {
            panic!("expected Ok for schema-1 compat pair through parity enforcement, got {other:?}")
        }
    }
}

#[test]
fn known_kind_validators_pin_exact_behavior() -> Result<(), JournalError> {
    assert!(
        is_known_record_kind(RecordKind::StepStarted.id()),
        "StepStarted must be a known record kind"
    );
    assert!(
        !is_known_record_kind(999),
        "999 must not be a known record kind"
    );
    match validate_known_record_kind(RecordKind::StepStarted.id()) {
        Ok(()) => {}
        other => panic!("expected Ok for known StepStarted kind, got {other:?}"),
    }
    match validate_known_record_kind(999) {
        Err(JournalError::UnknownRecordKind { kind }) if kind == 999 => Ok(()),
        other => panic!("expected exact UnknownRecordKind for 999, got {other:?}"),
    }
}

#[test]
fn newest_record_kinds_decode_as_known() -> Result<(), JournalError> {
    assert_eq!(
        RecordKind::from_id(42),
        Some(RecordKind::RunOwnedInputs),
        "kind 42 must decode to RunOwnedInputs"
    );
    assert_eq!(
        RecordKind::from_id(43),
        Some(RecordKind::RunContinuation),
        "kind 43 must decode to RunContinuation"
    );
    assert!(
        is_known_record_kind(42),
        "kind 42 must be a known record kind"
    );
    assert!(
        is_known_record_kind(43),
        "kind 43 must be a known record kind"
    );
    match validate_known_record_kind(42) {
        Ok(()) => {}
        other => panic!("expected Ok for known kind 42, got {other:?}"),
    }
    match validate_known_record_kind(43) {
        Ok(()) => Ok(()),
        other => panic!("expected Ok for known kind 43, got {other:?}"),
    }
}

#[test]
fn header_admits_payload_length_at_exact_maximum() -> Result<(), JournalError> {
    let payload = b"exact-maximum";
    let header = encode_record_header(
        MAGIC_JOURNAL_EVENT,
        RecordKind::StepStarted,
        9,
        payload,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let max = u32::try_from(payload.len()).map_err(|_| JournalError::InvalidEvent)?;
    let decoded = decode_record_header(&header, MAGIC_JOURNAL_EVENT, max)?;
    assert_eq!(
        decoded.payload_len, max,
        "payload length at the exact maximum must be admitted"
    );
    Ok(())
}
