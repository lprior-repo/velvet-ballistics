#![forbid(unsafe_code)]
//! Recovery types for velvet-ballistics journal.
//!
//! Provides:
//! - Digest mismatch detection types
//! - Recovery state types
//! - Frame seed types for live-frame reconstruction

mod cannot_resume;
mod cannot_resume_reason;
mod digest_check;
mod digests;
mod error_impls;
mod errors;
mod hydration;
mod missing;
mod recovered;
mod replay;
mod runtime;
mod seed;
mod unsupported;

pub use cannot_resume::RecoveryCannotResumeState;
pub use digest_check::DigestCheck;
pub use digests::{
    ActionAbiDigestComparison, DigestPair, DigestVerificationRequest, FullDigestEvidence,
    PolicyDigestComparison,
};
pub use errors::{RecoveryError, RecoveryResult};
pub use hydration::{
    NonResumableRecoveryFrameSeedProduct, RecoveryFrameSeedProduct, RecoveryHydration,
    ResumableRecoveryFrameSeedProduct,
};
pub use missing::{MissingRunStateComponent, MissingRunStateComponents};
pub use recovered::{
    RecoveredPendingAction, RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState,
};
pub(crate) use replay::ActionReplayEffect;
pub use replay::{ActionReplayTracker, RunSnapshot};
pub use runtime::{RecoveredRunAdmission, RecoveryRuntimeSummary, RecoveryTerminalState};
pub use seed::RecoveryFrameSeed;
pub use unsupported::UnsupportedRecoveryState;
