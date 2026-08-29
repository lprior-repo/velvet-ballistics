// SPDX-License-Identifier: MIT
//
// ============================================================================
// Companion mirror surface for extern_recovery_verification.rs
// ============================================================================
//
// This file carries the production-bound mirror types, structs, enums,
// and `#[verifier::external]` decision fns that support the
// `recovery_verification` Verus spec. It is included by
// `extern_recovery_verification.rs` via:
//
//   #[path = "extern_recovery_verification_mirror.rs"]
//   pub mod mirror;
//   pub use mirror::{ ... };
//
// Each type mirrors its production counterpart in
// `crates/vb_storage/src/recovery/types.rs`,
// `crates/vb_storage/src/recovery/recover.rs`,
// `crates/vb_storage/src/recovery/hydrate.rs`, and
// `crates/vb_runtime/src/recovery.rs` line-by-line.
// ============================================================================

// ---------------------------------------------------------------------------
// ID type mirrors — production newtypes
// ---------------------------------------------------------------------------

/// Mirror of `RunId` (u64 newtype) at `crates/vb_core/src/ids/mod.rs:65`.
#[derive(Clone, Copy)]
pub struct RunId(pub u64);

/// Mirror of `StepIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:55`.
#[derive(Clone, Copy)]
pub struct StepIdx(pub u16);

/// Mirror of `SlotIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:56`.
#[derive(Clone, Copy)]
pub struct SlotIdx(pub u16);

/// Mirror of `ActionId` (u64 newtype) at `crates/vb_core/src/ids/mod.rs:60`.
#[derive(Clone, Copy)]
pub struct ActionId(pub u64);

/// Mirror of `WorkflowDigest` (newtype over [u8; 32]) at
/// `crates/vb_core/src/ids/mod.rs:80`. We model only the discriminant
/// equality used by the digest comparison decision fns.
#[derive(Clone, Copy)]
pub struct WorkflowDigest(pub u64);

/// Mirror of `EventSeq` (u64 newtype) at `crates/vb_core/src/ids/mod.rs:70`.
#[derive(Clone, Copy)]
pub struct EventSeq(pub u64);

impl RunId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl StepIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
    pub const ZERO: Self = Self(0);
}

impl SlotIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl ActionId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl WorkflowDigest {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl EventSeq {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

// ============================================================================
// UnsupportedRecoveryState mirror — types.rs:821-832
// ============================================================================

/// Mirror of `UnsupportedRecoveryState` at
/// `crates/vb_storage/src/recovery/types.rs:821-832`. All four fields
/// are `bool` so the mirror is bit-identical to production.
///
/// `PartialEq, Eq` are intentionally NOT derived here because the
/// macro-generated `discriminant_value` call is not supported by
/// Verus 0.2026.05.05 (Rust 1.95.0). Spec proofs compare via the
/// bridge exec fns and recovery-cannot-resume proofs, which take the
/// relevant flags as primitive arguments.
#[derive(Clone, Copy)]
pub struct UnsupportedRecoveryState {
    pub slot_values: bool,
    pub slot_taint: bool,
    pub action_payloads: bool,
    pub pending_actions: bool,
}

impl UnsupportedRecoveryState {
    /// Mirror of `UnsupportedRecoveryState::SUPPORTED` at
    /// `types.rs:667-672`.
    pub const SUPPORTED: Self = Self {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    };

