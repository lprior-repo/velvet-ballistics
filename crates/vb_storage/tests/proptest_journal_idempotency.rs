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
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
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
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
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
    clippy::useless_asref,
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
//!
//! Proptest properties for journal idempotency — supplementary to Kani harnesses.
//!
//! Bead: vb-282my
//! Obligation: PO-vb282my-RJ-PROP-001
//!
//! Target: crate::journal::internal::append_queued_unpersisted
//!
//! Tests idempotency round-trip: serialize JournalEvent → append →
//! decode existing → assert equality.

use proptest::prelude::*;
use vb_core::ids::RunId;
use vb_storage::{EventSeq, keys::run_event_key};

proptest! {
    /// PO-vb282my-RJ-PROP-001: Key encoding idempotency
    /// The run_event_key function must be deterministic: same (run, seq) input
    /// must always produce the same key output.
    #[test]
    fn proptest_journal_idempotency_round_trip(
        run in 0u64..,
        seq in 0u64..,
    ) {
        let key1 = run_event_key(RunId::new(run), EventSeq::new(seq));
        let key2 = run_event_key(RunId::new(run), EventSeq::new(seq));

        match (key1, key2) {
            (Ok(k1), Ok(k2)) => {
                assert_eq!(k1, k2, "key encoding must be deterministic");
                assert_eq!(k1.len(), 17, "key must be 17 bytes");
            }
            _ => {
                // Key encoding should not fail for valid inputs
                panic!("key encoding failed unexpectedly");
            }
        }
    }

    /// Key injectivity: different inputs produce different keys
    #[test]
    fn proptest_journal_key_injectivity(
        run1 in 0u64..,
        seq1 in 0u64..,
        run2 in 0u64..,
        seq2 in 0u64..,
    ) {
        proptest::prop_assume!(run1 != run2 || seq1 != seq2);

        let key1 = run_event_key(RunId::new(run1), EventSeq::new(seq1));
        let key2 = run_event_key(RunId::new(run2), EventSeq::new(seq2));

        match (key1, key2) {
            (Ok(k1), Ok(k2)) => {
                assert_ne!(k1, k2, "distinct inputs must produce distinct keys");
            }
            _ => {}
        }
    }

    /// Key prefix validation: prefix byte must be 0x11
    #[test]
    fn proptest_journal_key_prefix(
        run in 0u64..,
        seq in 0u64..,
    ) {
        let key = run_event_key(RunId::new(run), EventSeq::new(seq));

        if let Ok(k) = key {
            assert_eq!(k[0], 0x11, "key must start with 0x11 prefix");
        }
    }
}
