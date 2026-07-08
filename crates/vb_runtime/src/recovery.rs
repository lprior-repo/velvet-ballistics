#![forbid(unsafe_code)]
//! Runtime recovery boundary over storage summary hydration.

mod frame;
mod full;
mod product;

use vb_core::frame::RunFrame;
use vb_storage::recovery::{
    RecoveryCannotResumeState, RecoveryFrameSeed, RecoveryFrameSeedProduct, RecoveryHydration,
    RecoveryRuntimeSummary, UnsupportedRecoveryState,
};

pub use full::{
    NonResumableRecoveryProduct, RecoveredOpenAsk, RecoveredRunBoundary, RecoveredRunBoundaryKind,
    ResumableRecoveryProduct, RuntimeRecoveryProduct, SummaryRecoveryProduct,
};
pub(crate) use full::{
    RecoveredAdmissionContext, RecoveredAdmissionEvidence, RecoveredPendingActionTicket,
    ResumableRecoveryParts, action_abi_digests_from_contracts, classify_full_recovery_resume,
    hydrate_frame_for_full_recovery, pending_action_ticket_from_events,
    recovered_run_boundary_from_seed, validate_recovered_admission_evidence,
};

use crate::{RuntimeError, RuntimeResult};
use product::DurableFrameRecoveryProduct;

/// Hydrates the latest run admission metadata from durable storage events.
#[must_use]
pub fn hydrate_run_admission_from_events(
    events: &[vb_storage::JournalEvent],
) -> Option<crate::admission::RunAdmission> {
    vb_storage::recovery::replay::summary::recover_run_admission_from_events(events).map(
        |admission| {
            crate::admission::RunAdmission::new(
                admission.artifact_digest,
                admission.run_id,
                admission.granted_capabilities,
                admission.policy,
            )
        },
    )
}

/// Runtime-facing recovery entrypoint.
pub trait RuntimeRecoveryBoundary {
    /// Returns summary data that can be safely recovered from durable events.
    fn summary(&self) -> RecoveryRuntimeSummary;

    /// Reports whether durable recovery evidence is sufficient to resume.
    fn resume_status(&self) -> RecoveryResumeStatus;

    /// Attempts to hydrate a live run frame.
    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame>;
}

/// Runtime-facing resume decision from durable recovery evidence.
///
/// `Resumable` is intentionally not a variant today: a `RunFrame`
/// seed alone never carries the full runtime boundary state required
/// for live execution (workflow, store, action attempts, admission,
/// collect states, action contracts, action ABI digests), so the
/// typed never-resume witness is the only state a frame seed can
/// emit. When a future recovery path can hydrate a complete
/// `RunState`, add a `Resumable(FullRunState { ... })` carrying the
/// full evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryResumeStatus {
    /// Recovery has precise evidence explaining why execution cannot resume.
    CannotResume(RecoveryCannotResumeState),
    /// Storage exposed summary data only; no live frame seed exists.
    SummaryOnly,
}

/// Runtime recovery boundary backed by a parsed durable frame product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableFrameRecoveryBoundary {
    product: DurableFrameRecoveryProduct,
}

impl DurableFrameRecoveryBoundary {
    /// Builds a runtime boundary from a raw durable storage frame seed.
    ///
    /// The raw seed is consumed at the boundary and classified into a typed
    /// product: malformed shape or shape-checked cannot-resume. The public
    /// boundary never treats a frame seed as a resumable `RunState` claim.
    ///
    /// **Compat-only constructor (FINDING-001).** Production
    /// `Runtime::recover_product` / `Runtime::recover_and_resume` callers
    /// do **not** route through this constructor. They route through the
    /// parallel layered boundary that emits
    /// [`RuntimeRecoveryProduct`]. This `from_seed` constructor exists
    /// for:
    ///
    /// - low-level replay tests (e.g. `crates/vb_runtime/tests/recovery_hydration_tests.rs`,
    ///   `crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs`),
    /// - Verus verifier mirrors under `verification/verus/extern_recovery_verification.rs`
    ///   that bind to the raw [`RecoveryFrameSeed`] DTO,
    /// - storage-only compat paths that already materialize a raw seed
    ///   via `RecoveryFrameSeedBuilder::build` or
    ///   `vb_storage::recovery::recover::recover_raw_*`.
    ///
    /// New code MUST prefer [`Self::from_product`] when the storage layer
    /// has already typed-classified the seed (see also bead `vb-sixsf`).
    /// Full closure of this compat surface is not yet claimed.
    #[must_use]
    pub fn from_seed(seed: RecoveryFrameSeed) -> Self {
        Self {
            product: DurableFrameRecoveryProduct::from_seed(seed),
        }
    }

