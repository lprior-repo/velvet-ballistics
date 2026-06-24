#![forbid(unsafe_code)]
//! Bounded keyspace preview for doctor inspection.
//!
//! `preview_keyspace` processes a stream of key-value entries and
//! applies `PreviewConfig` caps to produce a `DecodedPreview`.
//!
//! The function is I/O-free: it takes a pre-collected entry slice
//! (key bytes + value bytes). The caller (doctor command) is responsible
//! for reading entries from the journal keyspace.

use crate::JournalError;
use crate::error::KeyDecodeError;
use crate::keys::{KeyspaceScanPolicy, decode_storage_key};
use crate::types::{DecodedPreview, PreviewConfig, PreviewPayload, StorageKey};

/// Produces a bounded preview from a slice of keyspace entries.
///
/// Each entry is a `(key_bytes, value_bytes)` pair as read from the
/// journal. The function:
///
/// 1. Decodes each key via `decode_storage_key()`. The supplied
///    [`KeyspaceScanPolicy`] selects whether corrupt keys are
///    skipped silently (`SkipMalformed`) or surface as a typed
///    [`JournalError::MalformedKeyspaceRow`] (`FailClosed`).
/// 2. Applies `max_records` cap: stops including records once the cap
///    is reached.
/// 3. Applies `max_bytes` HARD cap: a record is NOT included if its
///    value bytes would cause `bytes_accumulated + payload_len > max_bytes`.
/// 4. Sets `truncated = true` if iteration stopped before processing
///    all entries due to either cap.
///
/// # Contract Guarantees
///
/// - `result.entries.len() <= config.max_records().get()`
/// - Total bytes accumulated across all entries <= `config.max_bytes()`
/// - `result.truncated` is true iff a cap was hit before all entries
///   were processed.
/// - Under `KeyspaceScanPolicy::SkipMalformed`, malformed keys are
///   silently skipped and do not affect the cap accounting (the cap
///   applies only to records that survive decode).
/// - Under `KeyspaceScanPolicy::FailClosed`, the first malformed key
///   aborts the scan and the function returns
///   [`JournalError::MalformedKeyspaceRow`].
/// - The `scratch` buffer is mutated in-place for every included entry:
///   it is cleared, extended with the entry's value bytes, then swapped
///   out via `mem::take` so the buffer can be reused on the next
///   iteration without re-allocating. The caller's `scratch` is left
///   empty (capacity retained) on return. CC-003 fix: avoids a fresh
///   `Vec<u8>` allocation per included entry.
///
/// # Errors
///
/// Returns `JournalError::PayloadTooLarge` if any entry's value bytes
/// length exceeds `u32::MAX`, guarding against silent truncation on
/// 64-bit platforms. Returns `JournalError::MalformedKeyspaceRow`
/// (only under `KeyspaceScanPolicy::FailClosed`) on the first row
/// whose key cannot be decoded.
pub fn preview_keyspace(
    policy: KeyspaceScanPolicy,
    config: PreviewConfig,
    entries: &[(Vec<u8>, Vec<u8>)],
    scratch: &mut Vec<u8>,
) -> Result<DecodedPreview, JournalError> {
    let max_records_val = config.max_records().get();
    let max_bytes_val = config.max_bytes();

    let mut result_entries: Vec<(StorageKey, Vec<u8>, PreviewPayload)> = Vec::new();
    let mut records_yielded: usize = 0;
    let mut bytes_accumulated: u32 = 0;
    let mut truncated = false;
    let total_entries =
        u64::try_from(entries.len()).map_err(|_| JournalError::PayloadTooLarge {
            len: u32::MAX,
            max: u32::MAX,
        })?;

    for (key_bytes, value_bytes) in entries {
        // Decode the key. The `KeyspaceScanPolicy` selects the failure
        // path: `SkipMalformed` continues, `FailClosed` aborts with a
        // typed `MalformedKeyspaceRow`.
        let key = match decode_storage_key(key_bytes) {
            Ok(k) => k,
            Err(err) => match policy {
                KeyspaceScanPolicy::SkipMalformed => continue,
                KeyspaceScanPolicy::FailClosed => {
                    return Err(malformed_to_journal_error(err, key_bytes.len()));
                }
            },
        };

        let payload_len =
            u32::try_from(value_bytes.len()).map_err(|_| JournalError::PayloadTooLarge {
                len: u32::MAX,
                max: u32::MAX,
            })?;

        // Record cap: stop if we've already yielded max_records.
        if records_yielded >= max_records_val {
            truncated = true;
            break;
        }

        // Byte cap: HARD cap — do NOT include this record if it would
        // cause bytes_accumulated to exceed max_bytes.
        // Uses saturating_add as defensive coding (GOD RULE 3).
        let projected = bytes_accumulated.saturating_add(payload_len);
        if projected > max_bytes_val {
            truncated = true;
            break;
        }

        // Both caps not hit: include this record.
        // CC-003 fix: reuse the caller's scratch buffer instead of
        // allocating a fresh `Vec<u8>` per included entry. After
        // `extend_from_slice`, `mem::take(scratch)` swaps in an empty
        // `Vec` so the next iteration can keep using the same buffer
        // without losing the bytes we just pushed.
        bytes_accumulated = projected; // Safe: checked projected <= max_bytes_val
        scratch.clear();
        scratch.extend_from_slice(value_bytes);
        result_entries.push((key, std::mem::take(scratch), PreviewPayload::Raw));
        records_yielded = records_yielded.saturating_add(1);
    }

    Ok(DecodedPreview {
        entries: result_entries,
        total_keyspace_records: total_entries,
        truncated,
    })
}

