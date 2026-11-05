//! Single stable-tag parser contract; retired ordinal/default shapes are refusals.

use super::*;
use crate::types::RecordEnvelope;

enum Rejection {
    Decode(postcard::Error),
    Kind { envelope: u16, payload: u16 },
}

fn assert_rejection(
    result: Result<(RecordEnvelope, JournalEvent), JournalError>,
    expected: &Rejection,
) -> Result<(), JournalError> {
    match (result, expected) {
        (Err(JournalError::PostcardDecodeFailed(actual)), Rejection::Decode(expected)) => {
            assert_eq!(&actual, expected);
            Ok(())
        }
        (
            Err(JournalError::RecordKindPayloadMismatch {
                envelope_kind,
                payload_kind,
            }),
            Rejection::Kind { envelope, payload },
        ) => {
            assert_eq!((envelope_kind, payload_kind), (*envelope, *payload));
            Ok(())
        }
        (Err(error), _) => Err(error),
        (Ok(_), _) => Err(JournalError::InvalidEvent),
    }
}

fn checked_envelope(
    bytes: &[u8],
    payload: &[u8],
    kind: RecordKind,
    seq: EventSeq,
) -> Result<RecordEnvelope, JournalError> {
    let (envelope, actual) = crate::codec::payload::decode_record_payload(
        bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(actual, payload);
    assert_eq!(envelope.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(envelope.record_kind, kind.id());
    assert_eq!(envelope.sequence, seq.get());
    Ok(envelope)
}

fn assert_payload_rejected(
    payload: &[u8],
    kind: RecordKind,
    seq: EventSeq,
    expected: &Rejection,
) -> Result<(), JournalError> {
    let bytes = encode_schema_one_payload_bytes(payload, kind, seq)?;
    checked_envelope(&bytes, payload, kind, seq)?;
    assert_rejection(
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES),
        expected,
    )?;
    assert_rejection(
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES),
        expected,
    )
}

fn assert_ordinal_rejected(
    payload: &[u8],
    kind: RecordKind,
    seq: EventSeq,
) -> Result<(), JournalError> {
    // The independent stable parser pins the error propagated by the wrapper.
    // Do not accept a matching stable event as an expected refusal.
    let expected = match postcard::take_from_bytes::<JournalEvent>(payload) {
        Err(error) => Rejection::Decode(error),
        Ok((event, _)) => {
            assert_ne!(event.record_kind(), kind);
            Rejection::Kind {
                envelope: kind.id(),
                payload: event.record_kind().id(),
            }
        }
    };
    assert_payload_rejected(payload, kind, seq, &expected)
}

fn current_roundtrip(event: &JournalEvent) -> Result<Vec<u8>, JournalError> {
    assert!(event.is_valid());
    let kind = event.record_kind();
    let payload = postcard::to_allocvec(event).map_err(JournalError::Encode)?;
    assert_eq!(payload.first().copied(), u8::try_from(kind.id()).ok());
    let (direct, remainder) = postcard::take_from_bytes::<JournalEvent>(&payload)
        .map_err(JournalError::PostcardDecodeFailed)?;
    assert_eq!(direct, *event);
    assert!(remainder.is_empty());
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        kind,
        event.seq().get(),
        event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    checked_envelope(&bytes, &payload, kind, event.seq())?;
    let (_, generic) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, journal) =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    assert_eq!(generic, *event);
    assert_eq!(journal, *event);
    Ok(payload)
}

fn assert_current_trailing_rejected(event: &JournalEvent) -> Result<(), JournalError> {
    let mut payload = current_roundtrip(event)?;
    payload.push(0);
    let (decoded, remainder) = postcard::take_from_bytes::<JournalEvent>(&payload)
        .map_err(JournalError::PostcardDecodeFailed)?;
    assert_eq!(decoded, *event);
    assert_eq!(remainder, &[0]);
    assert_payload_rejected(
        &payload,
        event.record_kind(),
        event.seq(),
        &Rejection::Decode(postcard::Error::DeserializeBadEncoding),
    )
}

