#![forbid(unsafe_code)]

pub(crate) fn assert_encode_ok(result: Result<Vec<u8>, vb_storage::JournalError>) {
    match result {
        Ok(encoded) => assert!(!encoded.is_empty(), "valid encoding produced empty bytes"),
        Err(error) => assert!(
            std::hint::black_box(false),
            "valid encode_record returned {error:?}"
        ),
    }
}

pub(crate) fn assert_encode_family_mismatch(
    result: Result<Vec<u8>, vb_storage::JournalError>,
    expected_magic: u32,
    expected_kind: u16,
) {
    match result {
        Err(vb_storage::JournalError::RecordKindFamilyMismatch { magic, kind }) => {
            assert!(
                magic == expected_magic,
                "family mismatch magic {magic:#010x}, expected {expected_magic:#010x}"
            );
            assert!(
                kind == expected_kind,
                "family mismatch kind {kind}, expected {expected_kind}"
            );
        }
        Ok(bytes) => assert!(
            std::hint::black_box(false),
            "invalid family ({expected_magic:#010x}, {expected_kind}) encoded {} bytes",
            bytes.len()
        ),
        Err(error) => assert!(
            std::hint::black_box(false),
            "invalid family ({expected_magic:#010x}, {expected_kind}) returned {error:?}, expected RecordKindFamilyMismatch"
        ),
    }
}

pub(crate) fn assert_fresh_decode_ok(encoded: &[u8], magic: u32, max_payload: u32) {
    match vb_storage::decode_record::<vb_storage::JournalEvent>(encoded, magic, max_payload) {
        Ok((_, event)) => assert!(
            event.is_valid(),
            "fresh encoded event is invalid: {event:?}"
        ),
        Err(error) => assert!(
            std::hint::black_box(false),
            "fresh encoded record rejected with {error:?}"
        ),
    }
}
