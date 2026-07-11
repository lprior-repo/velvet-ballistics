#![forbid(unsafe_code)]
//! Recovery types for velvet-ballistics journal.
//!
//! Provides:
//! - Digest mismatch detection types
//! - Recovery state types
//! - Frame seed types for live-frame reconstruction

use crate::{EventSeq, JournalError};
use vb_core::{
    ActionId, ActionTicket, CapabilitySet, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx,
    Taint, WorkflowDigest,
};

#[cfg(kani)]
#[derive(Debug, Clone, Default)]
struct ReplayResolutionSet(Vec<(ActionId, StepIdx)>);

#[cfg(kani)]
impl ReplayResolutionSet {
    fn insert(&mut self, value: (ActionId, StepIdx)) -> bool {
        if self.contains(&value) {
            false
        } else {
            self.0.push(value);
            true
        }
    }

    fn contains(&self, value: &(ActionId, StepIdx)) -> bool {
        self.0.iter().any(|entry| entry == value)
    }
}

/// Recovery failures with typed diagnostics.
#[derive(Debug)]
#[non_exhaustive]
pub enum RecoveryError {
    /// Journal operation failed during recovery.
    Journal(JournalError),
    /// Workflow source digest does not match the stored record.
    WorkflowSourceDigestMismatch {
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Compiled IR digest does not match the stored record.
    CompiledIrDigestMismatch {
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Action ABI digest mismatch during recovery.
    ActionAbiMismatch {
        /// Action with mismatched ABI.
        action_id: ActionId,
    },
    /// Policy digest mismatch during recovery.
    PolicyDigestMismatch {
        /// Step where policy diverged.
        step: StepIdx,
    },
    /// A non-idempotent action was encountered during recovery and cannot be re-executed.
    NonIdempotentActionBlocked {
        /// Action identifier.
        action: ActionId,
        /// Step where the action was scheduled.
        step: StepIdx,
    },
    /// Replay diverged from expected state machine trajectory.
    ReplayDivergence {
        /// Step where divergence was detected.
        step: StepIdx,
        /// Divergence description.
        detail: String,
    },
    /// Recovery could not read existing slot taint and must fail closed.
    SlotTaintReadFailed {
        /// Slot whose taint could not be read.
        slot: SlotIdx,
    },
    /// Durable slot taint metadata was present but could not be decoded.
    CorruptSlotTaint {
        /// Slot whose persisted taint metadata was corrupt.
        slot: SlotIdx,
    },
    /// No snapshot or journal events found for run.
    NoRecoveryData {
        /// Run identifier.
        run: RunId,
    },
    /// Snapshot is present but corrupt or unreadable.
    CorruptSnapshot {
        /// Run identifier.
        run: RunId,
        /// Snapshot sequence.
        seq: EventSeq,
    },
    /// No snapshot has been written for the requested (run, seq).
    ///
    /// This is distinct from `CorruptSnapshot`: a missing snapshot has no
    /// record in the keyspace (`Ok(None)` from `journal.snapshot`), while a
    /// corrupt snapshot exists but its envelope / postcard payload cannot
    /// be decoded (`Err(JournalError::PostcardDecodeFailed(_))`). Master §18
    /// line 873 requires a typed storage-error surface so the recovery
    /// boundary can pick snapshot-plus-tail recovery (missing) versus
    /// fail-closed on real corruption (corrupt) without conflating them.
    MissingSnapshot {
        /// Run identifier.
        run: RunId,
        /// Snapshot sequence that was requested.
        seq: EventSeq,
    },
    /// Recovery produced a terminal state that does not match expectations.
    TerminalStateMismatch {
        /// Expected terminal event kind.
        expected: String,
        /// Found terminal event kind.
        found: String,
    },
    /// Durable event indexes exceed the runtime frame dimensions that can be represented.
    FrameDimensionOverflow {
        /// Run identifier.
        run: RunId,
    },
    /// Recovery frame seed cannot support a live `RunState` (workflow,
    /// store, action attempts, admission, collect states, action
    /// contracts, or action ABI digests are not present in durable
    /// events). The runtime cannot resume execution from a frame-only
    /// seed; callers must reconcile via summary-only observation or
    /// restart from scratch.
    UnsupportedFrameSeed {
        /// Run identifier.
        run: RunId,
        /// Canonical reason string for the cannot-resume classification.
        reason: String,
    },
    /// Artifact was not found in the store during recovery.
    ArtifactNotFound {
        /// Digest that was looked up.
        digest: WorkflowDigest,
    },
    /// Artifact IR bytes could not be decoded to WorkflowParts.
    ArtifactDecodeFailed,
}

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

impl From<JournalError> for RecoveryError {
    fn from(inner: JournalError) -> Self {
        Self::Journal(inner)
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

impl Eq for RecoveryError {}

/// Result alias for recovery operations.
pub type RecoveryResult<T> = Result<T, RecoveryError>;

/// Expected/found digest pair for a recovery verification subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigestPair {
    /// Expected digest from the trusted manifest or caller contract.
    pub expected: WorkflowDigest,
    /// Observed digest recovered from durable evidence.
    pub found: WorkflowDigest,
}

impl DigestPair {
    /// Builds a digest comparison pair.
    #[must_use]
    pub const fn new(expected: WorkflowDigest, found: WorkflowDigest) -> Self {
        Self { expected, found }
    }
}

/// Action ABI digest comparison tied to the action identity it protects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionAbiDigestComparison {
    /// Action whose ABI digest is checked.
    pub action_id: ActionId,
    /// Expected/found digest pair.
    pub digest: DigestPair,
}

impl ActionAbiDigestComparison {
    /// Builds an action ABI digest comparison.
    #[must_use]
    pub const fn new(action_id: ActionId, expected: WorkflowDigest, found: WorkflowDigest) -> Self {
        Self {
            action_id,
            digest: DigestPair::new(expected, found),
        }
    }
}

/// Policy digest comparison tied to the step whose policy is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyDigestComparison {
    /// Step whose policy digest is checked.
    pub step: StepIdx,
    /// Expected/found digest pair.
    pub digest: DigestPair,
}

impl PolicyDigestComparison {
    /// Builds a policy digest comparison.
    #[must_use]
    pub const fn new(step: StepIdx, expected: WorkflowDigest, found: WorkflowDigest) -> Self {
        Self {
            step,
            digest: DigestPair::new(expected, found),
        }
    }
}

/// Explicit evidence required by full digest verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FullDigestEvidence<'a> {
    action_abi: &'a [ActionAbiDigestComparison],
    policy: &'a [PolicyDigestComparison],
}

