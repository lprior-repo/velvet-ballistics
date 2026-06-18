#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
use super::{encoded_record, flip_byte, scribble_u32};
use crate::JournalError;
use crate::codec::decode_journal_event;
use crate::constants::MAGIC_JOURNAL_EVENT;

#[test]
fn decode_rejects_truncated_payload() {
    let mut bytes = encoded_record();
    let new_len = bytes.len().saturating_sub(4);
    bytes.truncate(new_len);
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("truncated payload must fail decode");
    assert!(
        matches!(err, JournalError::UnexpectedEof),
        "truncated payload must yield UnexpectedEof, got {err:?}"
    );
}

#[test]
fn decode_rejects_swapped_magic() {
    let mut bytes = encoded_record();
    scribble_u32(&mut bytes, 0);
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("swapped magic must fail decode");
    assert!(
        matches!(err, JournalError::BadMagic { .. }),
        "swapped magic must yield BadMagic, got {err:?}"
    );
}

#[test]
fn decode_rejects_corrupted_crc32c() {
    let mut bytes = encoded_record();
    scribble_u32(&mut bytes, 56);
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("corrupted CRC32C must fail decode");
    assert!(
        matches!(err, JournalError::HeaderChecksumMismatch),
        "corrupted CRC32C must yield HeaderChecksumMismatch, got {err:?}"
    );
}

#[test]
fn decode_rejects_blake3_digest_mismatch() {
    let mut bytes = encoded_record();
    flip_byte(&mut bytes, 40);
    let new_crc = crc32c::crc32c(&bytes[..56]);
    bytes[56..60].copy_from_slice(&new_crc.to_le_bytes());
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("BLAKE3 digest mismatch must fail decode");
    assert!(
        matches!(err, JournalError::PayloadDigestMismatch),
        "BLAKE3 digest mismatch must yield PayloadDigestMismatch, got {err:?}"
    );
}

#[test]
fn decode_rejects_payload_len_overflow() {
    let mut bytes = encoded_record();
    let max_payload = 65_536_u32;
    let max_bytes = u32::MAX.to_le_bytes();
    for (i, slot) in bytes.iter_mut().enumerate().skip(12).take(4) {
        *slot = max_bytes[i - 12];
    }
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, max_payload)
        .expect_err("payload_len overflow must fail decode");
    assert!(
        matches!(err, JournalError::PayloadTooLarge { .. }),
        "payload_len overflow must yield PayloadTooLarge, got {err:?}"
    );
}

#[test]
fn decode_rejects_header_len_mismatch() {
    let mut bytes = encoded_record();
    scribble_u32(&mut bytes, 8);
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("header_len mismatch must fail decode");
    assert!(
        matches!(err, JournalError::HeaderLengthMismatch { .. }),
        "header_len mismatch must yield HeaderLengthMismatch, got {err:?}"
    );
}

#[test]
fn decode_rejects_record_kind_outside_family() {
    let mut bytes = encoded_record();
    let kind_bytes = 0x00_FF_u16.to_le_bytes();
    for (i, slot) in bytes.iter_mut().enumerate().skip(6).take(2) {
        *slot = kind_bytes[i - 6];
    }
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("record_kind outside family must fail decode");
    assert!(
        matches!(
            err,
            JournalError::UnknownRecordKind { .. } | JournalError::RecordKindFamilyMismatch { .. }
        ),
        "invalid record_kind must yield UnknownRecordKind or RecordKindFamilyMismatch, got {err:?}"
    );
}

#[test]
fn decode_rejects_unknown_record_kind_family() {
    let mut bytes = encoded_record();
    for slot in bytes.iter_mut().skip(6).take(2) {
        *slot = 0;
    }
    let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
        .expect_err("unknown record_kind must fail decode");
    assert!(
        matches!(
            err,
            JournalError::UnknownRecordKind { .. } | JournalError::RecordKindFamilyMismatch { .. }
        ),
        "unknown record_kind family must yield typed error, got {err:?}"
    );
}
