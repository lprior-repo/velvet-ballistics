#![forbid(unsafe_code)]

use super::{
    RecoveredPendingAction, RecoveredSlotEntry, RecoveredStepEntry, RecoveryCannotResumeState,
    RecoveryFrameSeedProduct, RecoveryRuntimeSummary, UnsupportedRecoveryState,
};
use vb_core::StepIdx;

impl PartialEq<RecoveryFrameSeedProduct> for RecoveryFrameSeed {
    fn eq(&self, other: &RecoveryFrameSeedProduct) -> bool {
        self == other.seed()
    }
}

/// Compatibility-only raw replay DTO recovered from durable journal headers/events.
///
/// This struct is intentionally NOT a recovery boundary. Public recovery
/// entry points on the storage layer return [`RecoveryFrameSeedProduct`],
/// which carries the typed cannot-resume/resumable classification that the
/// runtime boundary depends on. Constructing or consuming a raw
/// [`RecoveryFrameSeed`] outside the documented compat surfaces erases
/// the storage cannot-resume witness and must not be used as evidence
/// that a live runtime `RunState` can resume.
///
/// **Typestate status (bead `vb-sixsf`): full closure is NOT yet claimed.**
/// Public visibility is preserved for low-level replay tests, verifier
/// mirrors, and `vb_storage::recovery::recover::recover_raw_*` compat
/// paths. Production `Runtime` callers MUST use the storage-layer helpers
/// `recover_runtime_frame_seed` / `recover_runtime_frame_seed_from_events`
/// (which return the typestate product) instead of the `_raw_` variants.
/// See bead `vb-sixsf` for the closure roadmap.
///
/// **Production path** (bead `vb-w25-runtime-a2` FINDING-001): production
/// `Runtime::recover_product` / `Runtime::recover_and_resume` do **not**
/// consume a raw [`RecoveryFrameSeed`]; they route durable evidence
/// through the parallel layered boundary that emits
/// `vb_runtime::recovery::RuntimeRecoveryProduct` (`SummaryOnly` /
/// `CannotResume { reason }` / `Resumable`). The
/// `vb_runtime::recovery::DurableFrameRecoveryBoundary` and its
/// `from_seed` / `from_product` constructors are a non-production public
/// surface used by storage-side replay tests and Verus verifier mirrors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryFrameSeed {
    /// Runtime summary for the same event set.
    pub summary: RecoveryRuntimeSummary,
    /// First program-counter step for the rebuilt frame.
    pub first_step: StepIdx,
    /// Minimum step-state capacity needed for observed events.
    pub step_count: u16,
    /// Minimum slot capacity needed for observed slot/result references.
    pub slot_count: u16,
    /// Program counter inferred from the latest observed step event.
    pub pc: StepIdx,
    /// Final step states inferred from durable lifecycle events.
    pub steps: Vec<RecoveredStepEntry>,
    /// Slot values reconstructed by deterministic replay.
    pub slots: Vec<RecoveredSlotEntry>,
    /// Actions scheduled but not completed or failed at the recovery point.
    pub pending_actions: Vec<RecoveredPendingAction>,
    /// Exact pieces of live runtime state not represented by durable events yet.
    pub unsupported: UnsupportedRecoveryState,
}

impl RecoveryFrameSeed {
    /// Returns the exact cannot-resume classification for this seed.
    #[must_use]
    pub fn cannot_resume_state(&self) -> RecoveryCannotResumeState {
        RecoveryCannotResumeState::from_seed(self)
    }

    /// Returns true when this seed has enough evidence to hydrate a live frame safely.
    #[must_use]
    pub fn is_resumable(&self) -> bool {
        self.cannot_resume_state().is_resumable()
    }
}