impl<'a> FullDigestEvidence<'a> {
    /// Builds full digest evidence from typed subject comparisons.
    #[must_use]
    pub const fn new(
        action_abi: &'a [ActionAbiDigestComparison],
        policy: &'a [PolicyDigestComparison],
    ) -> Self {
        Self { action_abi, policy }
    }

    /// Explicit evidence that the manifest has no action/policy subjects.
    #[must_use]
    pub const fn no_contracts() -> Self {
        Self {
            action_abi: &[],
            policy: &[],
        }
    }

    /// Evidence for action ABI subjects only.
    #[must_use]
    pub const fn action_abi_only(action_abi: &'a [ActionAbiDigestComparison]) -> Self {
        Self {
            action_abi,
            policy: &[],
        }
    }

    /// Evidence for policy subjects only.
    #[must_use]
    pub const fn policy_only(policy: &'a [PolicyDigestComparison]) -> Self {
        Self {
            action_abi: &[],
            policy,
        }
    }

    /// Action ABI comparisons carried by this evidence.
    #[must_use]
    pub const fn action_abi(self) -> &'a [ActionAbiDigestComparison] {
        self.action_abi
    }

    /// Policy comparisons carried by this evidence.
    #[must_use]
    pub const fn policy(self) -> &'a [PolicyDigestComparison] {
        self.policy
    }
}

/// Type-coupled digest verification request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestVerificationRequest<'a> {
    /// Verify only the workflow source digest from durable RunAccepted evidence.
    WorkflowSourceOnly {
        /// Expected workflow/source digest.
        expected_workflow_digest: WorkflowDigest,
    },
    /// Verify workflow/source digest and compiled IR digest.
    WorkflowAndIr {
        /// Expected workflow/source digest.
        expected_workflow_digest: WorkflowDigest,
        /// Expected compiled IR digest.
        expected_ir_digest: WorkflowDigest,
        /// Observed compiled IR digest.
        found_ir_digest: WorkflowDigest,
    },
    /// Verify workflow/source, compiled IR, action ABI, and policy digests.
    Full {
        /// Expected workflow/source digest.
        expected_workflow_digest: WorkflowDigest,
        /// Expected compiled IR digest.
        expected_ir_digest: WorkflowDigest,
        /// Observed compiled IR digest.
        found_ir_digest: WorkflowDigest,
        /// Typed full-verification evidence.
        evidence: FullDigestEvidence<'a>,
    },
}

impl<'a> DigestVerificationRequest<'a> {
    /// Builds a workflow-source-only verification request.
    #[must_use]
    pub const fn workflow_source_only(expected_workflow_digest: WorkflowDigest) -> Self {
        Self::WorkflowSourceOnly {
            expected_workflow_digest,
        }
    }

    /// Builds a workflow plus compiled-IR verification request.
    #[must_use]
    pub const fn workflow_and_ir(
        expected_workflow_digest: WorkflowDigest,
        expected_ir_digest: WorkflowDigest,
        found_ir_digest: WorkflowDigest,
    ) -> Self {
        Self::WorkflowAndIr {
            expected_workflow_digest,
            expected_ir_digest,
            found_ir_digest,
        }
    }

    /// Builds a full verification request with typed action/policy evidence.
    #[must_use]
    pub const fn full(
        expected_workflow_digest: WorkflowDigest,
        expected_ir_digest: WorkflowDigest,
        found_ir_digest: WorkflowDigest,
        evidence: FullDigestEvidence<'a>,
    ) -> Self {
        Self::Full {
            expected_workflow_digest,
            expected_ir_digest,
            found_ir_digest,
            evidence,
        }
    }
}

/// Terminal status recovered from durable journal events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryTerminalState {
    /// Run was cancelled before completion.
    Cancelled,
    /// Run was killed by the runtime.
    Killed,
    /// Run completed and selected a result slot.
    Finished {
        /// Result slot selected by the finish event.
        result: SlotIdx,
    },
    /// Run failed.
    Failed,
}

