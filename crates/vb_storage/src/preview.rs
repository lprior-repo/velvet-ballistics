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
use crate::keys::decode_storage_key;
use crate::types::{DecodedPreview, PreviewConfig, PreviewPayload, StorageKey};

/// Produces a bounded preview from a slice of keyspace entries.
///
/// Each entry is a `(key_bytes, value_bytes)` pair as read from the
/// journal. The function:
///
/// 1. Decodes each key via `decode_storage_key()`. Corrupt keys are
///    silently skipped (consistent with production doctor behavior).
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
///
/// # Errors
///
/// Returns `JournalError::PayloadTooLarge` if any entry's value bytes
/// length exceeds `u32::MAX`, guarding against silent truncation on
/// 64-bit platforms.
pub fn preview_keyspace(
    config: PreviewConfig,
    entries: &[(Vec<u8>, Vec<u8>)],
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
        // Decode the key. Corrupt keys are silently skipped
        // (consistent with production doctor behavior).
        let key = match decode_storage_key(key_bytes) {
            Ok(k) => k,
            Err(_) => continue,
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
        bytes_accumulated = projected; // Safe: checked projected <= max_bytes_val
        result_entries.push((key, value_bytes.clone(), PreviewPayload::Raw));
        records_yielded = records_yielded.saturating_add(1);
    }

    Ok(DecodedPreview {
        entries: result_entries,
        total_keyspace_records: total_entries,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PreviewConfig;
    #[test]
    fn empty_entries_produces_empty_preview() {
        let config = PreviewConfig::new(10, 1024).unwrap();
        let entries: Vec<(Vec<u8>, Vec<u8>)> = vec![];
        let result = preview_keyspace(config, &entries).unwrap();
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
        let result = preview_keyspace(config, &entries).unwrap();
        assert!(result.entries.len() <= 3);
        assert!(result.truncated);
    }

    #[test]
    fn max_bytes_hard_cap_never_exceeded() {
        let config = PreviewConfig::new(100, 50).unwrap();
        let entries: Vec<_> = (0..10)
            .map(|_| (vec![0x10, 0, 0, 0, 0, 0, 0, 0, 1], vec![0u8; 20]))
            .collect();
        let result = preview_keyspace(config, &entries).unwrap();
        // Each entry is 20 bytes, max_bytes is 50. At most 2 entries (40 bytes) +
        // the 3rd would be 60 which exceeds 50, so max 2 entries.
        assert!(result.entries.len() <= 5);
        let total: u32 = result.entries.iter().map(|(_, v, _)| v.len() as u32).sum();
        assert!(total <= 50);
    }
}