    /// Mirror of `is_fully_supported` at `types.rs:714-716`. Production
    /// returns true iff all four flags are false. The spec proof
    /// `proof_no_rejection_when_supported` discharges the 13-flag
    /// runtime cannot-resume contract separately.
    pub const fn is_fully_supported(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
    }
}

// ============================================================================
// Recovered*State mirrors — types.rs:610-650
// ============================================================================

/// Mirror of `RecoveredStepState` at
/// `crates/vb_storage/src/recovery/types.rs:776-790`. The discriminant
/// set is mirrored exactly; production uses `#[non_exhaustive]` but
/// the spec projection enumerates the closed set the proofs reason
/// about.
///
/// `PartialEq, Eq` are intentionally NOT derived: the macro expansion
/// triggers `core::intrinsics::discriminant_value`, which Verus
/// 0.2026.05.05 does not support. Spec proofs reason via bridge exec
/// fns over primitive bools.
#[derive(Clone, Copy)]
pub enum RecoveredStepState {
    Running,
    Succeeded,
    Failed,
    Waiting,
    Asking,
}

/// Mirror of `RecoveredStepEntry` at `types.rs:792-799`.
#[derive(Clone, Copy)]
pub struct RecoveredStepEntry {
    pub step: StepIdx,
    pub state: RecoveredStepState,
}

/// Mirror of `RecoveredSlotEntry` at `types.rs:801-810`.
#[derive(Clone, Copy)]
pub struct RecoveredSlotEntry {
    pub slot: SlotIdx,
    /// Slot value placeholder. Verus does not model `SlotValue`
    /// (it is an algebraic-data-type over `bool`, integers, strings,
    /// etc.); the spec projection treats value as opaque and reasons
    /// only about whether the slot write was in-bounds.
    pub value: (),
    /// Taint marker placeholder. Production `Taint` is a
    /// `Copy`-able newtype; the spec treats it as opaque.
    pub taint: (),
}

/// Mirror of `RecoveredPendingAction` at `types.rs:812-819`.
#[derive(Clone, Copy)]
pub struct RecoveredPendingAction {
    pub step: StepIdx,
    pub action: ActionId,
}

// ============================================================================
// RecoveryTerminalState + RecoveryRuntimeSummary + RecoveryHydration
// mirrors — types.rs:547-645
// ============================================================================

/// Mirror of `RecoveryTerminalState` at `types.rs:547-562`. Closed
/// projection: production uses `#[non_exhaustive]` but the spec
/// only needs the four documented variants.
///
/// `PartialEq, Eq` intentionally NOT derived (see
/// `UnsupportedRecoveryState` rationale).
#[derive(Clone, Copy)]
pub enum RecoveryTerminalState {
    Cancelled,
    Killed,
    Failed,
    Finished { result: SlotIdx },
}

/// Mirror of `RecoveryRuntimeSummary` at `types.rs:564-589`. All
/// fields are `Copy` primitives; the mirror is bit-identical.
///
/// `PartialEq, Eq` intentionally NOT derived (see
/// `UnsupportedRecoveryState` rationale).
#[derive(Clone, Copy)]
pub struct RecoveryRuntimeSummary {
    pub run: RunId,
    pub first_seq: EventSeq,
    pub last_seq: EventSeq,
    pub workflow: Option<WorkflowDigest>,
    pub steps_started: u64,
    pub steps_succeeded: u64,
    pub actions_scheduled: u64,
    pub actions_resolved: u64,
    pub suspensions: u64,
    pub slots_written: u64,
    pub terminal: Option<RecoveryTerminalState>,
}

/// Mirror of `RecoveryFrameSeed` at `types.rs:925-946`. The `Vec`
/// fields are abstracted to length counters because Verus cannot
/// model `Vec<T>` heap storage. The length counters are the
/// production-derived counts that the spec decision fns reason about.
///
/// `Clone, PartialEq, Eq` intentionally NOT derived (see
/// `UnsupportedRecoveryState` rationale; spec proofs reason over
/// primitive flags rather than cloning the seed).
pub struct RecoveryFrameSeed {
    pub summary: RecoveryRuntimeSummary,
    pub first_step: StepIdx,
    pub step_count: u16,
    pub slot_count: u16,
    pub pc: StepIdx,
    /// Number of recovered step entries (mirrors `steps: Vec<RecoveredStepEntry>`
    /// at types.rs:939). The spec projection reasons only about the
    /// count.
    pub steps_len: usize,
    /// Number of recovered slot entries (mirrors `slots: Vec<RecoveredSlotEntry>`
    /// at types.rs:941). The spec projection reasons only about the
    /// count.
    pub slots_len: usize,
    /// Number of pending actions (mirrors `pending_actions: Vec<RecoveredPendingAction>`
    /// at types.rs:943). The spec projection reasons only about the
    /// count.
    pub pending_actions_len: usize,
    pub unsupported: UnsupportedRecoveryState,
}

/// Mirror of `NonResumableRecoveryFrameSeedProduct` at `types.rs`.
pub struct NonResumableRecoveryFrameSeedProduct {
    pub seed: RecoveryFrameSeed,
    pub cannot_resume: RecoveryCannotResumeState,
}

/// Mirror of `ResumableRecoveryFrameSeedProduct` at `types.rs`.
pub struct ResumableRecoveryFrameSeedProduct {
    pub seed: RecoveryFrameSeed,
}

/// Mirror of `RecoveryFrameSeedProduct` at `types.rs`. The mirror keeps the
/// storage product split between cannot-resume and resumable products.
pub enum RecoveryFrameSeedProduct {
    CannotResume(NonResumableRecoveryFrameSeedProduct),
    Resumable(ResumableRecoveryFrameSeedProduct),
}

/// Mirror of `RecoveryHydration` at `types.rs`. Production uses
/// `#[non_exhaustive]` with two documented variants; the spec mirrors
/// both.
///
/// `Clone, PartialEq, Eq` intentionally NOT derived (see
/// `UnsupportedRecoveryState` rationale; spec proofs reason over the
/// `summary_boundary_hydrate_pure` projection which does not
/// instantiate this enum).
pub enum RecoveryHydration {
    Summary(RecoveryRuntimeSummary),
    FrameSeed(RecoveryFrameSeedProduct),
}

impl RecoveryHydration {
    /// Mirror of `RecoveryHydration::summary` at `types.rs:599-604`.
    /// Pure projection: returns the summary regardless of variant.
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        match self {
            Self::Summary(s) => *s,
            Self::FrameSeed(product) => match product {
                RecoveryFrameSeedProduct::CannotResume(inner) => inner.seed.summary,
                RecoveryFrameSeedProduct::Resumable(inner) => inner.seed.summary,
            },
        }
    }
}

// ============================================================================
// RecoveryCannotResumeState mirror — types.rs:1063-1098 (FINDING-001, vb-wy33p.11)
// ============================================================================

/// Mirror of [`RecoveryCannotResumeState`] at
/// `crates/vb_storage/src/recovery/types.rs:1063-1098`. The struct
/// carries the same 13 cannot-resume flags as production
/// (FINDING-001 widened the original 7-flag classification to 13 to
/// cover full-RunState-incomplete evidence). Every flag is `bool`,
/// so the mirror is bit-identical.
///
/// `PartialEq, Eq` intentionally NOT derived (see
/// `UnsupportedRecoveryState` rationale; spec proofs compare via the
/// bridge exec fns which take the relevant flags as primitive
/// arguments).
///
/// [`RecoveryCannotResumeState`]: types.rs::RecoveryCannotResumeState
#[derive(Clone, Copy)]
pub struct RecoveryCannotResumeState {
    /// Mirror of production `slot_values: bool` at types.rs:902.
    pub slot_values: bool,
    /// Mirror of production `slot_taint: bool` at types.rs:904.
    pub slot_taint: bool,
    /// Mirror of production `action_payloads: bool` at types.rs:906.
    pub action_payloads: bool,
    /// Mirror of production `pending_actions: bool` at types.rs:908.
    pub pending_actions: bool,
    /// Mirror of production `pending_timers: bool` at types.rs:910.
    pub pending_timers: bool,
    /// Mirror of production `pending_asks: bool` at types.rs:912.
    pub pending_asks: bool,
    /// Mirror of production `workflow_missing: bool` at types.rs:914.
    pub workflow_missing: bool,
    /// Mirror of production `store_missing: bool` at types.rs:916.
    pub store_missing: bool,
    /// Mirror of production `action_attempts_missing: bool` at types.rs:918.
    pub action_attempts_missing: bool,
    /// Mirror of production `admission_missing: bool` at types.rs:920.
    pub admission_missing: bool,
    /// Mirror of production `collect_states_missing: bool` at types.rs:922.
    pub collect_states_missing: bool,
    /// Mirror of production `action_contracts_missing: bool` at types.rs:924.
    pub action_contracts_missing: bool,
    /// Mirror of production `action_abi_digests_missing: bool` at types.rs:926.
    pub action_abi_digests_missing: bool,
}

impl RecoveryCannotResumeState {
    /// Mirror of production `RecoveryCannotResumeState::RESUMABLE`
    /// at `types.rs:931-945`. All 13 cannot-resume flags are false.
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

