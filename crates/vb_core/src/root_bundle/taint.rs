//! Phase 8 taint validation.
//!
//! Validates that each root's declared taint is consistent with the
//! graph-computed descendant taint. Uses the actual join_taint function
//! from value.rs; does not assume numeric ordering.

use crate::ids::SlotIdx;
use crate::root_bundle::errors::OwnedRootBundleError;
use crate::value::{SlotValue, Taint, join_taint};

use super::graph::Graph;

/// Validate root taints against graph-computed descendant taints.
pub(super) fn validate(
    roots: &[(SlotIdx, SlotValue, Taint)],
    graph: &Graph,
) -> Result<(), OwnedRootBundleError> {
    for (slot, value, actual) in roots {
        let cached = graph.descendant_taint(*value)?;
        let expected = join_taint(*actual, cached);
        if expected != *actual {
            return Err(OwnedRootBundleError::TaintMismatch {
                slot: *slot,
                expected,
                actual: *actual,
            });
        }
    }
    Ok(())
}
