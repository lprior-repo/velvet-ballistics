#![forbid(unsafe_code)]
//! Branch table validation helpers.
//!
//! A branch table must contain at least one branch or an otherwise clause.

use crate::workflow::WorkflowError;

/// Validates that a branch table has at least one target (branch or otherwise).
///
/// Generic over `T` so callers can pass `Option<StepIdx>` or any other
/// `Option` without conversion.
pub(crate) fn validate_branch_route<T>(
    branch_count: usize,
    otherwise: Option<T>,
) -> Result<(), WorkflowError> {
    if branch_count == 0 && otherwise.is_none() {
        Err(WorkflowError::EmptyBranchTable)
    } else {
        Ok(())
    }
}