// ---------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------

// Translates a `KeyDecodeError` into the typed `MalformedKeyspaceRow`
// variant that production paths surface. Used only on the
// `FailClosed` policy path.
//
// `KeyDecodeError::KeyLengthMismatch` already carries the same three
// fields the typed error expects, so the happy path is a struct
// rename. The other variants (empty key, unknown prefix, semantic
// field rejections) are reduced to a `MalformedKeyspaceRow` with the
// observed `actual_len` and `expected_len = 0` so the operator can
// still pinpoint the offending row.
fn malformed_to_journal_error(err: KeyDecodeError, actual_len: usize) -> JournalError {
    match err {
        KeyDecodeError::KeyLengthMismatch {
            prefix,
            expected,
            actual,
        } => JournalError::MalformedKeyspaceRow {
            prefix,
            expected_len: expected,
            actual_len: actual,
        },
        KeyDecodeError::UnknownPrefix { prefix } => JournalError::MalformedKeyspaceRow {
            prefix,
            expected_len: 0,
            actual_len,
        },
        KeyDecodeError::EmptyKey => JournalError::MalformedKeyspaceRow {
            prefix: 0,
            expected_len: 0,
            actual_len: 0,
        },
        KeyDecodeError::InvalidRunId | KeyDecodeError::ReservedSeqSentinel => {
            // Semantic field rejection — the structural decode succeeded
            // so the prefix and length were already validated. Surface a
            // typed error with `expected_len = 0` to indicate "semantic,
            // not structural" mismatch; the caller can recover the prefix
            // from the first byte if it needs it.
            JournalError::MalformedKeyspaceRow {
                prefix: 0,
                expected_len: 0,
                actual_len,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PreviewConfig;
    #[test]
    fn empty_entries_produces_empty_preview() {
        let config = PreviewConfig::new(10, 1024).unwrap();
        let entries: Vec<(Vec<u8>, Vec<u8>)> = vec![];
        let mut scratch: Vec<u8> = Vec::new();
        let result = preview_keyspace(
            KeyspaceScanPolicy::default_doctor(),
            config,
            &entries,
            &mut scratch,
        )
        .unwrap();
        assert!(result.entries.is_empty());
        assert!(!result.truncated);
        assert_eq!(result.total_keyspace_records, 0);
    }

    #[test]
    fn max_records_cap_never_exceeded() {
        let config = PreviewConfig::new(3, 1024 * 1024).unwrap();
        // Create 10 entries but cap at 3.
        let entries: Vec<_> = (0..10)
            .map(|_| (vec![0x10, 0, 0, 0, 0, 0, 0, 0, 1], vec![42u8; 10]))
            .collect();
        let mut scratch: Vec<u8> = Vec::new();
        let result = preview_keyspace(
            KeyspaceScanPolicy::default_doctor(),
            config,
            &entries,
            &mut scratch,
        )
        .unwrap();
        assert!(result.entries.len() <= 3);
        assert!(result.truncated);
    }

    #[test]
    fn max_bytes_hard_cap_never_exceeded() {
        let config = PreviewConfig::new(100, 50).unwrap();
        let entries: Vec<_> = (0..10)
            .map(|_| (vec![0x10, 0, 0, 0, 0, 0, 0, 0, 1], vec![0u8; 20]))
            .collect();
        let mut scratch: Vec<u8> = Vec::new();
        let result = preview_keyspace(
            KeyspaceScanPolicy::default_doctor(),
            config,
            &entries,
            &mut scratch,
        )
        .unwrap();
        // Each entry is 20 bytes, max_bytes is 50. At most 2 entries (40 bytes) +
        // the 3rd would be 60 which exceeds 50, so max 2 entries.
        assert!(result.entries.len() <= 5);
        let total: u32 = result.entries.iter().map(|(_, v, _)| v.len() as u32).sum();
        assert!(total <= 50);
    }

    // -----------------------------------------------------------------
    // CC-002 follow-up: policy-aware malformed-key handling.
    // -----------------------------------------------------------------

    #[test]
    fn preview_keyspace_skips_malformed() {
        // SkipMalformed: a corrupt key in the middle of an entry list
        // is silently dropped; surrounding valid entries still appear.
        let config = PreviewConfig::new(10, 1024).unwrap();
        // Valid run-header keys for runs 1, 2, 3.
        let make_valid = |run: u64| {
            crate::keys::run_header_key(vb_core::RunId::new(run))
                .unwrap()
                .to_vec()
        };
        // A length-mismatched run-header key: PREFIX_RUN_HEADER + 2 bytes
        // (expected 9 bytes total). Structural `KeyLengthMismatch` error.
        let short_key = vec![0x10, 0xAB, 0xCD];
        let entries = vec![
            (make_valid(1), vec![0xAAu8; 4]),
            (short_key, vec![0xBBu8; 4]),
            (make_valid(2), vec![0xCCu8; 4]),
            (make_valid(3), vec![0xDDu8; 4]),
        ];
        let mut scratch: Vec<u8> = Vec::new();
        let result = preview_keyspace(
            KeyspaceScanPolicy::default_doctor(),
            config,
            &entries,
            &mut scratch,
        )
        .unwrap();
        assert_eq!(result.entries.len(), 3);
        assert!(!result.truncated);
        assert_eq!(result.total_keyspace_records, 4);
        // The three surviving entries are runs 1, 2, 3 — order preserved.
        for (i, (key, _, _)) in result.entries.iter().enumerate() {
            match key {
                crate::StorageKey::RunHeader { run } => {
                    assert_eq!(run.get(), (i as u64) + 1);
                }
                other => panic!("expected RunHeader, got {other:?}"),
            }
        }
    }

    #[test]
    fn preview_keyspace_fails_closed() {
        // FailClosed: a malformed key aborts the scan and surfaces a
        // typed `MalformedKeyspaceRow` error carrying prefix, expected
        // length, and actual length.
        let config = PreviewConfig::new(10, 1024).unwrap();
        let make_valid = |run: u64| {
            crate::keys::run_header_key(vb_core::RunId::new(run))
                .unwrap()
                .to_vec()
        };
        // Well-formed entry first so the policy reaches a malformed row
        // at index 1 (the length-mismatch case).
        let short_key: Vec<u8> = vec![0x10, 0x00, 0x00, 0x00]; // 4 bytes, expected 9
        let entries = vec![
            (make_valid(1), vec![0xAAu8; 4]),
            (short_key.clone(), vec![0xBBu8; 4]),
            (make_valid(2), vec![0xCCu8; 4]),
        ];
        let mut scratch: Vec<u8> = Vec::new();
        let err = preview_keyspace(
            KeyspaceScanPolicy::default_production(),
            config,
            &entries,
            &mut scratch,
        )
        .expect_err("FailClosed must abort on malformed key");
        match err {
            JournalError::MalformedKeyspaceRow {
                prefix,
                expected_len,
                actual_len,
            } => {
                assert_eq!(prefix, 0x10);
                assert_eq!(expected_len, 9);
                assert_eq!(actual_len, short_key.len());
            }
            other => panic!("expected MalformedKeyspaceRow, got {other:?}"),
        }
    }

    #[test]
    fn preview_keyspace_fail_closed_unknown_prefix() {
        // FailClosed path also catches unknown-prefix keys with the
        // typed error. `expected_len` is 0 because there is no
        // structural expectation for an unrecognised prefix byte.
        let config = PreviewConfig::new(10, 1024).unwrap();
        // First byte 0xFF is not one of the nine known prefixes.
        let entries: Vec<(Vec<u8>, Vec<u8>)> = vec![(vec![0xFF, 0x01, 0x02, 0x03], vec![0u8; 4])];
        let mut scratch: Vec<u8> = Vec::new();
        let err = preview_keyspace(
            KeyspaceScanPolicy::default_production(),
            config,
            &entries,
            &mut scratch,
        )
        .expect_err("FailClosed must abort on unknown prefix");
        match err {
            JournalError::MalformedKeyspaceRow {
                prefix,
                expected_len,
                actual_len,
            } => {
                assert_eq!(prefix, 0xFF);
                assert_eq!(expected_len, 0);
                assert_eq!(actual_len, 4);
            }
            other => panic!("expected MalformedKeyspaceRow, got {other:?}"),
        }
    }
}
