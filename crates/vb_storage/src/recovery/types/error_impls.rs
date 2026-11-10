#![forbid(unsafe_code)]

use super::RecoveryError;

impl core::fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Journal(inner) => {
                write!(f, "journal error during recovery: {inner}")
            }
            Self::WorkflowSourceDigestMismatch { expected, found } => {
                write!(
                    f,
                    "workflow source digest mismatch: expected {expected:?}, found {found:?}"
                )
            }
            Self::CompiledIrDigestMismatch { expected, found } => {
                write!(
                    f,
                    "compiled IR digest mismatch: expected {expected:?}, found {found:?}"
                )
            }
            Self::ActionAbiMismatch { action_id } => {
                write!(f, "action ABI digest mismatch for action {action_id:?}")
            }
            Self::PolicyDigestMismatch { step } => {
                write!(f, "policy digest mismatch for step {step:?}")
            }
            Self::NonIdempotentActionBlocked { action, step } => {
                write!(
                    f,
                    "non-idempotent action {action:?} at step {step:?} cannot be re-executed during recovery"
                )
            }
            Self::ReplayDivergence { step, detail } => {
                write!(f, "replay divergence at step {step:?}: {detail}")
            }
            Self::SlotTaintReadFailed { slot } => {
                write!(f, "slot taint read_taint failed for slot {slot:?}")
            }
            Self::CorruptSlotTaint { slot } => {
                write!(f, "slot taint metadata corrupt for slot {slot:?}")
            }
            Self::NoRecoveryData { run } => {
                write!(f, "no recovery data found for run {run:?}")
            }
            Self::CorruptSnapshot { run, seq } => {
                write!(f, "snapshot corrupt for run {run:?} at seq {seq:?}")
            }
            Self::MissingSnapshot { run, seq } => {
                write!(f, "no snapshot found for run {run:?} at seq {seq:?}")
            }
            Self::TerminalStateMismatch { expected, found } => {
                write!(
                    f,
                    "recovery terminal state mismatch: expected {expected:?}, found {found:?}"
                )
            }
            Self::FrameDimensionOverflow { run } => {
                write!(f, "recovery frame dimension overflow for run {run:?}")
            }
            Self::UnsupportedFrameSeed { run, reason } => {
                write!(
                    f,
                    "recovery frame seed is not resumable for run {run:?}: {reason}"
                )
            }
            Self::ArtifactNotFound { digest } => {
                write!(f, "artifact not found for recovery: {digest:?}")
            }
            Self::ArtifactDecodeFailed => {
                write!(f, "artifact decode failed during recovery")
            }
        }
    }
}

impl std::error::Error for RecoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Journal(inner) => Some(inner),
            Self::WorkflowSourceDigestMismatch { .. }
            | Self::CompiledIrDigestMismatch { .. }
            | Self::ActionAbiMismatch { .. }
            | Self::PolicyDigestMismatch { .. }
            | Self::NonIdempotentActionBlocked { .. }
            | Self::ReplayDivergence { .. }
            | Self::SlotTaintReadFailed { .. }
            | Self::CorruptSlotTaint { .. }
            | Self::NoRecoveryData { .. }
            | Self::CorruptSnapshot { .. }
            | Self::MissingSnapshot { .. }
            | Self::TerminalStateMismatch { .. }
            | Self::FrameDimensionOverflow { .. }
            | Self::UnsupportedFrameSeed { .. }
            | Self::ArtifactNotFound { .. }
            | Self::ArtifactDecodeFailed => None,
        }
    }
}

// Manual `PartialEq, Eq` impl: `RecoveryError::Journal(_)` wraps
// `JournalError`, whose variants transitively contain `fjall::Error`
// and `std::io::Error` (neither implements `PartialEq`), so a
// `#[derive(PartialEq, Eq)]` is infeasible without restructuring the
// broader storage error surface. Two `Journal(_)` payloads are treated
// as never equal (the inner `JournalError` cannot be compared); every
// other variant is compared field-by-field. This adds the trait
// capability without changing any variant, field, `Display`, `From`,
// or `Error` semantic on `RecoveryError`.
impl PartialEq for RecoveryError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Journal(_), Self::Journal(_)) => false,
            (
                Self::WorkflowSourceDigestMismatch {
                    expected: le,
                    found: lf,
                },
                Self::WorkflowSourceDigestMismatch {
                    expected: re,
                    found: rf,
                },
            ) => le == re && lf == rf,
            (
                Self::CompiledIrDigestMismatch {
                    expected: le,
                    found: lf,
                },
                Self::CompiledIrDigestMismatch {
                    expected: re,
                    found: rf,
                },
            ) => le == re && lf == rf,
            (
                Self::ActionAbiMismatch { action_id: la },
                Self::ActionAbiMismatch { action_id: ra },
            ) => la == ra,
            (Self::PolicyDigestMismatch { step: ls }, Self::PolicyDigestMismatch { step: rs }) => {
                ls == rs
            }
            (
                Self::NonIdempotentActionBlocked {
                    action: la,
                    step: ls,
                },
                Self::NonIdempotentActionBlocked {
                    action: ra,
                    step: rs,
                },
            ) => la == ra && ls == rs,
            (
                Self::ReplayDivergence {
                    step: ls,
                    detail: ld,
                },
                Self::ReplayDivergence {
                    step: rs,
                    detail: rd,
                },
            ) => ls == rs && ld == rd,
            (Self::SlotTaintReadFailed { slot: ls }, Self::SlotTaintReadFailed { slot: rs }) => {
                ls == rs
            }
            (Self::CorruptSlotTaint { slot: ls }, Self::CorruptSlotTaint { slot: rs }) => ls == rs,
            (Self::NoRecoveryData { run: lr }, Self::NoRecoveryData { run: rr }) => lr == rr,
            (
                Self::CorruptSnapshot { run: lr, seq: ls },
                Self::CorruptSnapshot { run: rr, seq: rs },
            ) => lr == rr && ls == rs,
            (
                Self::MissingSnapshot { run: lr, seq: ls },
                Self::MissingSnapshot { run: rr, seq: rs },
            ) => lr == rr && ls == rs,
            (
                Self::TerminalStateMismatch {
                    expected: le,
                    found: lf,
                },
                Self::TerminalStateMismatch {
                    expected: re,
                    found: rf,
                },
            ) => le == re && lf == rf,
            (
                Self::FrameDimensionOverflow { run: lr },
                Self::FrameDimensionOverflow { run: rr },
            ) => lr == rr,
            (
                Self::UnsupportedFrameSeed {
                    run: lr,
                    reason: ld,
                },
                Self::UnsupportedFrameSeed {
                    run: rr,
                    reason: rd,
                },
            ) => lr == rr && ld == rd,
            (Self::ArtifactNotFound { digest: ld }, Self::ArtifactNotFound { digest: rd }) => {
                ld == rd
            }
            (Self::ArtifactDecodeFailed, Self::ArtifactDecodeFailed) => true,
            _ => false,
        }
    }
}
