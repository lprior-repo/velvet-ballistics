#![forbid(unsafe_code)]
// Tight-scope `clippy::unwrap_used` allows appear on each individual test
// function below. They are NOT permitted at file scope because Holzman Rust
// doctrine (NASA/JPL Power of Ten) requires the tightest scope for every
// lint override and forbids blanket silences over production code paths.
//! Bounded keyspace preview for doctor inspection.
//!
//! `preview_keyspace` processes a stream of key-value entries and
//! applies `PreviewConfig` caps to produce a `DecodedPreview`.
//!
//! The function is I/O-free: it takes a pre-collected entry slice
//! (key bytes + value bytes). The caller (doctor command) is responsible
//! for reading entries from the journal keyspace.

use std::ops::ControlFlow;

use crate::JournalError;
use crate::keys::decode_storage_key;
use crate::types::{DecodedPreview, PreviewConfig, PreviewPayload, StorageKey};

/// Per-iteration state threaded through `process_entry`.
struct PreviewState {
    records_yielded: usize,
    bytes_accumulated: u32,
    truncated: bool,
    max_records_val: usize,
    max_bytes_val: u32,
}

impl PreviewState {
    /// Create a fresh `PreviewState` from the configured caps.
    const fn new(max_records_val: usize, max_bytes_val: u32) -> Self {
        Self {
            records_yielded: 0,
            bytes_accumulated: 0,
            truncated: false,
            max_records_val,
            max_bytes_val,
        }
    }
}

/// Check whether admitting `payload_len` extra bytes would exceed `max_bytes`.
///
/// Returns `Ok(new_accumulated)` if the record fits within the cap, or
/// `Err(())` if the projected running total would exceed `max_bytes`.
/// The sum is computed with `saturating_add` per the defensive-coding rule.
const fn try_admit_record(
    bytes_accumulated: u32,
    payload_len: u32,
    max_bytes: u32,
) -> Result<u32, ()> {
    let projected = bytes_accumulated.saturating_add(payload_len);
    if projected > max_bytes {
        Err(())
    } else {
        Ok(projected)
    }
}

/// Process a single (key, value) entry: decode, byte-cap check, records-cap check, append.
///
/// Returns `ControlFlow::Break(())` once either cap has been hit and the
/// caller should stop iterating, or `ControlFlow::Continue(())` if the
/// entry was admitted (or silently skipped because of an invalid key).
fn process_entry(
    (key_bytes, value_bytes): (&[u8], &[u8]),
    result_entries: &mut Vec<(StorageKey, Vec<u8>, PreviewPayload)>,
    state: &mut PreviewState,
) -> Result<ControlFlow<()>, JournalError> {
    let key = match decode_storage_key(key_bytes) {
        Ok(k) => k,
        Err(_) => return Ok(ControlFlow::Continue(())),
    };
    let payload_len =
        u32::try_from(value_bytes.len()).map_err(|_| JournalError::PayloadLenOverflow {
            len: u64::try_from(value_bytes.len()).unwrap_or(u64::MAX),
        })?;
    if state.records_yielded >= state.max_records_val {
        state.truncated = true;
        return Ok(ControlFlow::Break(()));
    }
    match try_admit_record(state.bytes_accumulated, payload_len, state.max_bytes_val) {
        Ok(new_total) => {
            state.bytes_accumulated = new_total;
            result_entries.push((key, value_bytes.to_vec(), PreviewPayload::Raw));
            state.records_yielded = state.records_yielded.saturating_add(1);
            Ok(ControlFlow::Continue(()))
        }
        Err(()) => {
            state.truncated = true;
            Ok(ControlFlow::Break(()))
        }
    }
}

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
/// Returns `JournalError::PayloadLenOverflow` if any entry's value bytes
/// length exceeds `u32::MAX`, carrying the real observed length, or if
/// `entries.len()` overflows `u64` on 32-bit platforms.
pub fn preview_keyspace(
    config: PreviewConfig,
    entries: &[(Vec<u8>, Vec<u8>)],
) -> Result<DecodedPreview, JournalError> {
    let total_entries = u64::try_from(entries.len())
        .map_err(|_| JournalError::PayloadLenOverflow {
            len: match u64::try_from(entries.len()) {
                Ok(value) => value,
                Err(_) => u64::MAX,
            },
        })?;
    let mut result: Vec<(StorageKey, Vec<u8>, PreviewPayload)> =
        Vec::with_capacity(config.max_records().get());
    let mut state = PreviewState::new(config.max_records().get(), config.max_bytes());
    for (k, v) in entries {
        if process_entry((k, v), &mut result, &mut state)?.is_break() {
            break;
        }
    }
    Ok(DecodedPreview {
        entries: result,
        total_keyspace_records: total_entries,
        truncated: state.truncated,
    })
}

#[cfg(test)]
mod preview_red_queen_bytes_props;
#[cfg(test)]
mod preview_red_queen_keys_props;
#[cfg(test)]
mod preview_red_queen_records_props;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PreviewConfig;

    #[test]
    #[allow(
        clippy::unwrap_used,
        reason = "test fixture: hard-coded inputs are statically valid"
    )]
    fn empty_entries_produces_empty_preview() {
        let config = PreviewConfig::new(10, 1024).unwrap();
        let entries: Vec<(Vec<u8>, Vec<u8>)> = vec![];
        let result = preview_keyspace(config, &entries).unwrap();
        assert!(result.entries.is_empty());
        assert!(!result.truncated);
        assert_eq!(result.total_keyspace_records, 0);
    }

    #[test]
    #[allow(
        clippy::unwrap_used,
        reason = "test fixture: hard-coded inputs are statically valid"
    )]
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
    #[allow(
        clippy::unwrap_used,
        reason = "test fixture: hard-coded inputs are statically valid"
    )]
    fn max_bytes_hard_cap_never_exceeded() {
        let config = PreviewConfig::new(100, 50).unwrap();
        let entries: Vec<_> = (0..10)
            .map(|_| (vec![0x10, 0, 0, 0, 0, 0, 0, 0, 1], vec![0u8; 20]))
            .collect();
        let result = preview_keyspace(config, &entries).unwrap();
        // Each entry is 20 bytes, max_bytes is 50. At most 2 entries (40 bytes) +
        // the 3rd would be 60 which exceeds 50, so max 2 entries.
        assert!(result.entries.len() <= 5);
        let total: u32 = result
            .entries
            .iter()
            .map(|(_, v, _)| u32::try_from(v.len()).unwrap_or(u32::MAX))
            .sum();
        assert!(total <= 50);
    }
}
