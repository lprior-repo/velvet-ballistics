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
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::enum_variant_names,
    clippy::manual_contains,
    clippy::if_same_then_else,
    clippy::multiple_bound_locations,
    clippy::identity_op,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
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

use proptest::prelude::*;
use vb_core::RunId;
use vb_runtime::shard::{CompletionWatermark, CompletionWatermarkError};

#[test]
fn completing_prefix_in_order_drains_each_sequence() {
    let run = RunId::new(11);
    let mut watermark = CompletionWatermark::new(run, 4, 4);

    let first = watermark.complete(run, 1);
    assert_eq!(
        first.map(|drain| (drain.boundary, drain.drained.into_vec())),
        Ok((1, vec![1]))
    );

    let second = watermark.complete(run, 2);
    assert_eq!(
        second.map(|drain| (drain.boundary, drain.drained.into_vec())),
        Ok((2, vec![2]))
    );
}

#[test]
fn out_of_order_completion_waits_for_gap_then_drains_prefix() {
    let run = RunId::new(12);
    let mut watermark = CompletionWatermark::new(run, 4, 4);

    let gap = watermark.complete(run, 2);
    assert_eq!(
        gap.map(|drain| (drain.boundary, drain.drained.into_vec())),
        Ok((0, Vec::new()))
    );
    assert_eq!(watermark.pending_len(), 1);

    let prefix = watermark.complete(run, 1);
    assert_eq!(
        prefix.map(|drain| (drain.boundary, drain.drained.into_vec())),
        Ok((2, vec![1, 2]))
    );
    assert_eq!(watermark.pending_len(), 0);
}

#[test]
fn duplicate_completion_does_not_double_drain() {
    let run = RunId::new(13);
    let mut watermark = CompletionWatermark::new(run, 4, 4);

    assert!(watermark.complete(run, 1).is_ok());
    assert_eq!(
        watermark.complete(run, 1),
        Err(CompletionWatermarkError::Duplicate { seq: 1 })
    );
    assert_eq!(watermark.boundary(), 1);
}

#[test]
fn invalid_sequence_and_capacity_return_typed_errors() {
    let run = RunId::new(14);
    let mut watermark = CompletionWatermark::new(run, 1, 1);

    assert_eq!(
        watermark.complete(run, 0),
        Err(CompletionWatermarkError::InvalidSequence { seq: 0 })
    );
    assert_eq!(
        watermark.register_waiter(0),
        Err(CompletionWatermarkError::InvalidSequence { seq: 0 })
    );
    assert_eq!(watermark.register_waiter(2), Ok(()));
    assert_eq!(
        watermark.register_waiter(3),
        Err(CompletionWatermarkError::QueueFull { capacity: 1 })
    );
}

#[test]
fn large_boundary_sequence_does_not_overflow() {
    let run = RunId::new(15);
    let mut watermark = CompletionWatermark::from_boundary(run, u64::MAX - 1, 1, 1);
    let result = watermark.complete(run, u64::MAX);

    assert_eq!(
        result.map(|drain| (drain.boundary, drain.drained.into_vec())),
        Ok((u64::MAX, vec![u64::MAX]))
    );
    assert_eq!(
        watermark.complete(run, u64::MAX),
        Err(CompletionWatermarkError::Duplicate { seq: u64::MAX })
    );
}

proptest! {
    #[test]
    fn completion_watermark_boundary_never_decreases(seq_values in prop::collection::vec(1_u64..=8, 1..16)) {
        let run = RunId::new(16);
        let mut watermark = CompletionWatermark::new(run, 8, 8);
        let mut previous = watermark.boundary();

        for seq in seq_values {
            let _ = watermark.complete(run, seq);
            prop_assert!(watermark.boundary() >= previous);
            previous = watermark.boundary();
        }
    }
}
