#![forbid(unsafe_code)]
//! Bounded compiled slug types for yield-budget-constrained workflow execution.

use crate::workflow::PathSegment;
use crate::workflow::admission_kernel::{
    AdmissionKernelError, accumulate_yield_cost, validate_admission_summary,
};
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
    slugs: Box<[YbBoundedSlug]>,
    /// Remaining yield budget after decoding and validation.
    remaining_budget: u64,
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

fn checked_total_yield_cost(slugs: &[YbBoundedSlug]) -> Result<u64, SlugParseError> {
    let mut total = 0_u64;
    for slug in slugs {
        total = accumulate_yield_cost(total, slug.yield_cost)
            .map_err(|_| SlugParseError::YieldCostOverflow)?;
    }
    Ok(total)
}

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
    use crate::ids::SymbolId;

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