/// Runtime summary that can be recovered without reconstructing a live `RunFrame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryRuntimeSummary {
    /// Run identifier summarized by this recovery view.
    pub run: RunId,
    /// First sequence observed for the run.
    pub first_seq: EventSeq,
    /// Last sequence observed for the run.
    pub last_seq: EventSeq,
    /// Compiled workflow digest from the acceptance event, when present.
    pub workflow: Option<WorkflowDigest>,
    /// Number of step start events.
    pub steps_started: u64,
    /// Number of step success events.
    pub steps_succeeded: u64,
    /// Number of action schedule events.
    pub actions_scheduled: u64,
    /// Number of resolved action events.
    pub actions_resolved: u64,
    /// Number of boundary suspension events.
    pub suspensions: u64,
    /// Number of slot write events.
    pub slots_written: u64,
    /// Terminal status, when a terminal event exists.
    pub terminal: Option<RecoveryTerminalState>,
}

/// Admission metadata recovered from durable journal events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredRunAdmission {
    /// Digest of the accepted compiled artifact.
    pub artifact_digest: WorkflowDigest,
    /// Run identifier assigned at admission.
    pub run_id: RunId,
    /// Capabilities granted for this run.
    pub granted_capabilities: CapabilitySet,
    /// Admission policy that governed this admission decision.
    pub policy: RuntimePolicy,
}

/// Explicit recovery product. Supports summary-only recovery or a typed
/// frame-seed product recovered from durable journal events.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryHydration {
    /// Summary-only recovery product.
    Summary(RecoveryRuntimeSummary),
    /// Frame-seed recovery product classified at the storage boundary.
    FrameSeed(RecoveryFrameSeedProduct),
}

impl RecoveryHydration {
    /// Builds a summary-only recovery product.
    #[must_use]
    pub const fn from_summary(summary: RecoveryRuntimeSummary) -> Self {
        Self::Summary(summary)
    }

    /// Builds a typed frame-seed recovery product from a raw recovered seed.
    #[must_use]
    pub fn from_frame_seed(seed: RecoveryFrameSeed) -> Self {
        Self::FrameSeed(RecoveryFrameSeedProduct::from_seed(seed))
    }

    /// Returns the summary carried by this hydration product.
    #[must_use]
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        match self {
            Self::Summary(summary) => *summary,
            Self::FrameSeed(product) => product.summary(),
        }
    }

    /// Returns the typed frame-seed product when this hydration carries one.
    #[must_use]
    pub const fn frame_seed_product(&self) -> Option<&RecoveryFrameSeedProduct> {
        match self {
            Self::FrameSeed(product) => Some(product),
            Self::Summary(_) => None,
        }
    }
}

/// Storage recovery product for a raw [`RecoveryFrameSeed`].
///
/// The seed remains a durable replay DTO, while this product is the typed
/// boundary claim. Callers cannot turn a raw seed into a `RecoveryHydration`
/// frame product without classifying it as `CannotResume` or `Resumable`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryFrameSeedProduct {
    /// Frame-seed evidence is present but cannot safely resume.
    CannotResume(NonResumableRecoveryFrameSeedProduct),
    /// Frame-seed evidence is sufficient for a lower-level live-frame resume.
    Resumable(ResumableRecoveryFrameSeedProduct),
}

/// Cannot-resume frame-seed product carrying the typed witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonResumableRecoveryFrameSeedProduct {
    seed: RecoveryFrameSeed,
    cannot_resume: RecoveryCannotResumeState,
}

/// Resumable frame-seed product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumableRecoveryFrameSeedProduct {
    seed: RecoveryFrameSeed,
}

impl RecoveryFrameSeedProduct {
    /// Classifies a raw recovered frame seed into a storage recovery product.
    #[must_use]
    pub fn from_seed(seed: RecoveryFrameSeed) -> Self {
        let cannot_resume = seed.cannot_resume_state();
        if cannot_resume.is_resumable() {
            Self::Resumable(ResumableRecoveryFrameSeedProduct { seed })
        } else {
            Self::CannotResume(NonResumableRecoveryFrameSeedProduct {
                seed,
                cannot_resume,
            })
        }
    }

    /// Summary carried by the underlying recovered seed.
    #[must_use]
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        self.seed().summary
    }

    /// Typed cannot-resume witness computed when the product was constructed.
    /// Resumable products return [`RecoveryCannotResumeState::RESUMABLE`].
    #[must_use]
    pub const fn cannot_resume_state(&self) -> RecoveryCannotResumeState {
        match self {
            Self::CannotResume(product) => product.cannot_resume,
            Self::Resumable(_) => RecoveryCannotResumeState::RESUMABLE,
        }
    }

    /// Returns true only if the typed witness contains no cannot-resume flags.
    #[must_use]
    pub const fn is_resumable(&self) -> bool {
        matches!(self, Self::Resumable(_))
    }

    /// Borrows the raw seed for lower-level recovery code that still consumes
    /// replay DTOs directly.
    #[must_use]
    pub const fn seed(&self) -> &RecoveryFrameSeed {
        match self {
            Self::CannotResume(product) => &product.seed,
            Self::Resumable(product) => &product.seed,
        }
    }

    /// Returns the cannot-resume product when classification found missing
    /// recovery evidence.
    #[must_use]
    pub const fn cannot_resume_product(&self) -> Option<&NonResumableRecoveryFrameSeedProduct> {
        match self {
            Self::CannotResume(product) => Some(product),
            Self::Resumable(_) => None,
        }
    }
}

impl std::ops::Deref for RecoveryFrameSeedProduct {
    type Target = RecoveryFrameSeed;

    fn deref(&self) -> &Self::Target {
        self.seed()
    }
}

