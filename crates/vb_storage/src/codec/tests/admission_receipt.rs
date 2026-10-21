use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn admission(binding: crate::RunInputBinding) -> JournalEvent {
    JournalEvent::RunAdmission {
        run: RunId::new(7),
        seq: EventSeq::new(1),
        artifact_digest: codec_sample_digest(1),
        input_digest: binding,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Strict,
    }
}

#[test]
fn bound_and_historical_admission_receipts_have_distinct_stable_wire_tags() -> TestResult {
    for (binding, tag) in [
        (crate::RunInputBinding::LegacyUnbound, 24),
        (crate::RunInputBinding::Digest(codec_sample_digest(2)), 36),
    ] {
        let event = admission(binding);
        let payload = postcard::to_allocvec(&event)?;
        assert_eq!(payload.first(), Some(&tag));
        assert_eq!(event.record_kind().id(), u16::from(tag));
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            1,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) =
            decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(decoded, event);
        let mut trailing = bytes;
        trailing.push(0);
        assert!(matches!(
            decode_journal_event(
                &trailing,
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES
            ),
            Err(JournalError::PostcardDecodeFailed(
                postcard::Error::DeserializeBadEncoding
            ))
        ));
    }
    Ok(())
}

#[test]
fn schema_one_ordinal_admission_decodes_explicitly_unbound() -> TestResult {
    // Postcard enum ordinal 1 followed by the historical five-field payload.
    let ordinal_payload = (
        1_u8,
        RunId::new(7),
        EventSeq::new(1),
        codec_sample_digest(1),
        CapabilitySet::empty(),
        RuntimePolicy::Strict,
    );
    let event = admission(crate::RunInputBinding::LegacyUnbound);
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAdmission,
        1,
        &ordinal_payload,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    rewrite_schema_version_for_test(&mut bytes, SCHEMA_ONE_VERSION_FOR_TEST)?;
    let (_, decoded) =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn bound_admission_cannot_be_labeled_as_historical_envelope() -> TestResult {
    let event = admission(crate::RunInputBinding::Digest(codec_sample_digest(2)));
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAdmission,
        1,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert!(matches!(
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES),
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: 24,
            payload_kind: 36
        })
    ));
    Ok(())
}
