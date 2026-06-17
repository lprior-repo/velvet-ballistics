#![forbid(unsafe_code)]
//! Bounded compiled slug codec for yield-budget-constrained workflow execution.

use super::types::{CompiledSlugs, SlugParseError, YbBoundedSlugs};
use super::validation::validate_compiled_slugs;

/// Decodes compiled slugs from bytes and validates them against a yield budget.
///
/// Deserializes the `CompiledSlugs` structure using `postcard::from_bytes` and
/// recomputes and verifies the accumulated yield cost before checking it against
/// `max_yield_budget`.
/// Each slug's path depth is also validated against `MAX_SLUG_PATH_SEGMENTS`.
///
/// # Errors
///
/// Returns `SlugParseError::Decode` if the byte sequence is not valid
/// postcard-encoded `CompiledSlugs`. Returns `SlugParseError::YbBudgetExceeded`
/// if the total yield cost of all slugs exceeds `max_yield_budget`. Returns
/// `SlugParseError::SlugPathTooDeep` if any slug exceeds the path depth limit.
/// Returns `SlugParseError::TooManySlugs` if the number of slugs exceeds
/// `MAX_SLUGS_PER_WORKFLOW`. Returns `SlugParseError::YieldCostOverflow` if the
/// recomputed yield sum overflows `u64`. Returns
/// `SlugParseError::TotalYieldCostMismatch` if the serialized total differs from
/// the recomputed sum.
#[allow(clippy::needless_pass_by_value)]
pub fn from_bytes_compiled_slugs(
    bytes: &[u8],
    max_yield_budget: u64,
) -> Result<YbBoundedSlugs, SlugParseError> {
    let compiled: CompiledSlugs = postcard::from_bytes(bytes).map_err(SlugParseError::Decode)?;
    validate_compiled_slugs(compiled, max_yield_budget)
}

#[cfg(test)]
mod tests {
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
    unused_variables,
)]

    use super::*;
    use crate::workflow::compiled_slug::types::YbBoundedSlug;

    fn encode_slugs(payload: &CompiledSlugs) -> Result<Vec<u8>, String> {
        postcard::to_allocvec(payload).map_err(|err| format!("slug postcard encode failed: {err}"))
    }

    fn unit_slug(cost: u64) -> YbBoundedSlug {
        YbBoundedSlug {
            path: Vec::new().into_boxed_slice(),
            yield_cost: cost,
        }
    }

    #[test]
    fn compiled_slugs_reject_underdeclared_total() -> Result<(), String> {
        let payload = CompiledSlugs {
            slugs: vec![unit_slug(7), unit_slug(11)].into(),
            total_yield_cost: 17,
        };
        let bytes = encode_slugs(&payload)?;

        let result = from_bytes_compiled_slugs(&bytes, 18);

        assert_eq!(
            result,
            Err(SlugParseError::TotalYieldCostMismatch {
                declared: 17,
                recomputed: 18,
            })
        );
        Ok(())
    }

    #[test]
    fn compiled_slugs_reject_overdeclared_total() -> Result<(), String> {
        let payload = CompiledSlugs {
            slugs: vec![unit_slug(7), unit_slug(11)].into(),
            total_yield_cost: 19,
        };
        let bytes = encode_slugs(&payload)?;

        let result = from_bytes_compiled_slugs(&bytes, 19);

        assert_eq!(
            result,
            Err(SlugParseError::TotalYieldCostMismatch {
                declared: 19,
                recomputed: 18,
            })
        );
        Ok(())
    }

    #[test]
    fn compiled_slugs_reject_yield_sum_overflow() -> Result<(), String> {
        let payload = CompiledSlugs {
            slugs: vec![unit_slug(u64::MAX), unit_slug(1)].into(),
            total_yield_cost: 0,
        };
        let bytes = encode_slugs(&payload)?;

        let result = from_bytes_compiled_slugs(&bytes, u64::MAX);

        assert_eq!(result, Err(SlugParseError::YieldCostOverflow));
        Ok(())
    }

    #[test]
    fn compiled_slugs_accept_exact_total_with_remaining_budget() -> Result<(), String> {
        let payload = CompiledSlugs {
            slugs: vec![unit_slug(7), unit_slug(11)].into(),
            total_yield_cost: 18,
        };
        let bytes = encode_slugs(&payload)?;

        let result = from_bytes_compiled_slugs(&bytes, 25);

        match result {
            Ok(admitted) => {
                assert_eq!(admitted.len(), 2);
                assert_eq!(admitted.remaining_budget(), 7);
                Ok(())
            }
            Err(err) => Err(format!("compiled slug admission failed: {err}")),
        }
    }

    #[test]
    fn compiled_slugs_keep_empty_path_root_accessor_valid() -> Result<(), String> {
        let payload = CompiledSlugs {
            slugs: vec![unit_slug(4)].into(),
            total_yield_cost: 4,
        };
        let bytes = encode_slugs(&payload)?;

        let result = from_bytes_compiled_slugs(&bytes, 4);

        match result {
            Ok(admitted) => {
                assert_eq!(admitted.len(), 1);
                assert!(matches!(admitted.slugs().first(), Some(item) if item.path_depth() == 0));
                assert_eq!(admitted.remaining_budget(), 0);
                Ok(())
            }
            Err(err) => Err(format!("compiled slug admission failed: {err}")),
        }
    }
}