impl PartialEq<RecoveryFrameSeed> for RecoveryFrameSeedProduct {
    fn eq(&self, other: &RecoveryFrameSeed) -> bool {
        self.seed() == other
    }
}

impl PartialEq<RecoveryFrameSeedProduct> for RecoveryFrameSeed {
    fn eq(&self, other: &RecoveryFrameSeedProduct) -> bool {
        self == other.seed()
    }
}

impl NonResumableRecoveryFrameSeedProduct {
    /// Summary carried by the underlying recovered seed.
    #[must_use]
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        self.seed.summary
    }

    /// Typed cannot-resume witness for this product.
    #[must_use]
    pub const fn cannot_resume_state(&self) -> RecoveryCannotResumeState {
        self.cannot_resume
    }

    /// Borrows the raw seed for diagnostic or compatibility paths.
    #[must_use]
    pub const fn seed(&self) -> &RecoveryFrameSeed {
        &self.seed
    }
}

impl ResumableRecoveryFrameSeedProduct {
    /// Summary carried by the underlying recovered seed.
    #[must_use]
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        self.seed.summary
    }

    /// Borrows the raw seed for lower-level frame hydration.
    #[must_use]
    pub const fn seed(&self) -> &RecoveryFrameSeed {
        &self.seed
    }
}

/// Step state recovered from durable lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveredStepState {
    /// Step has started or is waiting on action completion.
    Running,
    /// Step completed successfully.
    Succeeded,
    /// Step failed.
    Failed,
    /// Step is suspended on a wait primitive.
    Waiting,
    /// Step is suspended on an ask primitive.
    Asking,
}

/// One recovered step-state entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveredStepEntry {
    /// Step index.
    pub step: StepIdx,
    /// Durable state inferred for this step.
    pub state: RecoveredStepState,
}

/// One slot value recovered by deterministic workflow replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveredSlotEntry {
    /// Slot index.
    pub slot: SlotIdx,
    /// Reconstructed slot value.
    pub value: SlotValue,
    /// Reconstructed taint marker.
    pub taint: Taint,
}

/// One pending action reconstructed from unresolved action lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveredPendingAction {
    /// Step that scheduled the action.
    pub step: StepIdx,
    /// Durable action identifier.
    pub action: ActionId,
}

/// State that durable headers/events still cannot reconstruct into a live frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedRecoveryState {
    /// Slot values are not present in current slot-written records.
    pub slot_values: bool,
    /// Slot taint is not present in current slot-written records.
    pub slot_taint: bool,
    /// Action payload/result bodies are not present in current action records.
    pub action_payloads: bool,
    /// Pending action resumability cannot be projected into the runtime frame yet.
    pub pending_actions: bool,
}

impl UnsupportedRecoveryState {
    /// Recovery state is fully supported by the runtime hydration boundary.
    pub const SUPPORTED: Self = Self {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    };

    /// Event-only slot values have no durable taint payload.
    #[must_use]
    pub const fn event_slot_taint_unsupported() -> Self {
        Self {
            slot_taint: true,
            ..Self::SUPPORTED
        }
    }

    /// Some slot value bodies were missing or corrupt in the durable record.
    #[must_use]
    pub const fn slot_values_unsupported() -> Self {
        Self {
            slot_values: true,
            ..Self::SUPPORTED
        }
    }

    /// Pending actions were recovered but cannot yet be resumed by `RunFrame`.
    #[must_use]
    pub const fn pending_actions_unsupported() -> Self {
        Self {
            pending_actions: true,
            ..Self::SUPPORTED
        }
    }

    /// Combines two support descriptors without permitting contradictory states.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self {
            slot_values: self.slot_values || other.slot_values,
            slot_taint: self.slot_taint || other.slot_taint,
            action_payloads: self.action_payloads || other.action_payloads,
            pending_actions: self.pending_actions || other.pending_actions,
        }
    }

    /// Production proof surface for `SUPPORTED`: every unsupported flag is false.
    #[must_use]
    pub const fn is_fully_supported(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
    }

    /// Production proof surface for flag-wise union correspondence.
    #[must_use]
    pub const fn union_matches_flags(self, other: Self, union: Self) -> bool {
        union.slot_values == (self.slot_values || other.slot_values)
            && union.slot_taint == (self.slot_taint || other.slot_taint)
            && union.action_payloads == (self.action_payloads || other.action_payloads)
            && union.pending_actions == (self.pending_actions || other.pending_actions)
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

/// Identifier for a single full-RunState component whose absence is
/// tracked on a [`RecoveryCannotResumeState`] witness.
///
/// These map 1:1 to the seven `*_missing` flags on
/// [`RecoveryCannotResumeState`]. The enum is intentionally distinct
/// from the mask struct ([`MissingRunStateComponents`]) so callers
/// can talk about a single component (e.g. via
/// [`MissingRunStateComponents::single`]) without constructing the
/// whole bitmask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingRunStateComponent {
    /// A compiled workflow is required by the live runtime but is not represented.
    Workflow,
    /// A cold value store is required by the live runtime but is not represented.
    Store,
    /// Per-Do-step action attempt counters are required but not represented.
    ActionAttempts,
    /// Admission metadata is required by the live runtime but is not represented.
    Admission,
    /// Per-run collect pagination state is required but not represented.
    CollectStates,
    /// Validated action contracts are required but not represented.
    ActionContracts,
    /// Dense action ABI digest table is required but not represented.
    ActionAbiDigests,
}