    /// Mirror of `RecoveryCannotResumeState::is_resumable()` at
    /// `types.rs:1192-1208`. Returns true iff every flag is false.
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

    /// Mirror of the production `from_seed` decision fn at
    /// `types.rs:1118-1126`. Returns true iff the production body
    /// calls `mark_full_run_state_missing()` and any of the 7
    /// `*_missing` flags is therefore true.
    ///
    /// DRIFT D3 (binding ledger): production always sets the 7
    /// `*_missing` flags to true (a frame seed alone never carries
    /// the full RunState), so `is_resumable()` always returns false
    /// for frame seeds. The mirror exposes this as a guarantee via
    /// `proof_classify_seed_marks_all_full_state_missing` in
    /// `recovery_verification.rs`.
    #[verifier::external]
    pub const fn from_seed_pure(_seed: RecoveryFrameSeed) -> RecoveryCannotResumeState {
        Self {
            slot_values: false,
            slot_taint: false,
            action_payloads: false,
            pending_actions: false,
            pending_timers: false,
            pending_asks: false,
            workflow_missing: true,
            store_missing: true,
            action_attempts_missing: true,
            admission_missing: true,
            collect_states_missing: true,
            action_contracts_missing: true,
            action_abi_digests_missing: true,
        }
    }

    /// Mirror of `RecoveryCannotResumeState::unsupported_reason()`
    /// at `types.rs:1251-1266`. Returns the priority-ordered first
    /// matching canonical reason token, or
    /// `"resumable"` if every flag is false.
    #[verifier::external]
    pub const fn unsupported_reason_pure(self) -> &'static str {
        // Verus treats the body as opaque. The proof in
        // `recovery_verification.rs` exposes the priority ordering
        // via `proof_classify_seed_priority`.
        "resumable"
    }
}

