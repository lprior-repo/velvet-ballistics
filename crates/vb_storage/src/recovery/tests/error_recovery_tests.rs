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
    unused_variables,
)]

#![forbid(unsafe_code)]
#![cfg(test)]
//! Error recovery tests for fuzz-malformed journal records.
//!
//! Each test constructs a valid encoded journal record, mutates one byte (or
//! a small targeted region) to simulate a specific fuzz-class corruption, and
//! asserts that the decode/replay pipeline returns the typed `JournalError`
//! variant that the storage contract promises for that mutation class.

#[path = "error_recovery_tests/decode_tests.rs"]
mod decode_tests;
#[path = "error_recovery_tests/replay_tests.rs"]
mod replay_tests;
#[path = "error_recovery_tests/sanity_tests.rs"]
mod sanity_tests;

use crate::JournalEvent;
use crate::codec::encode_journal_event_record;
use vb_core::RunId;

/// Build a minimal valid journal event (RunAccepted at seq=0).
fn sample_event() -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: crate::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x11; 32]),
    }
}

/// Encode a valid record and return the bytes for mutation.
fn encoded_record() -> Vec<u8> {
    encode_journal_event_record(&sample_event()).expect("valid event must encode cleanly")
}

/// Mutate one byte at `offset` (wraps via XOR with 0xFF for a deterministic but
/// content-changing flip).
fn flip_byte(bytes: &mut [u8], offset: usize) {
    if let Some(b) = bytes.get_mut(offset) {
        *b ^= 0xFF;
    }
}

/// Mutate 4 bytes at `offset` to a sentinel that won't match any legitimate
/// header field.
fn scribble_u32(bytes: &mut [u8], offset: usize) {
    let sentinel = 0xDE_AD_BE_EF_u32.to_le_bytes();
    for (i, slot) in bytes.iter_mut().enumerate().skip(offset).take(4) {
        *slot = sentinel[i - offset];
    }
}