/// Mask for which full-RunState components are absent from a seed.
///
/// Used to parameterize
/// [`RecoveryCannotResumeState::mark_missing_components`] so the
/// priority chain reason string in
/// [`RecoveryCannotResumeState::unsupported_reason`] can exercise
/// every reachable token (e.g. `"store_missing"`,
/// `"action_attempts_missing"`, `"admission_missing"`,
/// `"collect_states_missing"`, `"action_contracts_missing"`,
/// `"action_abi_digests_missing"`), not just the first one in priority
/// order (`"workflow_missing"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MissingRunStateComponents {
    /// Compile workflow digest is not represented in the seed.
    pub workflow: bool,
    /// Cold value store is not represented in the seed.
    pub store: bool,
    /// Per-Do-step action attempt counters are not represented.
    pub action_attempts: bool,
    /// Admission metadata is not represented.
    pub admission: bool,
    /// Per-run collect pagination state is not represented.
    pub collect_states: bool,
    /// Validated action contracts are not represented.
    pub action_contracts: bool,
    /// Dense action ABI digest table is not represented.
    pub action_abi_digests: bool,
}

impl MissingRunStateComponents {
    /// Mask with every full-RunState component marked missing.
    pub const ALL: Self = Self {
        workflow: true,
        store: true,
        action_attempts: true,
        admission: true,
        collect_states: true,
        action_contracts: true,
        action_abi_digests: true,
    };

    /// Mask with no full-RunState component marked missing.
    pub const NONE: Self = Self {
        workflow: false,
        store: false,
        action_attempts: false,
        admission: false,
        collect_states: false,
        action_contracts: false,
        action_abi_digests: false,
    };

    /// Build a mask that marks exactly one component missing.
    #[must_use]
    pub const fn single(component: MissingRunStateComponent) -> Self {
        match component {
            MissingRunStateComponent::Workflow => Self {
                workflow: true,
                ..Self::NONE
            },
            MissingRunStateComponent::Store => Self {
                store: true,
                ..Self::NONE
            },
            MissingRunStateComponent::ActionAttempts => Self {
                action_attempts: true,
                ..Self::NONE
            },
            MissingRunStateComponent::Admission => Self {
                admission: true,
                ..Self::NONE
            },
            MissingRunStateComponent::CollectStates => Self {
                collect_states: true,
                ..Self::NONE
            },
            MissingRunStateComponent::ActionContracts => Self {
                action_contracts: true,
                ..Self::NONE
            },
            MissingRunStateComponent::ActionAbiDigests => Self {
                action_abi_digests: true,
                ..Self::NONE
            },
        }
    }
}

/// Typed recovery decision for live resume eligibility.
///
/// This is deliberately wider than [`UnsupportedRecoveryState`]: a frame seed
/// can have supported slot bytes and still be unsafe to resume because live
/// runtime boundary state is not represented by `RunFrame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryCannotResumeState {
    /// Slot values are not present or cannot be reconstructed.
    pub slot_values: bool,
    /// Slot taint is not present or cannot be reconstructed.
    pub slot_taint: bool,
    /// Action payload/result bodies are not present in durable records.
    pub action_payloads: bool,
    /// An unresolved action boundary exists without live queue reconstruction.
    pub pending_actions: bool,
    /// A wait/timer boundary exists without timer-wheel authority.
    pub pending_timers: bool,
    /// An ask boundary exists without ask-ticket/resume-slot authority.
    pub pending_asks: bool,
    /// A compiled workflow is required by the live runtime but is not represented.
    pub workflow_missing: bool,
    /// A cold value store is required by the live runtime but is not represented.
    pub store_missing: bool,
    /// Per-Do-step action attempt counters are required but not represented.
    pub action_attempts_missing: bool,
    /// Admission metadata is required by the live runtime but is not represented.
    pub admission_missing: bool,
    /// Per-run collect pagination state is required but not represented.
    pub collect_states_missing: bool,
    /// Validated action contracts are required but not represented.
    pub action_contracts_missing: bool,
    /// Dense action ABI digest table is required but not represented.
    pub action_abi_digests_missing: bool,
}

impl RecoveryCannotResumeState {
    /// Fully resumable state: no missing evidence and no pending live boundary.
    pub const RESUMABLE: Self = Self {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
        pending_timers: false,
        pending_asks: false,
        workflow_missing: false,
        store_missing: false,
        action_attempts_missing: false,
        admission_missing: false,
        collect_states_missing: false,
        action_contracts_missing: false,
        action_abi_digests_missing: false,
    };

    /// Starts classification from storage-level unsupported evidence.
    #[must_use]
    pub const fn from_unsupported(unsupported: UnsupportedRecoveryState) -> Self {
        Self {
            slot_values: unsupported.slot_values,
            slot_taint: unsupported.slot_taint,
            action_payloads: unsupported.action_payloads,
            pending_actions: unsupported.pending_actions,
            pending_timers: false,
            pending_asks: false,
            workflow_missing: false,
            store_missing: false,
            action_attempts_missing: false,
            admission_missing: false,
            collect_states_missing: false,
            action_contracts_missing: false,
            action_abi_digests_missing: false,
        }
    }