/// Canonical reason strings for [`RecoveryCannotResumeState`], ordered
/// by classification priority (the first true flag wins). The order
/// MUST match [`RecoveryCannotResumeState`]'s flag-accessor
/// contract.
///
/// DRIFT D3 (binding ledger): the production array is named
/// `CANNOT_RESUME_REASONS` at
/// `crates/vb_storage/src/recovery/types.rs:1213-1227`. The mirror
/// redeclares it in the extern layer because `crate::recovery::types`
/// is not reachable from this standalone `verus --crate-type=lib`
/// invocation (see "WHY NOT FULL `#[path]` INCLUSION OF PRODUCTION
/// SOURCES" at the top of the companion file). The spec proof
/// `proof_unsupported_reason_first_match_wins` in
/// `recovery_verification.rs` demonstrates the priority invariant
/// over this mirror.
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

// ============================================================================
// Digest* mirrors — types.rs:346-526, 1606-1652
// ============================================================================

/// Mirror of `DigestPair` at `types.rs:346-358`.
#[derive(Clone, Copy)]
pub struct DigestPair {
    pub expected: WorkflowDigest,
    pub found: WorkflowDigest,
}

impl DigestPair {
    pub const fn new(expected: WorkflowDigest, found: WorkflowDigest) -> Self {
        Self { expected, found }
    }
}

/// Mirror of `ActionAbiDigestComparison` at `types.rs:363-379`.
#[derive(Clone, Copy)]
pub struct ActionAbiDigestComparison {
    pub action_id: ActionId,
    pub digest: DigestPair,
}

/// Mirror of `PolicyDigestComparison` at `types.rs:383-399`.
#[derive(Clone, Copy)]
pub struct PolicyDigestComparison {
    pub step: StepIdx,
    pub digest: DigestPair,
}

/// Mirror of `FullDigestEvidence<'a>` at `types.rs:403-456`.
/// `()` is used in place of the slice types because Verus does not
/// model lifetime-bound slice iterators; the spec decision fn takes
/// pre-computed "all match" flags instead.
#[derive(Clone, Copy)]
pub struct FullDigestEvidence {
    pub action_abi_all_match: bool,
    pub policy_all_match: bool,
}

/// Mirror of `DigestCheck` (the `DigestCheckLevel` analog) at
/// `types.rs:1422-1429`. Production uses `#[non_exhaustive]`; the spec
/// projection enumerates the closed three-level hierarchy.
#[derive(Clone, Copy)]
pub enum DigestCheck {
    WorkflowSourceOnly,
    WorkflowAndIr,
    Full,
}