#[test]
fn ordinal_payloads_reject_with_unchanged_envelope_kinds() -> Result<(), JournalError> {
    let mut checked = 0usize;
    for case in schema_one_fixture_cases()? {
        if case.envelope_kind == case.expected.record_kind() {
            let payload = schema_one_fixture_payload(&case.payload)?;
            let bytes = encode_schema_one_fixture_record(
                &case.payload,
                case.envelope_kind,
                case.expected.seq(),
            )?;
            checked_envelope(&bytes, &payload, case.envelope_kind, case.expected.seq())?;
            assert_ordinal_rejected(&payload, case.envelope_kind, case.expected.seq())?;
            checked = checked.checked_add(1).ok_or(JournalError::InvalidEvent)?;
        }
    }
    assert_eq!(checked, 21);
    Ok(())
}

#[test]
fn ordinal_payloads_reject_with_former_shared_envelopes() -> Result<(), JournalError> {
    let mut checked = 0usize;
    for case in schema_one_fixture_cases()? {
        if case.envelope_kind != case.expected.record_kind() {
            let payload = schema_one_fixture_payload(&case.payload)?;
            let bytes = encode_schema_one_fixture_record(
                &case.payload,
                case.envelope_kind,
                case.expected.seq(),
            )?;
            checked_envelope(&bytes, &payload, case.envelope_kind, case.expected.seq())?;
            assert_ordinal_rejected(&payload, case.envelope_kind, case.expected.seq())?;
            checked = checked.checked_add(1).ok_or(JournalError::InvalidEvent)?;
        }
    }
    assert_eq!(checked, 3);
    Ok(())
}

#[test]
fn missing_default_field_ordinal_payloads_are_not_retried() -> Result<(), JournalError> {
    let cases = schema_one_missing_field_cases()?;
    assert_eq!(cases.len(), 3);
    for case in cases {
        let payload = schema_one_fixture_payload(&case.payload)?;
        assert_ordinal_rejected(&payload, case.envelope_kind, case.expected.seq())?;
    }
    Ok(())
}

#[test]
fn current_run_accepted_rejects_trailing_bytes_after_successful_parse() -> Result<(), JournalError>
{
    assert_current_trailing_rejected(&JournalEvent::RunAccepted {
        run: RunId::new(901),
        seq: EventSeq::new(1),
        workflow: codec_sample_digest(0x91),
    })
}

#[test]
fn current_tags_33_34_35_roundtrip_then_reject_trailing_bytes() -> Result<(), JournalError> {
    let mut tags = Vec::new();
    for case in schema_one_fixture_cases()? {
        if case.envelope_kind != case.expected.record_kind() {
            tags.push(case.expected.record_kind().id());
            assert_current_trailing_rejected(&case.expected)?;
        }
    }
    tags.sort_unstable();
    assert_eq!(tags, vec![33, 34, 35]);
    Ok(())
}

#[test]
fn explicit_current_fields_roundtrip_then_reject_trailing_bytes() -> Result<(), JournalError> {
    for case in schema_one_missing_field_cases()? {
        assert_current_trailing_rejected(&case.expected)?;
    }
    Ok(())
}

#[test]
fn complete_ordinal_run_accepted_pins_unknown_tag_error() -> Result<(), JournalError> {
    let seq = EventSeq::new(1);
    let payload = schema_one_fixture_payload(&SchemaOneJournalEventFixture::RunAccepted {
        run: RunId::new(901),
        seq,
        workflow: codec_sample_digest(0x91),
    })?;
    assert_eq!(payload.first(), Some(&0));
    assert_eq!(
        postcard::take_from_bytes::<JournalEvent>(&payload),
        Err(postcard::Error::SerdeDeCustom)
    );
    assert_payload_rejected(
        &payload,
        RecordKind::RunAccepted,
        seq,
        &Rejection::Decode(postcard::Error::SerdeDeCustom),
    )
}

