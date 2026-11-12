#[test]
fn schema_one_fixture_payloads_decode_for_unchanged_envelope_kinds() -> Result<(), JournalError> {
    let cases = schema_one_fixture_cases()?;
    let mut checked = 0usize;
    for (i, case) in cases.iter().enumerate() {
        if case.envelope_kind != case.expected.record_kind() {
            continue;
        }
        let bytes = encode_schema_one_fixture_record(
            &case.payload,
            case.envelope_kind,
            case.expected.seq(),
        )?;
        let (envelope, decoded) = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_eq!(
            envelope.schema_version, SCHEMA_ONE_VERSION_FOR_TEST,
            "schema-one fixture compatibility must be explicit schema 1 at index {i}"
        );
        assert_eq!(envelope.record_kind, case.envelope_kind.id());
        assert_eq!(
            decoded, case.expected,
            "schema-one fixture decode mismatch at index {i}"
        );
        checked = checked.saturating_add(1);
    }
    assert_eq!(
        checked, 21,
        "schema-one ordinal fixture decode must cover unchanged envelope kinds only"
    );
    Ok(())
}

#[test]
fn schema_one_fixture_shared_envelope_records_decode_through_compatibility_path()
-> Result<(), JournalError> {
    let cases = schema_one_fixture_cases()?;
    let mut checked = 0usize;
    for (i, case) in cases.iter().enumerate() {
        if case.envelope_kind == case.expected.record_kind() {
            continue;
        }
        let bytes = encode_schema_one_fixture_record(
            &case.payload,
            case.envelope_kind,
            case.expected.seq(),
        )?;
        let (envelope, decoded) =
            decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(
            envelope.schema_version, SCHEMA_ONE_VERSION_FOR_TEST,
            "shared-envelope fixture path must be explicit schema 1 at index {i}"
        );
        assert_eq!(
            envelope.record_kind,
            case.envelope_kind.id(),
            "old envelope kind mismatch at index {i}"
        );
        assert_eq!(
            decoded, case.expected,
            "shared-envelope fixture decode mismatch at index {i}"
        );
        checked = checked.saturating_add(1);
    }
    assert_eq!(
        checked, 3,
        "schema-one compatibility must cover exactly the three old shared-envelope records"
    );
    Ok(())
}

#[test]
fn schema_one_missing_default_fields_decode_with_historical_defaults() -> Result<(), JournalError> {
    let cases = schema_one_missing_field_cases()?;
    for case in &cases {
        let bytes = encode_schema_one_fixture_record(
            &case.payload,
            case.envelope_kind,
            case.expected.seq(),
        )?;
        let (envelope, decoded) =
            decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(
            envelope.schema_version, SCHEMA_ONE_VERSION_FOR_TEST,
            "{} must remain a schema-one fixture",
            case.name
        );
        assert_eq!(
            envelope.record_kind,
            case.envelope_kind.id(),
            "{} envelope kind changed",
            case.name
        );
        assert_eq!(
            decoded, case.expected,
            "{} must decode with historical default values",
            case.name
        );
    }
    Ok(())
}

#[test]
fn schema_one_fixture_unchanged_envelope_rejects_trailing_postcard_bytes()
-> Result<(), JournalError> {
    let case = SchemaOneFixtureCase {
        payload: SchemaOneJournalEventFixture::RunAccepted {
            run: RunId::new(901),
            seq: EventSeq::new(1),
            workflow: codec_sample_digest(0x91),
        },
        expected: JournalEvent::RunAccepted {
            run: RunId::new(901),
            seq: EventSeq::new(1),
            workflow: codec_sample_digest(0x91),
        },
        envelope_kind: RecordKind::RunAccepted,
    };
    let mut payload = schema_one_fixture_payload(&case.payload)?;
    payload.push(0x00);
    let bytes = encode_schema_one_payload_bytes(&payload, case.envelope_kind, case.expected.seq())?;

    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(
            result,
            Err(JournalError::PostcardDecodeFailed(
                postcard::Error::DeserializeBadEncoding
            ))
        ),
        "schema-one unchanged-envelope fixture with trailing bytes must be rejected, got {result:?}"
    );
    Ok(())
}

#[test]
fn schema_one_fixture_shared_envelope_rejects_trailing_postcard_bytes() -> Result<(), JournalError>
{
    let cases = schema_one_fixture_cases()?;
    let mut checked = 0usize;
    for (i, case) in cases.iter().enumerate() {
        if case.envelope_kind == case.expected.record_kind() {
            continue;
        }
        let mut payload = schema_one_fixture_payload(&case.payload)?;
        payload.push(0x00);
        let bytes =
            encode_schema_one_payload_bytes(&payload, case.envelope_kind, case.expected.seq())?;

        let result =
            decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(
                result,
                Err(JournalError::PostcardDecodeFailed(
                    postcard::Error::DeserializeBadEncoding
                ))
            ),
            "schema-one shared-envelope fixture with trailing bytes must be rejected at index {i}, got {result:?}"
        );
        checked = checked.saturating_add(1);
    }
    assert_eq!(
        checked, 3,
        "trailing-byte rejection must cover exactly the three schema-one shared-envelope cases"
    );
    Ok(())
}

#[test]
fn schema_one_missing_default_field_payloads_reject_trailing_postcard_bytes()
-> Result<(), JournalError> {
    let cases = schema_one_missing_field_cases()?;
    for case in &cases {
        let mut payload = schema_one_fixture_payload(&case.payload)?;
        payload.push(0x00);
        let bytes =
            encode_schema_one_payload_bytes(&payload, case.envelope_kind, case.expected.seq())?;
        let result =
            decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(
                result,
                Err(JournalError::PostcardDecodeFailed(
                    postcard::Error::DeserializeBadEncoding
                ))
            ),
            "{} fixture with trailing bytes must be rejected, got {result:?}",
            case.name
        );
    }
    Ok(())
}

#[test]
fn workflow_source_payload_rejects_digest_valid_trailing_postcard_bytes() -> Result<(), JournalError>
{
    let source = b"workflow: exact-consumption".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord { digest, source };
    let mut payload = postcard::to_allocvec(&record).map_err(JournalError::Encode)?;
    payload.push(0x00);
    let payload_len = super::payload::payload_len_u32(payload.len(), MAX_WORKFLOW_SOURCE_BYTES)?;
    let bytes = super::payload::encode_record_payload(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &payload,
        payload_len,
    )?;

    let result = decode_record::<WorkflowSourceRecord>(
        &bytes,
        MAGIC_WORKFLOW_SOURCE,
        MAX_WORKFLOW_SOURCE_BYTES,
    );
    assert!(
        matches!(
            result,
            Err(JournalError::PostcardDecodeFailed(
                postcard::Error::DeserializeBadEncoding
            ))
        ),
        "non-journal decode_record payload with digest-valid trailing bytes must be rejected, got {result:?}"
    );
    Ok(())
}