    /// Classifies a recovered frame seed as resumable or cannot-resume.
    ///
    /// The frame seed is missing every component of the live runtime
    /// `RunState` (workflow, store, action attempts, admission,
    /// collect states, action contracts, action ABI digests), so the
    /// mask is [`MissingRunStateComponents::ALL`]. Tests that want to
    /// exercise a single second-half reason token call
    /// [`Self::mark_missing_components`] directly with
    /// [`MissingRunStateComponents::single`] instead.
    #[must_use]
    pub fn from_seed(seed: &RecoveryFrameSeed) -> Self {
        let mut state = Self::from_unsupported(seed.unsupported);
        state.pending_actions = state.pending_actions || !seed.pending_actions.is_empty();
        for entry in &seed.steps {
            state.classify_step_state(entry.state);
        }
        state = state.mark_missing_components(MissingRunStateComponents::ALL);
        state
    }

    /// Apply a parameter-mask of missing full-RunState components,
    /// setting the corresponding `*_missing` flags. Replaces the
    /// previous unconditional `mark_full_run_state_missing`. Tests
    /// drive this with [`MissingRunStateComponents::single`] to
    /// exercise each second-half reason token
    /// (`"store_missing"`, `"action_attempts_missing"`, etc.) in
    /// isolation, since the priority chain in
    /// [`Self::unsupported_reason`] is dominated by `"workflow_missing"`
    /// when all seven flags are set together.
    #[must_use]
    pub const fn mark_missing_components(mut self, components: MissingRunStateComponents) -> Self {
        if components.workflow {
            self.workflow_missing = true;
        }
        if components.store {
            self.store_missing = true;
        }
        if components.action_attempts {
            self.action_attempts_missing = true;
        }
        if components.admission {
            self.admission_missing = true;
        }
        if components.collect_states {
            self.collect_states_missing = true;
        }
        if components.action_contracts {
            self.action_contracts_missing = true;
        }
        if components.action_abi_digests {
            self.action_abi_digests_missing = true;
        }
        self
    }

    /// Deprecated alias for
    /// [`Self::mark_missing_components`] with
    /// [`MissingRunStateComponents::ALL`]. Retained so any external
    /// call site (currently none — only `from_seed` invokes it) keeps
    /// compiling unchanged. New code should call
    /// [`Self::mark_missing_components`] directly.
    #[must_use]
    #[deprecated(
        since = "0.0.0",
        note = "use `mark_missing_components(MissingRunStateComponents::ALL)` instead"
    )]
    pub const fn mark_full_run_state_missing(self) -> Self {
        self.mark_missing_components(MissingRunStateComponents::ALL)
    }

    fn classify_step_state(&mut self, state: RecoveredStepState) {
        match state {
            RecoveredStepState::Waiting => {
                self.pending_timers = true;
            }
            RecoveredStepState::Asking => {
                self.pending_asks = true;
            }
            RecoveredStepState::Running
            | RecoveredStepState::Succeeded
            | RecoveredStepState::Failed => {}
        }
    }

    /// Returns true only when every cannot-resume flag is false.
    #[must_use]
    pub const fn is_resumable(self) -> bool {
        !self.slot_values
            && !self.slot_taint
            && !self.action_payloads
            && !self.pending_actions
            && !self.pending_timers
            && !self.pending_asks
            && !self.workflow_missing
            && !self.store_missing
            && !self.action_attempts_missing
            && !self.admission_missing
            && !self.collect_states_missing
            && !self.action_contracts_missing
            && !self.action_abi_digests_missing
    }

    /// Canonical reason strings ordered by classification priority
    /// (the first true flag wins). Index `i` corresponds to
    /// [`flag_at`](Self::flag_at).
    pub const CANNOT_RESUME_REASONS: [&str; 13] = [
        "slot_values",
        "slot_taint",
        "action_payloads",
        "pending_actions",
        "pending_timers",
        "pending_asks",
        "workflow_missing",
        "store_missing",
        "action_attempts_missing",
        "admission_missing",
        "collect_states_missing",
        "action_contracts_missing",
        "action_abi_digests_missing",
    ];

    /// Returns the cannot-resume flag at priority index `i`. Out-of-
    /// range indices return `false` so the walk above terminates.
    #[must_use]
    pub const fn flag_at(self, i: usize) -> bool {
        match i {
            0 => self.slot_values,
            1 => self.slot_taint,
            2 => self.action_payloads,
            3 => self.pending_actions,
            4 => self.pending_timers,
            5 => self.pending_asks,
            6 => self.workflow_missing,
            7 => self.store_missing,
            8 => self.action_attempts_missing,
            9 => self.admission_missing,
            10 => self.collect_states_missing,
            11 => self.action_contracts_missing,
            12 => self.action_abi_digests_missing,
            _ => false,
        }
    }

    /// Canonical reason string for a typed `UnsupportedFrameSeed` error.
    ///
    /// Dispatches via the `priority_class_first_half` +
    /// `priority_class_second_half` priority scanners plus the
    /// `priority_reason` mapping. The first true flag in
    /// classification-priority order wins; `"resumable"` is the
    /// fallback when every flag is false.
    #[must_use]
    pub const fn unsupported_reason(self) -> &'static str {
        match self.priority_class_first_half() {
            Some(class) => priority_reason(class),
            None => match self.priority_class_second_half() {
                Some(class) => priority_reason(class),
                None => "resumable",
            },
        }
    }
}

/// Classification priority for the cannot-resume reason tokens.
/// The first-matching rule (highest enum variant above) wins;
/// the `None` arm of the priority scan carries through to the
/// "resumable" string fallback in [`unsupported_reason`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CannotResumeClass {
    SlotValues,
    SlotTaint,
    ActionPayloads,
    PendingActions,
    PendingTimers,
    PendingAsks,
    WorkflowMissing,
    StoreMissing,
    ActionAttemptsMissing,
    AdmissionMissing,
    CollectStatesMissing,
    ActionContractsMissing,
    ActionAbiDigestsMissing,
}

