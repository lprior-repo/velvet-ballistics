#![forbid(unsafe_code)]
//! Recovery types for velvet-ballistics journal.
//!
//! Provides:
//! - Digest mismatch detection types
//! - Recovery state types
//! - Frame seed types for live-frame reconstruction

use crate::{EventSeq, JournalError};
use serde::{Deserialize, Serialize};
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
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RecoveryError {
    /// Journal operation failed during recovery.
    #[error("journal error during recovery: {0}")]
    Journal(#[from] JournalError),
    /// Workflow source digest does not match the stored record.
    #[error("workflow source digest mismatch: expected {expected:?}, found {found:?}")]
    WorkflowSourceDigestMismatch {
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Compiled IR digest does not match the stored record.
    #[error("compiled IR digest mismatch: expected {expected:?}, found {found:?}")]
    CompiledIrDigestMismatch {
        /// Expected digest.
        expected: WorkflowDigest,
        /// Found digest.
        found: WorkflowDigest,
    },
    /// Action ABI digest mismatch during recovery.
    #[error("action ABI digest mismatch for action {action_id:?}")]
    ActionAbiMismatch {
        /// Action with mismatched ABI.
        action_id: ActionId,
    },
    /// Policy digest mismatch during recovery.
    #[error("policy digest mismatch for step {step:?}")]
    PolicyDigestMismatch {
        /// Step where policy diverged.
        step: StepIdx,
    },
    /// A non-idempotent action was encountered during recovery and cannot be re-executed.
    #[error(
        "non-idempotent action {action:?} at step {step:?} cannot be re-executed during recovery"
    )]
    NonIdempotentActionBlocked {
        /// Action identifier.
        action: ActionId,
        /// Step where the action was scheduled.
        step: StepIdx,
    },
    /// Replay diverged from expected state machine trajectory.
    #[error("replay divergence at step {step:?}: {detail}")]
    ReplayDivergence {
        /// Step where divergence was detected.
        step: StepIdx,
        /// Divergence description.
        detail: String,
    },
    /// Recovery could not read existing slot taint and must fail closed.
    #[error("slot taint read_taint failed for slot {slot:?}")]
    SlotTaintReadFailed {
        /// Slot whose taint could not be read.
        slot: SlotIdx,
    },
    /// Durable slot taint metadata was present but could not be decoded.
    #[error("slot taint metadata corrupt for slot {slot:?}")]
    CorruptSlotTaint {
        /// Slot whose persisted taint metadata was corrupt.
        slot: SlotIdx,
    },
    /// No snapshot or journal events found for run.
    #[error("no recovery data found for run {run:?}")]
    NoRecoveryData {
        /// Run identifier.
        run: RunId,
    },
    /// Snapshot is present but corrupt or unreadable.
    #[error("snapshot corrupt for run {run:?} at seq {seq:?}")]
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
    #[error("no snapshot found for run {run:?} at seq {seq:?}")]
    MissingSnapshot {
        /// Run identifier.
        run: RunId,
        /// Snapshot sequence that was requested.
        seq: EventSeq,
    },
    /// Recovery produced a terminal state that does not match expectations.
    #[error("recovery terminal state mismatch: expected {expected:?}, found {found:?}")]
    TerminalStateMismatch {
        /// Expected terminal event kind.
        expected: String,
        /// Found terminal event kind.
        found: String,
    },
    /// Durable event indexes exceed the runtime frame dimensions that can be represented.
    #[error("recovery frame dimension overflow for run {run:?}")]
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
    #[error("recovery frame seed is not resumable for run {run:?}: {reason}")]
    UnsupportedFrameSeed {
        /// Run identifier.
        run: RunId,
        /// Canonical reason string for the cannot-resume classification.
        reason: String,
    },
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Explicit recovery product. Supports summary-only or full live-frame seed
/// recovery from durable journal events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RecoveryHydration {
    /// Summary-only recovery product.
    Summary(RecoveryRuntimeSummary),
    /// Full live-frame seed recovered from durable events.
    FrameSeed(RecoveryFrameSeed),
}

