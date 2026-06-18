#![forbid(unsafe_code)]
//! Bounded compiled slug types for yield-budget-constrained workflow execution.

use crate::workflow::admission_kernel::accumulate_yield_cost;
use crate::workflow::PathSegment;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Maximum number of slugs permitted in a single workflow admission.
pub const MAX_SLUGS_PER_WORKFLOW: usize = 65_535;

/// Maximum number of path segments in a single slug.
pub const MAX_SLUG_PATH_SEGMENTS: usize = 16;

/// A compiled slug with a yield cost and a bounded accessor path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YbBoundedSlug {
    /// Accessor path from root slot to the target value.
    pub path: Box<[PathSegment]>,
    /// Computational cost charged against the yield budget upon execution.
    pub yield_cost: u64,
}

impl YbBoundedSlug {
    /// Validates the slug path depth against the hard limit.
    pub fn path_depth(&self) -> usize {
        self.path.len()
    }

    /// Returns `true` if the slug path exceeds the maximum allowed depth.
    #[must_use]
    pub fn is_path_too_deep(&self) -> bool {
        self.path.len() > MAX_SLUG_PATH_SEGMENTS
    }
}

/// The serialized format for a collection of compiled slugs, used by
/// `from_bytes_compiled_slugs` as the immediate decode target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledSlugs {
    /// Individual compiled slugs.
    pub slugs: Box<[YbBoundedSlug]>,
    /// Explicit sum of all yield costs across slugs, verified at decode time.
    pub total_yield_cost: u64,
}

/// Container for decoded slugs with remaining budget tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YbBoundedSlugs {
    /// Decoded slugs with bounded paths and yield costs.
    pub(crate) slugs: Box<[YbBoundedSlug]>,
    /// Remaining yield budget after decoding and validation.
    pub(crate) remaining_budget: u64,
}

impl YbBoundedSlugs {
    /// Returns a reference to the contained slugs.
    #[must_use]
    pub fn slugs(&self) -> &[YbBoundedSlug] {
        &self.slugs
    }

    /// Returns the remaining yield budget.
    #[must_use]
    pub const fn remaining_budget(&self) -> u64 {
        self.remaining_budget
    }

    /// Returns the number of slugs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slugs.len()
    }

    /// Returns `true` if there are no slugs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slugs.is_empty()
    }
}

/// Slug parse failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum SlugParseError {
    /// Deserialization failed.
    #[error("slug deserialization failed: {0}")]
    Decode(#[source] postcard::Error),
    /// The accumulated yield cost of all slugs exceeds the maximum budget.
    #[error("YB budget exceeded: total {total} exceeds max {max}")]
    YbBudgetExceeded {
        /// Sum of all slug yield costs.
        total: u64,
        /// Caller-supplied maximum budget.
        max: u64,
    },
    /// A slug path exceeds the maximum allowed depth.
    #[error("slug path too deep: {depth} segments (max {max})")]
    SlugPathTooDeep {
        /// Actual path depth.
        depth: usize,
        /// Maximum allowed depth.
        max: usize,
    },
    /// The number of slugs exceeds the hard limit.
    #[error("too many slugs: {count} (max {max})")]
    TooManySlugs {
        /// Actual slug count.
        count: usize,
        /// Maximum allowed slugs.
        max: usize,
    },
    /// Recomputing the sum of all per-slug yield costs overflowed `u64`.
    #[error("slug yield cost sum overflowed u64")]
    YieldCostOverflow,
    /// Serialized total yield cost does not match the recomputed sum.
    #[error("slug total yield cost mismatch: declared {declared}, recomputed {recomputed}")]
    TotalYieldCostMismatch {
        /// Serialized total yield cost.
        declared: u64,
        /// Recomputed sum of per-slug yield costs.
        recomputed: u64,
    },
}

pub(crate) fn checked_total_yield_cost(slugs: &[YbBoundedSlug]) -> Result<u64, SlugParseError> {
    let mut total = 0_u64;
    for slug in slugs {
        total = accumulate_yield_cost(total, slug.yield_cost)
            .map_err(|_| SlugParseError::YieldCostOverflow)?;
    }
    Ok(total)
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

    use crate::ids::SymbolId;
    use crate::workflow::PathSegment;

    use super::*;

    #[test]
    fn slug_path_depth_validation() {
        let shallow = YbBoundedSlug {
            path: vec![PathSegment::Field(SymbolId::new(1)), PathSegment::Index(0)].into(),
            yield_cost: 10,
        };
        assert!(!shallow.is_path_too_deep());

        let deep: Box<[PathSegment]> = (0..20)
            .map(|i| PathSegment::Field(SymbolId::new(i as u32)))
            .collect();
        let deep_slug = YbBoundedSlug {
            path: deep,
            yield_cost: 10,
        };
        assert!(deep_slug.is_path_too_deep());
    }

    #[test]
    fn slug_is_empty_and_len() {
        let empty_slugs: YbBoundedSlugs = YbBoundedSlugs {
            slugs: vec![].into(),
            remaining_budget: 100,
        };
        assert!(empty_slugs.is_empty());
        assert_eq!(empty_slugs.len(), 0);
        assert_eq!(empty_slugs.remaining_budget(), 100);
    }

    #[test]
    fn slug_len_and_remaining_budget() {
        let slug = YbBoundedSlug {
            path: vec![PathSegment::Field(SymbolId::new(1))].into(),
            yield_cost: 30,
        };
        let bounded_slugs = YbBoundedSlugs {
            slugs: vec![slug].into(),
            remaining_budget: 70,
        };
        assert!(!bounded_slugs.is_empty());
        assert_eq!(bounded_slugs.len(), 1);
        assert_eq!(bounded_slugs.remaining_budget(), 70);
    }

    #[test]
    fn slug_parse_error_display() {
        let err = SlugParseError::YbBudgetExceeded {
            total: 100,
            max: 50,
        };
        let msg = err.to_string();
        assert!(msg.contains("YB budget exceeded"));
        assert!(msg.contains("100"));
        assert!(msg.contains("50"));
    }

    #[test]
    fn slug_parse_error_too_many_slugs() {
        let err = SlugParseError::TooManySlugs {
            count: 70000,
            max: 65535,
        };
        let msg = err.to_string();
        assert!(msg.contains("too many slugs"));
        assert!(msg.contains("70000"));
    }

    #[test]
    fn slug_parse_error_path_too_deep() {
        let err = SlugParseError::SlugPathTooDeep { depth: 20, max: 16 };
        let msg = err.to_string();
        assert!(msg.contains("slug path too deep"));
        assert!(msg.contains("20"));
    }
}
