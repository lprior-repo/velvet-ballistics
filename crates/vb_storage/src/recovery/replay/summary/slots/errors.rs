#![forbid(unsafe_code)]
//! Replay error mapping to recovery errors.
//!
//! Provides:
//! - `replay_error_to_recovery` — ReplayError → RecoveryError mapping

use crate::recovery::RecoveryError;
use vb_core::StepIdx;
use vb_core::replay::ReplayError;

pub(crate) fn replay_error_to_recovery(error: ReplayError) -> RecoveryError {
    match error {
        ReplayError::StepNotFound { step } => RecoveryError::ReplayDivergence {
            step,
            detail: "replay step not found in compiled workflow".to_owned(),
        },
        ReplayError::NonDeterministicStep { step, kind } => RecoveryError::ReplayDivergence {
            step,
            detail: format!("replay blocked by non-deterministic {kind} step"),
        },
        ReplayError::SlotNotAvailable { slot } => RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: format!("replay required unavailable slot {:?}", slot),
        },
        ReplayError::ExpressionEvalFailed { step } => RecoveryError::ReplayDivergence {
            step,
            detail: "replay expression evaluation failed".to_owned(),
        },
        ReplayError::Internal { reason } => RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: reason.to_owned(),
        },
        // `ReplayError` is `#[non_exhaustive]`; unknown variants
        // map to a generic replay divergence error.
        _ => RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: "unknown replay error".to_owned(),
        },
    }
}
