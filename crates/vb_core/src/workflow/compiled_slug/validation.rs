#![forbid(unsafe_code)]
//! Bounded compiled slug validation for yield-budget-constrained workflow execution.

use crate::workflow::admission_kernel::{AdmissionKernelError, validate_admission_summary};

use super::types::{
    CompiledSlugs, MAX_SLUG_PATH_SEGMENTS, MAX_SLUGS_PER_WORKFLOW, SlugParseError, YbBoundedSlug,
    YbBoundedSlugs, checked_total_yield_cost,
};

/// Validates a decoded slug count against the workflow admission limit.
///
/// # Errors
///
/// Returns `SlugParseError::TooManySlugs` when `count` exceeds
/// `MAX_SLUGS_PER_WORKFLOW`.
pub fn validate_compiled_slug_count(count: usize) -> Result<(), SlugParseError> {
    if count > MAX_SLUGS_PER_WORKFLOW {
        return Err(SlugParseError::TooManySlugs {
            count,
            max: MAX_SLUGS_PER_WORKFLOW,
        });
    }

    Ok(())
}

/// Validates the summarized post-decode slug admission facts.
///
/// # Errors
///
/// Returns the same count, path-depth, total-mismatch, and budget errors as the
/// full decoded-slug validator after total recomputation succeeds.
pub fn validate_compiled_slug_summary(
    count: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_path_depth: usize,
    max_yield_budget: u64,
) -> Result<u64, SlugParseError> {
    validate_slug_admission_kernel(
        count,
        recomputed_total,
        declared_total_yield_cost,
        max_path_depth,
        max_yield_budget,
    )
    .map_err(|error| {
        slug_summary_error(
            error,
            count,
            recomputed_total,
            declared_total_yield_cost,
            max_path_depth,
            max_yield_budget,
        )
    })
}

fn validate_slug_admission_kernel(
    count: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_path_depth: usize,
    max_yield_budget: u64,
) -> Result<u64, AdmissionKernelError> {
    validate_admission_summary(
        count,
        MAX_SLUGS_PER_WORKFLOW,
        max_path_depth,
        MAX_SLUG_PATH_SEGMENTS,
        recomputed_total,
        declared_total_yield_cost,
        max_yield_budget,
    )
}

fn slug_summary_error(
    error: AdmissionKernelError,
    count: usize,
    recomputed_total: u64,
    declared_total_yield_cost: u64,
    max_path_depth: usize,
    max_yield_budget: u64,
) -> SlugParseError {
    match error {
        AdmissionKernelError::TooManyItems => SlugParseError::TooManySlugs {
            count,
            max: MAX_SLUGS_PER_WORKFLOW,
        },
        AdmissionKernelError::PathTooDeep => SlugParseError::SlugPathTooDeep {
            depth: max_path_depth,
            max: MAX_SLUG_PATH_SEGMENTS,
        },
        AdmissionKernelError::TotalYieldCostMismatch => SlugParseError::TotalYieldCostMismatch {
            declared: declared_total_yield_cost,
            recomputed: recomputed_total,
        },
        AdmissionKernelError::YieldBudgetExceeded => SlugParseError::YbBudgetExceeded {
            total: recomputed_total,
            max: max_yield_budget,
        },
    }
}

fn max_slug_path_depth(slugs: &[YbBoundedSlug]) -> usize {
    let mut max_depth = 0_usize;
    for slug in slugs {
        max_depth = max_depth.max(slug.path_depth());
    }
    max_depth
}

/// Validates decoded slug parts and returns the remaining budget.
///
/// This lower-level seam avoids ownership transfer so Kani can verify the
/// post-decode admission contract over bounded fixed arrays.
///
/// # Errors
///
/// Returns the same structural, total, and budget errors as
/// `validate_compiled_slugs`.
pub fn validate_compiled_slug_parts(
    slugs: &[YbBoundedSlug],
    declared_total_yield_cost: u64,
    max_yield_budget: u64,
) -> Result<u64, SlugParseError> {
    validate_compiled_slug_count(slugs.len())?;
    let max_path_depth = max_slug_path_depth(slugs);
    let recomputed_total = checked_total_yield_cost(slugs)?;
    validate_compiled_slug_summary(
        slugs.len(),
        recomputed_total,
        declared_total_yield_cost,
        max_path_depth,
        max_yield_budget,
    )
}

