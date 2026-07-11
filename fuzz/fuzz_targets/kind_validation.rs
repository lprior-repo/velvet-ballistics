#![no_main]

//! Fuzz target: kind_validation — PO-FUZZ-001 (vb-b8i8f)
//!
//! Exercises validate_kind_family, is_known_record_kind, and validate_known_kind
//! with arbitrary (magic, kind) pairs from byte input.
//!
//! GOD RULE: Zero panics, zero crashes. All errors must be typed JournalError variants.
//!
//! Coverage goals:
//! - Known journal kinds, including 31..35, accepted for MAGIC_JOURNAL_EVENT
//! - Journal kinds rejected for MAGIC_SNAPSHOT, MAGIC_BLOB, etc.
//! - Unknown kinds rejected with UnknownRecordKind
//! - All known magic values exercised
//!
//! Command: cargo +nightly fuzz run kind_validation -- -max_len=8 -runs=100000

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Parse (magic, kind) from input: first 4 bytes = magic (u32 LE), bytes 4-5 = kind (u16 LE)
    let magic = read_u32_le(data, 0).unwrap_or(vb_storage::MAGIC_JOURNAL_EVENT);

    let kind = read_u16_le(data, 4)
        .or_else(|| read_u16_le(data, 0))
        .unwrap_or(vb_storage::RecordKind::RunKilled.id());

    // Test 1: is_known_record_kind — must not panic
    let _known = vb_storage::codec::is_known_record_kind(kind);

    // Test 2: validate_known_kind — must not panic
    let known_result = vb_storage::codec::validate_known_record_kind(kind);
    match known_result {
        Ok(()) => {
            // Kind is known. Verify that is_known_record_kind agrees.
            assert_eq!(
                vb_storage::codec::is_known_record_kind(kind),
                true,
                "validate_known_kind Ok but is_known_record_kind false for kind {}",
                kind
            );
        }
        Err(e) => {
            // Kind is unknown. Verify error is correctly typed.
            assert_typed_unknown_kind_error(e, kind);
        }
    }

    // Test 3: validate_kind_family — must not panic
    let family_result = vb_storage::codec::validate_record_kind_family(magic, kind);
    match family_result {
        Ok(()) => {
            // Kind-magic pair is valid. The oracle is the production
            // RecordKind classifier so newly admitted journal kinds (31..35)
            // are accepted without duplicating stale range checks here.
            let accepted_by_production = vb_storage::RecordKind::from_id(kind)
                .is_some_and(|record_kind| record_kind.belongs_to_magic(magic));
            assert_eq!(
                accepted_by_production,
                true,
                "validate_record_kind_family accepted ({magic:#010x}, {kind}) but RecordKind rejected it"
            );
        }
        Err(e) => {
            // Kind-magic pair is invalid. Verify error is correctly typed.
            assert_typed_family_error(e, magic, kind);
        }
    }

    // Test 4: Exercise all known magic values
    let all_magics: [u32; 6] = [
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::MAGIC_BLOB,
        vb_storage::MAGIC_COMPILED_ARTIFACT,
        vb_storage::MAGIC_SNAPSHOT,
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::MAGIC_INDEX_RECORD,
    ];

    for test_magic in all_magics {
        drop(vb_storage::codec::validate_record_kind_family(
            test_magic, kind,
        ));
    }

    // Test 5: Boundary magic values
    for boundary_magic in [0xFFFF_FFFFu32, 0x0000_0000u32] {
        drop(vb_storage::codec::validate_record_kind_family(
            boundary_magic,
            kind,
        ));
    }
});

fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    let end = offset.checked_add(2)?;
    let bytes = data.get(offset..end)?;
    let array = <[u8; 2]>::try_from(bytes).ok()?;
    Some(u16::from_le_bytes(array))
}

fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let bytes = data.get(offset..end)?;
    let array = <[u8; 4]>::try_from(bytes).ok()?;
    Some(u32::from_le_bytes(array))
}

/// Assert that an unknown kind error is properly typed.
fn assert_typed_unknown_kind_error(error: vb_storage::JournalError, expected_kind: u16) {
    match error {
        vb_storage::JournalError::UnknownRecordKind { kind } => {
            assert_eq!(
                kind,
                expected_kind,
                "UnknownRecordKind carried {kind}, expected {expected_kind}"
            );
        }
        other => assert!(
            std::hint::black_box(false),
            "validate_known_kind returned {other:?}, expected UnknownRecordKind({expected_kind})"
        ),
    }
}

/// Assert that a family mismatch error is properly typed.
fn assert_typed_family_error(error: vb_storage::JournalError, magic: u32, kind: u16) {
    match vb_storage::RecordKind::from_id(kind) {
        Some(record_kind) if !record_kind.belongs_to_magic(magic) => match error {
            vb_storage::JournalError::RecordKindFamilyMismatch {
                magic: err_magic,
                kind: err_kind,
            } => {
                assert_eq!(
                    err_magic,
                    magic,
                    "family mismatch magic {err_magic:#010x} must match input {magic:#010x}"
                );
                assert_eq!(
                    err_kind,
                    kind,
                    "family mismatch kind {err_kind} must match input {kind}"
                );
            }
            other => assert!(
                std::hint::black_box(false),
                "known invalid family ({magic:#010x}, {kind}) returned {other:?}, expected RecordKindFamilyMismatch"
            ),
        },
        None => assert_typed_unknown_kind_error(error, kind),
        Some(record_kind) => {
            assert!(
                std::hint::black_box(false),
                "valid family ({magic:#010x}, {kind:?}) was rejected with {error:?}",
                kind = record_kind
            );
        }
    }
}