    /// Builds a runtime boundary from a storage-classified frame product.
    ///
    /// The runtime boundary preserves the storage typestate instead of
    /// erasing it back into a raw frame seed. A storage `CannotResume`
    /// product carries its witness into the runtime layer; a storage
    /// `Resumable` product is still rejected here because a frame seed
    /// alone does not contain the full live `RunState` components.
    ///
    /// **Layered boundary contract (FINDING-001).** The
    /// `DurableFrameRecoveryBoundary` is a *non-production* public surface.
    /// Production callers (`Runtime::recover_product`,
    /// `Runtime::recover_and_resume`) do **not** use this constructor.
    /// They go through a parallel layered boundary that emits the
    /// [`RuntimeRecoveryProduct`] enum (`SummaryOnly` / `CannotResume {
    /// reason }` / `Resumable`). That parallel boundary keeps the typed
    /// `RecoveryCannotResumeState` reason and the broader live-`RunState`
    /// cannot-resume classification (pending actions, pending timers,
    /// pending asks, missing workflow/store/admission/etc.) co-located on
    /// the runtime product. This `from_product` constructor exists so
    /// callers who only need the storage-typestate boundary (storage-side
    /// tests, recovery-pipeline integration tests) can still consume the
    /// typed product directly without going through the layered runtime
    /// boundary.
    ///
    /// Callers who need the full live-`RunState` cannot-resume witness
    /// (the production cannot-resume reason emitted on
    /// `RuntimeRecoveryProduct::CannotResume`) must use
    /// `Runtime::recover_product`. See bead `vb-sixsf`.
    #[must_use]
    pub fn from_product(product: RecoveryFrameSeedProduct) -> Self {
        Self {
            product: DurableFrameRecoveryProduct::from_product(product),
        }
    }

    /// Returns state that the current durable events still cannot hydrate.
    #[must_use]
    pub fn unsupported_state(&self) -> UnsupportedRecoveryState {
        self.product.unsupported_state()
    }

    /// Returns the typed cannot-resume classification for this frame seed.
    #[must_use]
    pub fn cannot_resume_state(&self) -> RecoveryCannotResumeState {
        self.product.cannot_resume_state()
    }
}

impl RuntimeRecoveryBoundary for DurableFrameRecoveryBoundary {
    fn summary(&self) -> RecoveryRuntimeSummary {
        self.product.summary()
    }

    fn resume_status(&self) -> RecoveryResumeStatus {
        self.product.resume_status()
    }

    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame> {
        self.product.hydrate_run_frame()
    }
}

/// Recovery boundary factory that selects summary-only or full-frame
/// hydration based on the storage recovery product.
pub fn recovery_boundary_from_hydration(
    hydration: RecoveryHydration,
) -> Box<dyn RuntimeRecoveryBoundary> {
    let summary = hydration.summary();
    match hydration {
        RecoveryHydration::Summary(summary) => Box::new(SummaryRecoveryBoundary { summary }),
        RecoveryHydration::FrameSeed(product) => {
            Box::new(DurableFrameRecoveryBoundary::from_product(product))
        }
        _ => Box::new(SummaryRecoveryBoundary { summary }),
    }
}

/// Summary-only recovery product accepted by the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryRecoveryBoundary {
    summary: RecoveryRuntimeSummary,
}

impl SummaryRecoveryBoundary {
    /// Builds a runtime recovery boundary from a storage recovery hydration.
    #[must_use]
    pub const fn from_summary(summary: RecoveryRuntimeSummary) -> Self {
        Self { summary }
    }
}

impl RuntimeRecoveryBoundary for SummaryRecoveryBoundary {
    fn summary(&self) -> RecoveryRuntimeSummary {
        self.summary
    }

    fn resume_status(&self) -> RecoveryResumeStatus {
        RecoveryResumeStatus::SummaryOnly
    }

    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame> {
        Err(RuntimeError::UnsupportedFullRecoveryHydration)
    }
}

#[cfg(test)]
#[path = "recovery/tests.rs"]
mod tests;
