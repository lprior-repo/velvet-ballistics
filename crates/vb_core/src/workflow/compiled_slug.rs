#![forbid(unsafe_code)]
//! Bounded compiled slug types for yield-budget-constrained workflow execution.

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
}

/// Decodes compiled slugs from bytes and validates them against a yield budget.
///
/// Deserializes the `CompiledSlugs` structure using `postcard::from_bytes` and
/// checks that the accumulated yield cost does not exceed `max_yield_budget`.
/// Each slug's path depth is also validated against `MAX_SLUG_PATH_SEGMENTS`.
///
/// # Errors
///
/// Returns `SlugParseError::Decode` if the byte sequence is not valid
/// postcard-encoded `CompiledSlugs`. Returns `SlugParseError::YbBudgetExceeded`
/// if the total yield cost of all slugs exceeds `max_yield_budget`. Returns
/// `SlugParseError::SlugPathTooDeep` if any slug exceeds the path depth limit.
/// Returns `SlugParseError::TooManySlugs` if the number of slugs exceeds
/// `MAX_SLUGS_PER_WORKFLOW`.
#[allow(clippy::needless_pass_by_value)]
pub fn from_bytes_compiled_slugs(
    bytes: &[u8],
    max_yield_budget: u64,
) -> Result<YbBoundedSlugs, SlugParseError> {
    let compiled: CompiledSlugs =
        postcard::from_bytes(bytes).map_err(SlugParseError::Decode)?;

    if compiled.slugs.len() > MAX_SLUGS_PER_WORKFLOW {
        return Err(SlugParseError::TooManySlugs {
            count: compiled.slugs.len(),
            max: MAX_SLUGS_PER_WORKFLOW,
        });
    }

    for slug in compiled.slugs.iter() {
        if slug.is_path_too_deep() {
            return Err(SlugParseError::SlugPathTooDeep {
                depth: slug.path_depth(),
                max: MAX_SLUG_PATH_SEGMENTS,
            });
        }
    }

    if compiled.total_yield_cost > max_yield_budget {
        return Err(SlugParseError::YbBudgetExceeded {
            total: compiled.total_yield_cost,
            max: max_yield_budget,
        });
    }

    let remaining = max_yield_budget
        .checked_sub(compiled.total_yield_cost)
        .unwrap_or(0);

    Ok(YbBoundedSlugs {
        slugs: compiled.slugs,
        remaining_budget: remaining,
    })
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
        let err = SlugParseError::YbBudgetExceeded { total: 100, max: 50 };
        let msg = err.to_string();
        assert!(msg.contains("YB budget exceeded"));
        assert!(msg.contains("100"));
        assert!(msg.contains("50"));
    }

    #[test]
    fn slug_parse_error_too_many_slugs() {
        let err = SlugParseError::TooManySlugs { count: 70000, max: 65535 };
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