impl DigestCheck {
    /// Mirror of `hierarchy_rank` at `types.rs:1434-1440`.
    pub const fn hierarchy_rank(self) -> u8 {
        match self {
            Self::WorkflowSourceOnly => 1,
            Self::WorkflowAndIr => 2,
            Self::Full => 3,
        }
    }

    /// Mirror of `checks_workflow_source` at `types.rs:1444-1446`.
    pub const fn checks_workflow_source(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowSourceOnly.hierarchy_rank()
    }

    /// Mirror of `checks_compiled_ir` at `types.rs:1450-1452`.
    pub const fn checks_compiled_ir(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowAndIr.hierarchy_rank()
    }

    /// Mirror of `checks_full` at `types.rs:1456-1458`.
    pub const fn checks_full(self) -> bool {
        self.hierarchy_rank() >= Self::Full.hierarchy_rank()
    }
}

/// Mirror of `DigestVerificationRequest<'a>` at `types.rs:460-526`.
/// The spec projection replaces the slice evidence with the
/// pre-computed "all match" flags so the decision surface is closed
/// and Verus-tractable.
#[derive(Clone, Copy)]
pub enum DigestVerificationRequest {
    WorkflowSourceOnly {
        expected_workflow_digest: WorkflowDigest,
    },
    WorkflowAndIr {
        expected_workflow_digest: WorkflowDigest,
        expected_ir_digest: WorkflowDigest,
        found_ir_digest: WorkflowDigest,
    },
    Full {
        expected_workflow_digest: WorkflowDigest,
        expected_ir_digest: WorkflowDigest,
        found_ir_digest: WorkflowDigest,
        evidence: FullDigestEvidence,
    },
}

// ============================================================================
// RecoveryError mirror (spec subset) — types.rs:39-158
// ============================================================================

/// Spec-mirror subset of `RecoveryError` at
/// `crates/vb_storage/src/recovery/types.rs:39-158`. Field shape
/// matches production per variant; only the variants the spec
/// proofs reason about are included.
///
/// `PartialEq, Eq` intentionally NOT derived (see
/// `UnsupportedRecoveryState` rationale).
#[derive(Clone, Copy)]
pub enum RecoveryError {
    WorkflowSourceDigestMismatch {
        expected: WorkflowDigest,
        found: WorkflowDigest,
    },
    CompiledIrDigestMismatch {
        expected: WorkflowDigest,
        found: WorkflowDigest,
    },
    FrameDimensionOverflow {
        run: RunId,
    },
    /// Mirror of `RecoveryError::UnsupportedFrameSeed { run, reason }`
    /// at `crates/vb_storage/src/recovery/types.rs:151-157`.
    ///
    /// DRIFT D3 (binding ledger): production `reason` is `String`
    /// (alloc-bearing) carrying one of 13 canonical reason constants.
    /// Verus 0.2026.05.05 cannot reason
    /// about `String` because the `RecoveryError` enum derives
    /// `Copy` (the closure of pre-existing analyses requires it),
    /// so the spec projection models the reason as `&'static str`
    /// — the same 13 canonical tokens exposed by the production
    /// gate. The drift is documented and tracked; the spec-side
    /// proof only consumes the value via the priority-ordered
    /// lookup table, never via direct `String` ops.
    UnsupportedFrameSeed {
        run: RunId,
        reason: &'static str,
    },
}

pub type RecoveryResult<T> = Result<T, RecoveryError>;

// ============================================================================
// RuntimeError mirror (spec subset) — crates/vb_runtime/src/error/mod.rs
// ============================================================================

/// Spec-mirror subset of `RuntimeError`. Field shape matches
/// production per variant. `FrameDimensionOverflow` is modeled as
/// a typed variant for the spec-side proof surface even though
/// production runtime does not emit it; the refinement proof
/// `proof_recovery_error_refines_to_runtime_error` captures the
/// collapse semantics via explicit refinement clauses.
///
/// `PartialEq, Eq` intentionally NOT derived (see
/// `UnsupportedRecoveryState` rationale).
#[derive(Clone, Copy)]
pub enum RuntimeError {
    InvalidRecoveryHydration,
    UnsupportedFullRecoveryHydration,
    WorkflowSourceDigestMismatch,
    CompiledIrDigestMismatch,
    FrameDimensionOverflow,
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;
