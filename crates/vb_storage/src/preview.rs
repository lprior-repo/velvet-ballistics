#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
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