/// Validates an already-decoded compiled slug payload against structural and
/// yield-budget admission rules.
///
/// This seam is intentionally post-decode and side-effect free so verification
/// tools can prove admission behavior without symbolically executing postcard.
/// `from_bytes_compiled_slugs` delegates here after successful deserialization.
///
/// # Errors
///
/// Returns `SlugParseError::TooManySlugs` if `compiled` exceeds
/// `MAX_SLUGS_PER_WORKFLOW`, `SlugParseError::SlugPathTooDeep` if any path
/// exceeds `MAX_SLUG_PATH_SEGMENTS`, `SlugParseError::YieldCostOverflow` if
/// recomputing totals overflows, `SlugParseError::TotalYieldCostMismatch` if
/// the declared total differs from the recomputed total, and
/// `SlugParseError::YbBudgetExceeded` if the recomputed total exceeds
/// `max_yield_budget`.
pub fn validate_compiled_slugs(
    compiled: CompiledSlugs,
    max_yield_budget: u64,
) -> Result<YbBoundedSlugs, SlugParseError> {
    let remaining =
        validate_compiled_slug_parts(&compiled.slugs, compiled.total_yield_cost, max_yield_budget)?;

    Ok(YbBoundedSlugs {
        slugs: compiled.slugs,
        remaining_budget: remaining,
    })
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
        unused_variables
    )]

    use super::*;

    #[test]
    fn slug_count_helper_accepts_exact_limit_and_rejects_next() {
        assert_eq!(validate_compiled_slug_count(MAX_SLUGS_PER_WORKFLOW), Ok(()));
        assert_eq!(
            validate_compiled_slug_count(MAX_SLUGS_PER_WORKFLOW + 1),
            Err(SlugParseError::TooManySlugs {
                count: MAX_SLUGS_PER_WORKFLOW + 1,
                max: MAX_SLUGS_PER_WORKFLOW,
            })
        );
    }

    #[test]
    fn slug_summary_helper_preserves_error_order_and_remaining_budget() {
        assert_eq!(
            validate_compiled_slug_summary(2, 18, 18, MAX_SLUG_PATH_SEGMENTS, 25),
            Ok(7)
        );
        assert_eq!(
            validate_compiled_slug_summary(2, 18, 17, MAX_SLUG_PATH_SEGMENTS, 25),
            Err(SlugParseError::TotalYieldCostMismatch {
                declared: 17,
                recomputed: 18,
            })
        );
        assert_eq!(
            validate_compiled_slug_summary(2, 18, 18, MAX_SLUG_PATH_SEGMENTS + 1, 25),
            Err(SlugParseError::SlugPathTooDeep {
                depth: MAX_SLUG_PATH_SEGMENTS + 1,
                max: MAX_SLUG_PATH_SEGMENTS,
            })
        );
        assert_eq!(
            validate_compiled_slug_summary(2, 18, 18, MAX_SLUG_PATH_SEGMENTS, 17),
            Err(SlugParseError::YbBudgetExceeded { total: 18, max: 17 })
        );
    }

    #[test]
    fn validate_compiled_slugs_rejects_underdeclared_total_without_decode() {
        let payload = CompiledSlugs {
            slugs: vec![unit_slug(7), unit_slug(11)].into(),
            total_yield_cost: 17,
        };

        let result = validate_compiled_slugs(payload, 18);

        assert_eq!(
            result,
            Err(SlugParseError::TotalYieldCostMismatch {
                declared: 17,
                recomputed: 18,
            })
        );
    }

    fn unit_slug(cost: u64) -> YbBoundedSlug {
        YbBoundedSlug {
            path: Vec::new().into_boxed_slice(),
            yield_cost: cost,
        }
    }

    #[test]
    fn validate_compiled_slugs_accepts_exact_total_without_decode() -> Result<(), String> {
        let payload = CompiledSlugs {
            slugs: vec![unit_slug(7), unit_slug(11)].into(),
            total_yield_cost: 18,
        };

        let result = validate_compiled_slugs(payload, 25);

        match result {
            Ok(admitted) => {
                assert_eq!(admitted.len(), 2);
                assert_eq!(admitted.remaining_budget(), 7);
                Ok(())
            }
            Err(err) => Err(format!("compiled slug admission failed: {err}")),
        }
    }
}
