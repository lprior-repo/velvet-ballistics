#![forbid(unsafe_code)]
//! Small digest comparison helpers for recovery checks.

use vb_core::{ActionId, StepIdx, WorkflowDigest};

/// Returns whether two [`WorkflowDigest`] values carry the same bytes.
///
/// SJ-004: the previous implementation destructured both digests into 64
/// named bindings and emitted 32 short-circuiting `==` comparisons by hand.
/// The compiler lowers array equality to `memcmp`, and the short-circuit form
/// was neither readable nor constant-time. Direct `PartialEq` (already
/// derived on `WorkflowDigest`) is correct, total, and just as fast.
///
/// If a true constant-time comparison is ever required, switch to
/// `subtle::ConstantTimeEq` from the `subtle` crate — do not hand-roll.
#[must_use]
pub(crate) fn workflow_digest_bytes_equal(left: WorkflowDigest, right: WorkflowDigest) -> bool {
    left == right
}

pub(crate) fn first_action_abi_mismatch(
    entries: &[(ActionId, WorkflowDigest, WorkflowDigest)],
) -> Option<(ActionId, WorkflowDigest, WorkflowDigest)> {
    entries.iter().find_map(|(action_id, expected, found)| {
        if workflow_digest_bytes_equal(*expected, *found) {
            None
        } else {
            Some((*action_id, *expected, *found))
        }
    })
}

pub(crate) fn first_policy_mismatch(
    entries: &[(StepIdx, WorkflowDigest, WorkflowDigest)],
) -> Option<(StepIdx, WorkflowDigest, WorkflowDigest)> {
    entries.iter().find_map(|(step, expected, found)| {
        if workflow_digest_bytes_equal(*expected, *found) {
            None
        } else {
            Some((*step, *expected, *found))
        }
    })
}