impl RecoveryCannotResumeState {
    /// First-half priority scan (storage-layer + pending-boundary
    /// flags 0..6). Returns the highest-priority cannot-resume reason
    /// seen in this half, or `None` if every flag in the half is
    /// false. Pure projection over the witness.
    const fn priority_class_first_half(self) -> Option<CannotResumeClass> {
        if self.slot_values {
            return Some(CannotResumeClass::SlotValues);
        }
        if self.slot_taint {
            return Some(CannotResumeClass::SlotTaint);
        }
        if self.action_payloads {
            return Some(CannotResumeClass::ActionPayloads);
        }
        if self.pending_actions {
            return Some(CannotResumeClass::PendingActions);
        }
        if self.pending_timers {
            return Some(CannotResumeClass::PendingTimers);
        }
        if self.pending_asks {
            return Some(CannotResumeClass::PendingAsks);
        }
        None
    }

    /// Second-half priority scan (the seven `*_missing` full-RunState
    /// flags 6..13). Returns the highest-priority cannot-resume reason
    /// seen in this half, or `None` if every flag in the half is
    /// false. Pure projection over the witness.
    const fn priority_class_second_half(self) -> Option<CannotResumeClass> {
        if self.workflow_missing {
            return Some(CannotResumeClass::WorkflowMissing);
        }
        if self.store_missing {
            return Some(CannotResumeClass::StoreMissing);
        }
        if self.action_attempts_missing {
            return Some(CannotResumeClass::ActionAttemptsMissing);
        }
        if self.admission_missing {
            return Some(CannotResumeClass::AdmissionMissing);
        }
        if self.collect_states_missing {
            return Some(CannotResumeClass::CollectStatesMissing);
        }
        if self.action_contracts_missing {
            return Some(CannotResumeClass::ActionContractsMissing);
        }
        if self.action_abi_digests_missing {
            return Some(CannotResumeClass::ActionAbiDigestsMissing);
        }
        None
    }
}

/// Maps a [`CannotResumeClass`] to its canonical reason token.
/// Order matches [`CannotResumeClass`] declaration; the spec proof
/// `proof_unsupported_reason_first_match_wins` in
/// `verification/verus/recovery_verification.rs` discharges the
/// priority invariant over this mapping.
#[must_use]
const fn priority_reason(class: CannotResumeClass) -> &'static str {
    match class {
        CannotResumeClass::SlotValues => "slot_values",
        CannotResumeClass::SlotTaint => "slot_taint",
        CannotResumeClass::ActionPayloads => "action_payloads",
        CannotResumeClass::PendingActions => "pending_actions",
        CannotResumeClass::PendingTimers => "pending_timers",
        CannotResumeClass::PendingAsks => "pending_asks",
        CannotResumeClass::WorkflowMissing => "workflow_missing",
        CannotResumeClass::StoreMissing => "store_missing",
        CannotResumeClass::ActionAttemptsMissing => "action_attempts_missing",
        CannotResumeClass::AdmissionMissing => "admission_missing",
        CannotResumeClass::CollectStatesMissing => "collect_states_missing",
        CannotResumeClass::ActionContractsMissing => "action_contracts_missing",
        CannotResumeClass::ActionAbiDigestsMissing => "action_abi_digests_missing",
    }
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

/// Snapshot of a run's runtime state at a specific event sequence.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunSnapshot {
    /// Run identifier.
    pub run: RunId,
    /// Sequence number at which this snapshot was taken.
    pub seq: EventSeq,
    /// Compiled workflow digest.
    pub workflow: WorkflowDigest,
    /// Slot values at snapshot time, compact binary form.
    pub slots: Vec<u8>,
    /// Slot taint markers at snapshot time, compact binary form.
    pub taint: Vec<u8>,
}

