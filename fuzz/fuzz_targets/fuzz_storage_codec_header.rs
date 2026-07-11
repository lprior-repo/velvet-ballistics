#![no_main]

//! Fuzz target: storage_codec_header
//!
//! Split from `vb_storage_codec` (PO-vb-y9d3v-0041). Header-only decode across
//! every known magic and every boundary `max_payload_len` (0, 1, 1024, 64 KiB,
//! 16 MiB, 64 MiB). Boundary input lengths (under the 60-byte header, exactly
//! 60 bytes, and oversized) are exercised by truncating `data`. The oracle
//! is that the decoder must always return a typed `JournalError` rather than
//! panic, regardless of header shape or limit value.
//!
//! Run with: cargo fuzz run fuzz_storage_codec_header -- -max_len=4096 -runs=100000

use libfuzzer_sys::fuzz_target;
use vb_storage::JournalError;

fuzz_target!(|data: &[u8]| {
    let magics: [u32; 8] = [
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::MAGIC_BLOB,
        vb_storage::MAGIC_COMPILED_ARTIFACT,
        vb_storage::MAGIC_SNAPSHOT,
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::MAGIC_INDEX_RECORD,
        0xDEAD_BEEFu32,
        0x0000_0000u32,
    ];
    let limits: [u32; 6] = [0, 1, 1024, 65_536, 16_777_216, 67_108_864];

    // Boundary truncation points: under the header (<60), exactly the header
    // (60), one over (61), and a larger over (128). Each prefix is bounded by
    // `data.len()` so empty inputs are still exercised.
    let cap = data.len().min(128);
    for magic in magics {
        for limit in limits {
            for len in [0usize, 1, 16, 59, 60, 61, 128] {
                if len > cap {
                    continue;
                }
                if let Some(prefix) = data.get(..len) {
                    observe_header_decode(vb_storage::decode_record_header(prefix, magic, limit));
                }
            }
        }
    }
});

fn observe_header_decode(result: Result<vb_storage::RecordHeader, JournalError>) {
    match result {
        Ok(header) => {
            assert!(
                header.payload_len <= vb_storage::MAX_BLOB_BYTES,
                "decoded header payload length must remain inside the largest storage bound"
            );
        }
        Err(error) => assert_header_decode_error(error),
    }
}

fn assert_header_decode_error(error: JournalError) {
    assert!(
        matches!(
            error,
            JournalError::UnexpectedEof
                | JournalError::BadMagic { .. }
                | JournalError::UnsupportedSchemaVersion { .. }
                | JournalError::MigrationRequired { .. }
                | JournalError::UnknownRecordKind { .. }
                | JournalError::RecordKindFamilyMismatch { .. }
                | JournalError::HeaderLengthMismatch { .. }
                | JournalError::PayloadTooLarge { .. }
                | JournalError::HeaderChecksumMismatch
        ),
        "header decoder must return only header-class JournalError variants"
    );
}
