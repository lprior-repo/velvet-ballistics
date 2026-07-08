#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy)]
pub(crate) enum ExpectedDecodeError {
    UnsupportedSchemaVersion { version: u16 },
    MigrationRequired { from: u16, to: u16 },
    UnknownRecordKind { kind: u16 },
    RecordKindFamilyMismatch { magic: u32, kind: u16 },
    HeaderLengthMismatch { found: u32 },
    HeaderChecksumMismatch,
    PayloadDigestMismatch,
    UnexpectedEof,
}

pub(crate) fn assert_decode_error(
    bytes: &[u8],
    magic: u32,
    max_payload: u32,
    expected: ExpectedDecodeError,
) {
    let result = vb_storage::decode_record::<vb_storage::JournalEvent>(bytes, magic, max_payload);
    match (result, expected) {
        (
            Err(vb_storage::JournalError::UnsupportedSchemaVersion { version }),
            ExpectedDecodeError::UnsupportedSchemaVersion {
                version: expected_version,
            },
        ) => assert!(
            version == expected_version,
            "UnsupportedSchemaVersion carried {version}, expected {expected_version}"
        ),
        (
            Err(vb_storage::JournalError::MigrationRequired { from, to }),
            ExpectedDecodeError::MigrationRequired {
                from: expected_from,
                to: expected_to,
            },
        ) => assert_migration_required(from, to, expected_from, expected_to),
        (
            Err(vb_storage::JournalError::UnknownRecordKind { kind }),
            ExpectedDecodeError::UnknownRecordKind {
                kind: expected_kind,
            },
        ) => assert!(
            kind == expected_kind,
            "UnknownRecordKind carried {kind}, expected {expected_kind}"
        ),
        (
            Err(vb_storage::JournalError::RecordKindFamilyMismatch { magic, kind }),
            ExpectedDecodeError::RecordKindFamilyMismatch {
                magic: expected_magic,
                kind: expected_kind,
            },
        ) => assert_family_mismatch(magic, kind, expected_magic, expected_kind),
        (
            Err(vb_storage::JournalError::HeaderLengthMismatch { found }),
            ExpectedDecodeError::HeaderLengthMismatch {
                found: expected_found,
            },
        ) => assert!(
            found == expected_found,
            "HeaderLengthMismatch carried {found}, expected {expected_found}"
        ),
        (
            Err(vb_storage::JournalError::HeaderChecksumMismatch),
            ExpectedDecodeError::HeaderChecksumMismatch,
        ) => {}
        (
            Err(vb_storage::JournalError::PayloadDigestMismatch),
            ExpectedDecodeError::PayloadDigestMismatch,
        ) => {}
        (Err(vb_storage::JournalError::UnexpectedEof), ExpectedDecodeError::UnexpectedEof) => {}
        (Ok((envelope, event)), expected) => assert!(
            std::hint::black_box(false),
            "decode accepted corrupted record as {envelope:?} / {event:?}, expected {expected:?}"
        ),
        (Err(error), expected) => assert!(
            std::hint::black_box(false),
            "decode returned {error:?}, expected {expected:?}"
        ),
    }
}

pub(crate) fn assert_digest_mismatch(result: Result<(), vb_storage::JournalError>) {
    match result {
        Err(vb_storage::JournalError::PayloadDigestMismatch) => {}
        Ok(()) => assert!(
            std::hint::black_box(false),
            "digest verifier accepted a corrupted digest"
        ),
        Err(error) => assert!(
            std::hint::black_box(false),
            "digest verifier returned {error:?}, expected PayloadDigestMismatch"
        ),
    }
}

fn assert_migration_required(from: u16, to: u16, expected_from: u16, expected_to: u16) {
    assert!(
        from == expected_from,
        "MigrationRequired from {from}, expected {expected_from}"
    );
    assert!(
        to == expected_to,
        "MigrationRequired to {to}, expected {expected_to}"
    );
}

fn assert_family_mismatch(magic: u32, kind: u16, expected_magic: u32, expected_kind: u16) {
    assert!(
        magic == expected_magic,
        "RecordKindFamilyMismatch magic {magic:#010x}, expected {expected_magic:#010x}"
    );
    assert!(
        kind == expected_kind,
        "RecordKindFamilyMismatch kind {kind}, expected {expected_kind}"
    );
}