/// Tracks which actions have been completed during recovery to prevent
/// re-execution of non-idempotent actions.
#[derive(Debug, Clone)]
pub struct ActionReplayTracker {
    scheduled_tickets: std::collections::HashMap<(ActionId, StepIdx), ActionScheduleEvidence>,
    completed: std::collections::HashSet<(ActionId, StepIdx)>,
    failed: std::collections::HashSet<(ActionId, StepIdx)>,
    completed_envelopes: std::collections::HashMap<(ActionId, StepIdx), ActionCompletionEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionScheduleEvidence {
    ticket: ActionTicket,
    input: SlotIdx,
    output: SlotIdx,
    action_abi_digest: WorkflowDigest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionCompletionEvidence {
    ticket: ActionTicket,
    output: SlotIdx,
    encoded_len: u32,
    taint: Taint,
    value_digest: [u8; 32],
    action_abi_digest: WorkflowDigest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionReplayEffect {
    Apply,
    Duplicate,
}

impl ActionReplayTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            scheduled_tickets: std::collections::HashMap::new(),
            completed: std::collections::HashSet::new(),
            failed: std::collections::HashSet::new(),
            completed_envelopes: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn mark_scheduled_ticket_effect(
        &mut self,
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
        action_abi_digest: WorkflowDigest,
    ) -> RecoveryResult<ActionReplayEffect> {
        let key = (ticket.action, ticket.step);
        if self.is_resolved(ticket.action, ticket.step) {
            return Err(RecoveryError::NonIdempotentActionBlocked {
                action: ticket.action,
                step: ticket.step,
            });
        }
        let evidence = ActionScheduleEvidence {
            ticket,
            input,
            output,
            action_abi_digest,
        };
        match self.scheduled_tickets.get(&key).copied() {
            Some(existing) if existing == evidence => Ok(ActionReplayEffect::Duplicate),
            Some(_) => Err(RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("divergent action schedule ticket"),
            }),
            None => {
                self.scheduled_tickets.insert(key, evidence);
                Ok(ActionReplayEffect::Apply)
            }
        }
    }

    pub(crate) fn require_scheduled_ticket(
        &self,
        ticket: ActionTicket,
        output: SlotIdx,
        action_abi_digest: WorkflowDigest,
    ) -> RecoveryResult<()> {
        let key = (ticket.action, ticket.step);
        match self.scheduled_tickets.get(&key).copied() {
            Some(existing)
                if existing.ticket == ticket
                    && existing.output == output
                    && existing.action_abi_digest == action_abi_digest =>
            {
                Ok(())
            }
            Some(_) => Err(RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("action completion envelope does not match schedule ticket"),
            }),
            None => Err(RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("action completion envelope missing schedule ticket"),
            }),
        }
    }

    /// Records that an action was completed during normal execution.
    /// During recovery, encountering this action again will block re-execution.
    pub fn mark_completed(&mut self, action: ActionId, step: StepIdx) {
        self.completed.insert((action, step));
    }
    pub(crate) fn mark_completed_envelope_effect(
        &mut self,
        ticket: ActionTicket,
        output: SlotIdx,
        encoded_len: u32,
        taint: Taint,
        value_digest: [u8; 32],
        action_abi_digest: WorkflowDigest,
    ) -> RecoveryResult<ActionReplayEffect> {
        let key = (ticket.action, ticket.step);
        let evidence = ActionCompletionEvidence {
            ticket,
            output,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
        };
        if let Some(schedule) = self.scheduled_tickets.get(&key).copied()
            && (schedule.ticket != ticket || schedule.output != output)
        {
            return Err(RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("action completion envelope does not match schedule ticket"),
            });
        }
        match self.completed_envelopes.get(&key).copied() {
            Some(existing) if existing == evidence => Ok(ActionReplayEffect::Duplicate),
            Some(_) => Err(RecoveryError::ReplayDivergence {
                step: ticket.step,
                detail: String::from("divergent action completion envelope"),
            }),
            None if self.completed.contains(&key) || self.failed.contains(&key) => {
                Err(RecoveryError::NonIdempotentActionBlocked {
                    action: ticket.action,
                    step: ticket.step,
                })
            }
            None => {
                self.completed_envelopes.insert(key, evidence);
                self.completed.insert(key);
                Ok(ActionReplayEffect::Apply)
            }
        }
    }

    pub fn mark_completed_envelope(
        &mut self,
        ticket: ActionTicket,
        output: SlotIdx,
        encoded_len: u32,
        taint: Taint,
        value_digest: [u8; 32],
        action_abi_digest: WorkflowDigest,
    ) -> RecoveryResult<()> {
        self.mark_completed_envelope_effect(
            ticket,
            output,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
        )
        .map(|_| ())
    }

    /// Records that an action failed during normal execution.
    pub fn mark_failed(&mut self, action: ActionId, step: StepIdx) {
        self.failed.insert((action, step));
    }

    /// Production proof surface: the completed set contains this action/step pair.
    #[must_use]
    pub fn has_completed(&self, action: ActionId, step: StepIdx) -> bool {
        self.completed.contains(&(action, step))
    }

    /// Production proof surface: the failed set contains this action/step pair.
    #[must_use]
    pub fn has_failed(&self, action: ActionId, step: StepIdx) -> bool {
        self.failed.contains(&(action, step))
    }

    /// Checks whether an action has already been resolved (completed or failed)
    /// and must not be re-executed during recovery.
    #[must_use]
    pub fn is_resolved(&self, action: ActionId, step: StepIdx) -> bool {
        self.completed.contains(&(action, step)) || self.failed.contains(&(action, step))
    }
}

impl Default for ActionReplayTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Digest check level for recovery validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DigestCheck {
    /// Only verify workflow source digest.
    WorkflowSourceOnly,
    /// Verify workflow source and compiled IR digests.
    WorkflowAndIr,
    /// Verify all digests including action ABI and policy.
    Full,
}

impl DigestCheck {
    /// Numeric rank for proof and testing of the strict digest hierarchy.
    #[must_use]
    pub const fn hierarchy_rank(self) -> u8 {
        match self {
            Self::WorkflowSourceOnly => 1,
            Self::WorkflowAndIr => 2,
            Self::Full => 3,
        }
    }

    /// Whether this level requires workflow-source digest verification.
    #[must_use]
    pub const fn checks_workflow_source(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowSourceOnly.hierarchy_rank()
    }

    /// Whether this level requires compiled-IR digest verification.
    #[must_use]
    pub const fn checks_compiled_ir(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowAndIr.hierarchy_rank()
    }

    /// Whether this level requires all currently-modeled digest checks.
    #[must_use]
    pub const fn checks_full(self) -> bool {
        self.hierarchy_rank() >= Self::Full.hierarchy_rank()
    }

    /// Production proof surface for strict ordering between two levels.
    #[must_use]
    pub const fn is_strictly_weaker_than(self, other: Self) -> bool {
        self.hierarchy_rank() < other.hierarchy_rank()
    }
}
