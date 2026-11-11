use super::*;

/// Independent canonical enumeration of all known RecordKind IDs from kind.rs.
/// Deliberately not derived via production from_id; this array is the test oracle.
const KNOWN_KIND_IDS: [u16; 45] = [
    1, 2, 3, 4, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
    31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
];

/// Independent enumeration of known IDs that are NOT journal events.
/// These must produce RecordKindFamilyMismatch under MAGIC_JOURNAL_EVENT.
const NON_JOURNAL_KIND_IDS: [u16; 8] = [1, 2, 3, 4, 30, 40, 42, 50];

/// Exact family assertion for a known kind under journal magic.
/// Non-journal kinds must produce RecordKindFamilyMismatch with exact magic/kind.
/// Journal kinds must decode Ok with decoded.record_kind == kind.
fn exact_assert_family(kind: u16, result: Result<vb_storage::types::RecordHeader, JournalError>) {
    if NON_JOURNAL_KIND_IDS.contains(&kind) {
        assert!(matches!(
            result,
            Err(JournalError::RecordKindFamilyMismatch {
                magic: vb_storage::constants::MAGIC_JOURNAL_EVENT,
                kind: k
            }) if k == kind
        ));
    } else {
        assert!(matches!(result, Ok(decoded) if decoded.record_kind == kind));
    }
}

#[test]
fn decode_rejects_unknown_record_kind() -> Result<(), JournalError> {
    // Given: a record header with kind = 255 (unknown)
    let mut header = make_header_with_kind(255);
    corrupt_kind_in_header(&mut header, 255);

    // When: decode_record_header is called
    let result = decode_record_header(
        &header,
        vb_storage::constants::MAGIC_JOURNAL_EVENT,
        vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    // Then: the result is Err(JournalError::UnknownRecordKind { kind: 255 })
    match result {
        Err(JournalError::UnknownRecordKind { kind }) => {
            assert_eq!(kind, 255, "unknown kind must be 255");
        }
        other => panic!("expected UnknownRecordKind(255), got {:?}", other),
    }
    Ok(())
}

#[test]
fn decode_accepts_all_known_journal_event_kinds() -> Result<(), JournalError> {
    for kind in journal_event_kinds() {
        exact_assert_family(kind, decode_kind_header(kind));
    }
    for kind in [38, 48, 49] {
        assert_positive_kind(kind)?;
    }
    Ok(())
}

#[test]
fn is_known_record_kind_returns_true_for_valid_kinds() {
    // Iterate every known ID from the independent canonical array.
    // Under journal magic, non-journal kinds must produce exact
    // RecordKindFamilyMismatch; journal kinds must decode Ok with
    // decoded.record_kind == kind. The helper exact_assert_family encodes
    // both expectations without accepting any other result.
    for &kind in &KNOWN_KIND_IDS {
        let mut header = make_header_with_kind(kind);
        corrupt_kind_in_header(&mut header, kind);

        let result = decode_record_header(
            &header,
            vb_storage::constants::MAGIC_JOURNAL_EVENT,
            vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );

        exact_assert_family(kind, result);
    }
}

#[test]
fn is_known_record_kind_returns_false_for_invalid_kinds() {
    for kind in u16::MIN..=u16::MAX {
        assert_record_kind(kind);
    }
    assert_kind_four_family();
}

fn decode_kind_header(kind: u16) -> Result<vb_storage::types::RecordHeader, JournalError> {
    let mut header = make_header_with_kind(kind);
    corrupt_kind_in_header(&mut header, kind);
    decode_record_header(
        &header,
        vb_storage::constants::MAGIC_JOURNAL_EVENT,
        vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
}

fn assert_positive_kind(kind: u16) -> Result<(), JournalError> {
    let decoded = decode_kind_header(kind)?;
    assert_eq!(
        decoded.record_kind, kind,
        "kind {} must decode with record_kind {}, got {}",
        kind, kind, decoded.record_kind
    );
    Ok(())
}

fn assert_record_kind(kind: u16) {
    let is_known = KNOWN_KIND_IDS.contains(&kind);
    let result = decode_kind_header(kind);
    if is_known {
        exact_assert_family(kind, result);
    } else {
        assert_exact_unknown(kind, result);
    }
}

fn assert_exact_unknown(kind: u16, result: Result<vb_storage::types::RecordHeader, JournalError>) {
    assert!(
        matches!(
            result,
            Err(JournalError::UnknownRecordKind { kind: k }) if k == kind
        ),
        "unknown kind {} must produce UnknownRecordKind with exact echoed ID, got {:?}",
        kind,
        result
    );
}

fn journal_event_kinds() -> Vec<u16> {
    vec![
        10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 31, 32, 33,
        34, 35, 36, 37, 38, 39, 41, 43, 44, 45, 46, 47, 48, 49,
    ]
}

fn assert_kind_four_family() {
    // Kind 4 is a known compiled-artifact kind (MAGIC_COMPILED_ARTIFACT).
    // When used with MAGIC_JOURNAL_EVENT, it must produce RecordKindFamilyMismatch,
    // not UnknownRecordKind.
    let kind4_header = make_header_with_kind(4);
    let mut kind4_corrupted = kind4_header;
    corrupt_kind_in_header(&mut kind4_corrupted, 4);

    let result = decode_record_header(
        &kind4_corrupted,
        vb_storage::constants::MAGIC_JOURNAL_EVENT,
        vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    assert!(
        matches!(
            result,
            Err(JournalError::RecordKindFamilyMismatch {
                magic: vb_storage::constants::MAGIC_JOURNAL_EVENT,
                kind: 4
            })
        ),
        "kind 4 with journal magic must produce RecordKindFamilyMismatch, got {:?}",
        result
    );
}