impl RecoveryHydration {
    /// Returns the summary carried by this hydration product.
    #[must_use]
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        match self {
            Self::Summary(summary) => *summary,
            Self::FrameSeed(seed) => seed.summary,
        }
    }
}

/// Step state recovered from durable lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredStepEntry {
    /// Step index.
    pub step: StepIdx,
    /// Durable state inferred for this step.
    pub state: RecoveredStepState,
}

/// One slot value recovered by deterministic workflow replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredSlotEntry {
    /// Slot index.
    pub slot: SlotIdx,
    /// Reconstructed slot value.
    pub value: SlotValue,
    /// Reconstructed taint marker.
    pub taint: Taint,
}

/// One pending action reconstructed from unresolved action lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredPendingAction {
    /// Step that scheduled the action.
    pub step: StepIdx,
    /// Durable action identifier.
    pub action: ActionId,
}

/// State that durable headers/events still cannot reconstruct into a live frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Minimal live-frame seed recovered from durable journal headers/events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Typed recovery decision for live resume eligibility.
///
/// This is deliberately wider than [`UnsupportedRecoveryState`]: a frame seed
/// can have supported slot bytes and still be unsafe to resume because live
/// runtime boundary state is not represented by `RunFrame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    #[must_use]
    pub fn from_seed(seed: &RecoveryFrameSeed) -> Self {
        let mut state = Self::from_unsupported(seed.unsupported);
        state.pending_actions = state.pending_actions || !seed.pending_actions.is_empty();
        for entry in &seed.steps {
            state.classify_step_state(entry.state);
        }
        state.mark_full_run_state_missing();
        state
    }

    const fn mark_full_run_state_missing(&mut self) {
        self.workflow_missing = true;
        self.store_missing = true;
        self.action_attempts_missing = true;
        self.admission_missing = true;
        self.collect_states_missing = true;
        self.action_contracts_missing = true;
        self.action_abi_digests_missing = true;
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

    /// Canonical reason string for a typed `UnsupportedFrameSeed` error.
    #[must_use]
    pub const fn unsupported_reason(self) -> &'static str {
        if self.slot_values {
            "slot_values"
        } else if self.slot_taint {
            "slot_taint"
        } else if self.action_payloads {
            "action_payloads"
        } else if self.pending_actions {
            "pending_actions"
        } else if self.pending_timers {
            "pending_timers"
        } else if self.pending_asks {
            "pending_asks"
        } else if self.workflow_missing {
            "workflow_missing"
        } else if self.store_missing {
            "store_missing"
        } else if self.action_attempts_missing {
            "action_attempts_missing"
        } else if self.admission_missing {
            "admission_missing"
        } else if self.collect_states_missing {
            "collect_states_missing"
        } else if self.action_contracts_missing {
            "action_contracts_missing"
        } else if self.action_abi_digests_missing {
            "action_abi_digests_missing"
        } else {
            "resumable"
        }
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActionCompletionEvidence {
    ticket: ActionTicket,
    output: SlotIdx,
    encoded_len: u32,
    taint: Taint,
    value_digest: [u8; 32],
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
    ) -> RecoveryResult<()> {
        let key = (ticket.action, ticket.step);
        match self.scheduled_tickets.get(&key).copied() {
            Some(existing) if existing.ticket == ticket && existing.output == output => Ok(()),
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
    ) -> RecoveryResult<ActionReplayEffect> {
        let key = (ticket.action, ticket.step);
        let evidence = ActionCompletionEvidence {
            ticket,
            output,
            encoded_len,
            taint,
            value_digest,
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

    /// Records a full durable completion envelope and rejects duplicates whose
    /// ticket or output evidence diverges from the first completed envelope.
    pub fn mark_completed_envelope(
        &mut self,
        ticket: ActionTicket,
        output: SlotIdx,
        encoded_len: u32,
        taint: Taint,
        value_digest: [u8; 32],
    ) -> RecoveryResult<()> {
        self.mark_completed_envelope_effect(ticket, output, encoded_len, taint, value_digest)
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
