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
#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::workflow::compiled_query::QueryParseError;
use vb_core::workflow::compiled_slug::SlugParseError;

#[path = "vb_ajc40_property_common.rs"]
mod common;

use common::{assert_query_error, assert_slug_error, query, query_payload, slug, slug_payload};

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, .. ProptestConfig::default() })]

    #[test]
    fn po_022_slug_and_query_reject_underdeclared_totals(actual in 1u64..1_000_000) {
        let declared = actual - 1;
        let slug_payload = slug_payload(vec![slug(0, actual)], declared);
        assert_slug_error(
            &slug_payload,
            actual,
            SlugParseError::TotalYieldCostMismatch { declared, recomputed: actual },
        )?;

        let query_payload = query_payload(vec![query(0, actual)], declared);
        assert_query_error(
            &query_payload,
            actual,
            QueryParseError::TotalYieldCostMismatch { declared, recomputed: actual },
        )?;
    }

    #[test]
    fn po_022_slug_and_query_reject_overdeclared_totals(actual in 0u64..1_000_000) {
        let declared = actual + 1;
        let slug_payload = slug_payload(vec![slug(0, actual)], declared);
        assert_slug_error(
            &slug_payload,
            declared,
            SlugParseError::TotalYieldCostMismatch { declared, recomputed: actual },
        )?;

        let query_payload = query_payload(vec![query(0, actual)], declared);
        assert_query_error(
            &query_payload,
            declared,
            QueryParseError::TotalYieldCostMismatch { declared, recomputed: actual },
        )?;
    }
}

#[test]
fn po_022_slug_and_query_reject_checked_add_overflow() -> Result<(), TestCaseError> {
    let slug_payload = slug_payload(vec![slug(0, u64::MAX), slug(0, 1)], 0);
    assert_slug_error(&slug_payload, u64::MAX, SlugParseError::YieldCostOverflow)?;

    let query_payload = query_payload(vec![query(0, u64::MAX), query(0, 1)], 0);
    assert_query_error(&query_payload, u64::MAX, QueryParseError::YieldCostOverflow)?;
    Ok(())
}
