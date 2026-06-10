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