#[test]
fn stable_tags_with_missing_required_fields_pin_eof_not_defaults() -> Result<(), JournalError> {
    for case in schema_one_missing_field_cases()? {
        current_roundtrip(&case.expected)?;
        let mut payload = schema_one_fixture_payload(&case.payload)?;
        let tag = u8::try_from(case.expected.record_kind().id())
            .map_err(|_| JournalError::InvalidEvent)?;
        *payload.first_mut().ok_or(JournalError::UnexpectedEof)? = tag;
        assert_eq!(
            postcard::take_from_bytes::<JournalEvent>(&payload),
            Err(postcard::Error::DeserializeUnexpectedEnd),
            "{}",
            case.name
        );
        assert_payload_rejected(
            &payload,
            case.expected.record_kind(),
            case.expected.seq(),
            &Rejection::Decode(postcard::Error::DeserializeUnexpectedEnd),
        )?;
    }
    Ok(())
}

#[test]
fn current_tag_mismatch_pins_both_record_kinds() -> Result<(), JournalError> {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(7),
        seq: EventSeq::new(9),
        step: StepIdx::new(3),
        output: SlotIdx::new(0),
    };
    let payload = current_roundtrip(&event)?;
    assert_eq!(payload.first(), Some(&33));
    let bytes = encode_schema_one_payload_bytes(&payload, event.record_kind(), event.seq())?;
    let mut envelope = checked_envelope(&bytes, &payload, event.record_kind(), event.seq())?;
    <JournalEvent as crate::codec::EnforceKindParity>::enforce_kind_parity(&envelope, &event)?;
    envelope.record_kind = RecordKind::SlotWritten.id();
    assert!(matches!(
        <JournalEvent as crate::codec::EnforceKindParity>::enforce_kind_parity(&envelope, &event),
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: 12,
            payload_kind: 33,
        })
    ));
    assert_payload_rejected(
        &payload,
        RecordKind::SlotWritten,
        event.seq(),
        &Rejection::Kind {
            envelope: 12,
            payload: 33,
        },
    )
}

fn current_ticket_event() -> JournalEvent {
    let run = RunId::new(7);
    JournalEvent::ActionScheduledTicket {
        run,
        seq: EventSeq::new(9),
        ticket: codec_action_ticket(run, StepIdx::new(3), ActionId::new(5)),
        input: SlotIdx::new(0),
        output: SlotIdx::new(1),
        action_abi_digest: codec_sample_digest(0xA1),
    }
}

#[test]
fn current_ticket_keeps_crc_digest_and_size_checks() -> Result<(), JournalError> {
    let event = current_ticket_event();
    let payload = current_roundtrip(&event)?;
    let bytes = encode_schema_one_payload_bytes(&payload, event.record_kind(), event.seq())?;
    let mut bad_crc = bytes.clone();
    *bad_crc
        .get_mut(CRC_OFFSET)
        .ok_or(JournalError::UnexpectedEof)? ^= 1;
    assert!(matches!(
        decode_journal_event(
            &bad_crc,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES
        ),
        Err(JournalError::HeaderChecksumMismatch)
    ));
    let mut bad_digest = bytes.clone();
    *bad_digest
        .get_mut(RECORD_HEADER_BYTES)
        .ok_or(JournalError::UnexpectedEof)? ^= 1;
    assert!(matches!(
        decode_journal_event(
            &bad_digest,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES
        ),
        Err(JournalError::PayloadDigestMismatch)
    ));
    let payload_len = u32::try_from(payload.len()).map_err(|_| JournalError::InvalidEvent)?;
    let limit = payload_len
        .checked_sub(1)
        .ok_or(JournalError::InvalidEvent)?;
    assert!(matches!(
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, limit),
        Err(JournalError::PayloadTooLarge { len, max }) if len == payload_len && max == limit
    ));
    let (_, decoded) = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, payload_len)?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn current_ticket_sequence_mismatch_is_not_masked_by_payload_failure() -> Result<(), JournalError> {
    let event = current_ticket_event();
    let payload = current_roundtrip(&event)?;
    let envelope_seq = EventSeq::new(10);
    let bytes = encode_schema_one_payload_bytes(&payload, event.record_kind(), envelope_seq)?;
    checked_envelope(&bytes, &payload, event.record_kind(), envelope_seq)?;
    let (envelope, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.sequence, 10);
    assert_eq!(decoded, event);
    assert!(matches!(
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES),
        Err(JournalError::ReplayEnvelopeSequenceMismatch {
            run,
            envelope_seq: 10,
            payload_seq: 9,
        }) if run == event.run_id()
    ));
    Ok(())
}
