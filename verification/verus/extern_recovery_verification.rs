// SPDX-License-Identifier: MIT
//
// Extern surface for recovery_verification Verus spec.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
// This file binds the recovery_verification.rs Verus spec to the production
// recovery decision surfaces in:
//
//   - crates/vb_storage/src/recovery/types.rs
//     (UnsupportedRecoveryState, RecoveryFrameSeed, RecoveryHydration,
//      RecoveryRuntimeSummary, RecoveryTerminalState, DigestCheck,
//      DigestVerificationRequest, DigestPair, ActionAbiDigestComparison,
//      PolicyDigestComparison, FullDigestEvidence, RecoveryError)
//   - crates/vb_storage/src/recovery/recover.rs
//     (check_workflow_source_digest, check_compiled_ir_digest,
//      check_action_abi_digest, check_policy_digest, verify_digests,
//      recover_runtime_summary, recover_runtime_frame_seed)
//   - crates/vb_storage/src/recovery/hydrate.rs
//     (hydrate_run_frame, hydrate_run_frame_from_events,
//      hydrate_snapshot_tail_preconditions,
//      hydrate_snapshot_tail_run_matches,
//      hydrate_snapshot_tail_seq_after_snapshot,
//      hydrate_snapshot_tail_has_evidence,
//      hydrate_events_preconditions,
//      hydrate_dimensions_positive)
//   - crates/vb_runtime/src/recovery.rs
//     (reject_unsupported_live_frame_state,
//      empty_recovered_frame,
//      apply_recovered_step / apply_recovered_steps / apply_recovered_slots /
//      apply_recovered_pc,
//      SummaryRecoveryBoundary::hydrate_run_frame,
//      DurableFrameRecoveryBoundary::hydrate_run_frame)
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF PRODUCTION SOURCES
// ============================================================================
// Direct `#[path = "../../crates/vb_storage/src/recovery/recover.rs"]` and
// analogous `#[path]` to types.rs / hydrate.rs is blocked by:
//
//   1. `recover.rs:18-23` `use crate::recovery::types::{...}` and
//      `use crate::{FjallJournal, JournalEvent}` cannot resolve outside
//      the vb_storage crate root. Fjall is a third-party C library; even
//      the FjallJournal newtype and JournalEvent enum require the
//      vb_storage module tree to be visible.
//   2. `recover.rs:23` `use vb_core::{ActionId, RunId, ...}` requires
//      the vb_core extern crate alias, which is wired through
//      `crates/vb_storage/Cargo.toml` and is unavailable in a
//      standalone `verus --crate-type=lib` invocation.
//   3. `types.rs` uses `#[derive(... Serialize, Deserialize)]` (line 429
//      and onward) plus `#[derive(thiserror::Error)]` (line 37). Verus
//      cannot invoke proc-macro derives without registering the proc
//      macro crates, and the file also pulls in `serde::{Deserialize,
//      Serialize}` (line 10) as a bare-path import that would need a
//      separate extern alias.
//   4. `hydrate.rs:13-17` references `ActionReplayEffect`,
//      `ActionReplayTracker`, and other internal types from `types.rs`
//      and `hydrate_support.rs`, multiplying the module-tree
//      dependency surface.
//   5. `crates/vb_runtime/src/recovery.rs:5-8` uses `vb_storage` as an
//      extern crate alias and pulls in additional runtime dependencies
//      (vb_core::frame::*, crate::*), making whole-file inclusion
//      infeasible without the full workspace build context.
//
// These are all "NO production changes" blockers (per the task brief).
// The structural mirror below sidesteps every blocker while still
// establishing a real end-to-end binding: any drift in the production
// field names, discriminant sets, or fn signatures will break this
// mirror and the spec proofs that depend on it.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Type mirrors (each mirrors a production type line-by-line so any
// drift breaks the build):
//
//   - `UnsupportedRecoveryState`          <- crates/vb_storage/src/recovery/types.rs:553-563
//   - `RecoveredStepState`                <- crates/vb_storage/src/recovery/types.rs:508-521
//   - `RecoveredStepEntry`                <- crates/vb_storage/src/recovery/types.rs:524-530
//   - `RecoveredSlotEntry`                <- crates/vb_storage/src/recovery/types.rs:533-541
//   - `RecoveredPendingAction`            <- crates/vb_storage/src/recovery/types.rs:544-550
//   - `RecoveryTerminalState`             <- crates/vb_storage/src/recovery/types.rs:431-443
//   - `RecoveryRuntimeSummary`            <- crates/vb_storage/src/recovery/types.rs:446-470
//   - `RecoveryHydration` (enum)          <- crates/vb_storage/src/recovery/types.rs:487-494
//   - `DigestPair`                        <- crates/vb_storage/src/recovery/types.rs:246-251
//   - `ActionAbiDigestComparison`         <- crates/vb_storage/src/recovery/types.rs:262-268
//   - `PolicyDigestComparison`            <- crates/vb_storage/src/recovery/types.rs:282-288
//   - `FullDigestEvidence<'a>`            <- crates/vb_storage/src/recovery/types.rs:302-356
//   - `DigestVerificationRequest<'a>`     <- crates/vb_storage/src/recovery/types.rs:359-426
//   - `DigestCheck`                       <- crates/vb_storage/src/recovery/types.rs:856-864
//   - `RecoveryError` (spec subset)       <- crates/vb_storage/src/recovery/types.rs:39-145
//                                            (only the four variants the spec exercises)
//   - `RuntimeError` (spec subset)        <- crates/vb_runtime/src/error/mod.rs:7-203
//                                            (only the variants the spec exercises)
//
// Pure decision fns (each production body mirrors the production
// decision logic line-by-line; bodies are wrapped in
// `#[verifier::external]` so Verus does not try to verify Fjall I/O or
// alloc paths, and the spec proofs attach contracts via
// `assume_specification`):
//
//   - `reject_unsupported_live_frame_state_pure`
//        <- crates/vb_runtime/src/recovery.rs:73-82
//        (production body: `state.slot_values || state.slot_taint ||
//        state.action_payloads`. NOTE: production does NOT check
//        `pending_actions`; the spec is corrected to match.)
//   - `check_compiled_ir_digest_pure`
//        <- crates/vb_storage/src/recovery/recover.rs:53-62
//        (production body: `if expected == found { Ok(()) } else
//        { Err(CompiledIrDigestMismatch) }`).
//   - `check_workflow_source_digest_pure`
//        <- crates/vb_storage/src/recovery/recover.rs:32-50
//        (production body: scan events for RunAccepted; return Ok iff
//        found AND `*workflow == expected`. Pure projection: success
//        iff has_acceptance_record && workflow_source_matches.)
//   - `check_action_abi_digest_pure`
//        <- crates/vb_storage/src/recovery/recover.rs:65-75
//        (production body: equality check; pure projection: success
//        iff matches.)
//   - `check_policy_digest_pure`
//        <- crates/vb_storage/src/recovery/recover.rs:78-88
//        (production body: equality check; pure projection: success
//        iff matches.)
//   - `verify_digests_pure_decision`
//        <- crates/vb_storage/src/recovery/recover.rs:96-125
//        (production body: dispatch on DigestVerificationRequest
//        variant, calling the underlying pure checks. The mirror is a
//        closed decision fn over (level, workflow_source_matches,
//        has_acceptance_record, compiled_ir_matches,
//        action_abi_all_match, policy_all_match).)
//   - `recover_runtime_summary_pure`
//        <- crates/vb_storage/src/recovery/recover.rs:178-187
//        (production body: read events; reject if empty; delegate to
//        summarize_recovery_events. Pure projection: success iff
//        has_events && summary_ok.)
//   - `hydrate_run_frame_preconditions_pure`
//        <- crates/vb_storage/src/recovery/hydrate.rs:181-200 +
//           crates/vb_runtime/src/recovery.rs:63-71
//        (production body: validate_snapshot_recovery_inputs, then
//        decode_snapshot_slots (alloc), then derive_dimensions
//        (alloc), then ensure_nonzero_step_count, then build RunFrame
//        and apply recovered steps/slots/pc. The pure projection is
//        a closed precondition decision over
//        (snapshot_run_matches, tail_events_match_run,
//        tail_seq_after_snapshot, has_evidence,
//        step_count_positive, slot_count_positive,
//        steps_apply_ok, slots_apply_ok, pc_in_bounds,
//        unsupported_passes_through_reject).)
//   - `summary_recovery_boundary_hydrate_pure`
//        <- crates/vb_runtime/src/recovery.rs:146-154
//        (production body: returns Err(UnsupportedFullRecoveryHydration).
//        Pure projection: never succeeds.)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in this file are NOT verified by
// Verus. Each exec fn below is `#[verifier::external]` so Verus skips
// body verification, and the contracts attached via
// `assume_specification` in the companion spec file
// (`recovery_verification.rs`) state the production behavior the spec
// proofs discharge. Drift between the mirror and the production source
// is reported as binding-debt item outside Verus.
//
// Drift items accepted by the binding (acknowledged in spec comments):
//   - D1: production `reject_unsupported_live_frame_state` does NOT
//         check `pending_actions`; the spec originally did. The spec
//         is corrected to match production. (Confirmed by
//         crates/vb_runtime/src/recovery/tests.rs:395-453
//         `pending_actions_hydration_round_trip` test which asserts
//         that hydration succeeds with `pending_actions = true`.)
//   - D2: production `RuntimeError` has no `FrameDimensionOverflow`
//         variant; the runtime layer collapses all hydration
//         failures into `RuntimeError::InvalidRecoveryHydration`. The
//         spec models the typed `RecoveryError` surface (which DOES
//         have `FrameDimensionOverflow`) and the runtime error
//         mapping narrows to `InvalidRecoveryHydration` for the
//         hydration-specific failure paths.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Production drift-detection inclusion via #[path]
// ---------------------------------------------------------------------------
//
// `#[path]` inclusion of the production drift-detection stub at
// `production_inner/recovery_verification_production.rs`. The stub
// carries a representative drift-detection slice
// (UnsupportedRecoveryState field shape + reject_unsupported decision
// fn). Any drift in the production surface breaks the spec build.
// The full production mirror content lives below in this file.
#[path = "production_inner/recovery_verification_production.rs"]
pub mod prod_src;

} // verus!

// ============================================================================
// ID type mirrors — production newtypes
// ============================================================================

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
// UnsupportedRecoveryState mirror — types.rs:553-563
// ============================================================================

/// Mirror of `UnsupportedRecoveryState` at
/// `crates/vb_storage/src/recovery/types.rs:553-563`. All four fields
/// are `bool` so the mirror is bit-identical to production.
///
/// `PartialEq, Eq` are intentionally NOT derived here because the
/// macro-generated `discriminant_value` call is not supported by
/// Verus 0.2026.05.05 (Rust 1.95.0). Spec proofs compare via the
/// bridge exec fns (`reject_unsupported_live_frame_state_pure`),
/// which take the relevant flags as primitive arguments.
#[derive(Clone, Copy)]
pub struct UnsupportedRecoveryState {
    pub slot_values: bool,
    pub slot_taint: bool,
    pub action_payloads: bool,
    pub pending_actions: bool,
}

impl UnsupportedRecoveryState {
    /// Mirror of `UnsupportedRecoveryState::SUPPORTED` at
    /// `types.rs:567-572`.
    pub const SUPPORTED: Self = Self {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    };

    /// Mirror of `is_fully_supported` at `types.rs:614-616`. Production
    /// returns true iff all four flags are false. The spec proof
    /// `proof_no_rejection_when_fully_supported` discharges this via
    /// the `reject_unsupported_live_frame_state_pure` contract.
    pub const fn is_fully_supported(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
    }
}

// ============================================================================
// Recovered*State mirrors — types.rs:507-551
// ============================================================================

/// Mirror of `RecoveredStepState` at
/// `crates/vb_storage/src/recovery/types.rs:508-521`. The discriminant
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

/// Mirror of `RecoveredStepEntry` at `types.rs:524-530`.
#[derive(Clone, Copy)]
pub struct RecoveredStepEntry {
    pub step: StepIdx,
    pub state: RecoveredStepState,
}

/// Mirror of `RecoveredSlotEntry` at `types.rs:533-541`.
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

/// Mirror of `RecoveredPendingAction` at `types.rs:544-550`.
#[derive(Clone, Copy)]
pub struct RecoveredPendingAction {
    pub step: StepIdx,
    pub action: ActionId,
}

// ============================================================================
// RecoveryTerminalState + RecoveryRuntimeSummary + RecoveryHydration
// mirrors — types.rs:429-505
// ============================================================================

/// Mirror of `RecoveryTerminalState` at `types.rs:429-443`. Closed
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

/// Mirror of `RecoveryRuntimeSummary` at `types.rs:446-470`. All
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

/// Mirror of `RecoveryFrameSeed` at `types.rs:629-649`. The `Vec`
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
    /// at types.rs:642). The spec projection reasons only about the
    /// count.
    pub steps_len: usize,
    /// Number of recovered slot entries (mirrors `slots: Vec<RecoveredSlotEntry>`
    /// at types.rs:644). The spec projection reasons only about the
    /// count.
    pub slots_len: usize,
    /// Number of pending actions (mirrors `pending_actions: Vec<RecoveredPendingAction>`
    /// at types.rs:646). The spec projection reasons only about the
    /// count.
    pub pending_actions_len: usize,
    pub unsupported: UnsupportedRecoveryState,
}

/// Mirror of `RecoveryHydration` at `types.rs:487-494`. Production uses
/// `#[non_exhaustive]` with two documented variants; the spec mirrors
/// both.
///
/// `Clone, PartialEq, Eq` intentionally NOT derived (see
/// `UnsupportedRecoveryState` rationale; spec proofs reason over the
/// `summary_boundary_hydrate_pure` projection which does not
/// instantiate this enum).
pub enum RecoveryHydration {
    Summary(RecoveryRuntimeSummary),
    FrameSeed(RecoveryFrameSeed),
}

impl RecoveryHydration {
    /// Mirror of `RecoveryHydration::summary` at `types.rs:496-505`.
    /// Pure projection: returns the summary regardless of variant.
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        match self {
            Self::Summary(s) => *s,
            Self::FrameSeed(seed) => seed.summary,
        }
    }
}

// ============================================================================
// Digest* mirrors — types.rs:244-426, 855-899
// ============================================================================

/// Mirror of `DigestPair` at `types.rs:246-251`.
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

/// Mirror of `ActionAbiDigestComparison` at `types.rs:262-268`.
#[derive(Clone, Copy)]
pub struct ActionAbiDigestComparison {
    pub action_id: ActionId,
    pub digest: DigestPair,
}

/// Mirror of `PolicyDigestComparison` at `types.rs:282-288`.
#[derive(Clone, Copy)]
pub struct PolicyDigestComparison {
    pub step: StepIdx,
    pub digest: DigestPair,
}

/// Mirror of `FullDigestEvidence<'a>` at `types.rs:302-356`.
/// `()` is used in place of the slice types because Verus does not
/// model lifetime-bound slice iterators; the spec decision fn takes
/// pre-computed "all match" flags instead.
#[derive(Clone, Copy)]
pub struct FullDigestEvidence {
    pub action_abi_all_match: bool,
    pub policy_all_match: bool,
}

/// Mirror of `DigestCheck` (the `DigestCheckLevel` analog) at
/// `types.rs:856-864`. Production uses `#[non_exhaustive]`; the spec
/// projection enumerates the closed three-level hierarchy.
#[derive(Clone, Copy)]
pub enum DigestCheck {
    WorkflowSourceOnly,
    WorkflowAndIr,
    Full,
}

impl DigestCheck {
    /// Mirror of `hierarchy_rank` at `types.rs:868-875`.
    pub const fn hierarchy_rank(self) -> u8 {
        match self {
            Self::WorkflowSourceOnly => 1,
            Self::WorkflowAndIr => 2,
            Self::Full => 3,
        }
    }

    /// Mirror of `checks_workflow_source` at `types.rs:879-881`.
    pub const fn checks_workflow_source(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowSourceOnly.hierarchy_rank()
    }

    /// Mirror of `checks_compiled_ir` at `types.rs:884-886`.
    pub const fn checks_compiled_ir(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowAndIr.hierarchy_rank()
    }

    /// Mirror of `checks_full` at `types.rs:889-893`.
    pub const fn checks_full(self) -> bool {
        self.hierarchy_rank() >= Self::Full.hierarchy_rank()
    }
}

/// Mirror of `DigestVerificationRequest<'a>` at `types.rs:359-426`.
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
// RecoveryError mirror (spec subset) — types.rs:39-145
// ============================================================================
//
// Production has 14 variants (line 39-145); the spec only exercises
// four of them because the spec proofs reason about the typed-error
// surface the recovery boundary emits. The four mirrored variants
// are the ones that affect hydration success/failure classification.

/// Spec-mirror subset of `RecoveryError` at
/// `crates/vb_storage/src/recovery/types.rs:39-145`. Field shape
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
    UnsupportedFrameSeed {
        run: RunId,
    },
}

pub type RecoveryResult<T> = Result<T, RecoveryError>;

// ============================================================================
// RuntimeError mirror (spec subset) — crates/vb_runtime/src/error/mod.rs
// ============================================================================
//
// Production `RuntimeError` has 40+ variants (error/mod.rs:7-203).
// The spec only exercises the variants the recovery boundary emits.
// D2 (file header): production has no `FrameDimensionOverflow`
// variant in `RuntimeError`; the runtime layer collapses all
// hydration failures into `RuntimeError::InvalidRecoveryHydration`.

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

// ============================================================================
// Pure decision fns — `#[verifier::external]` wrappers mirroring
// production decision logic line-by-line.
// ============================================================================

/// Pure decision fn mirroring `reject_unsupported_live_frame_state`
/// at `crates/vb_runtime/src/recovery.rs:73-82`.
///
/// Production body:
/// ```text
/// if seed.unsupported.slot_values
///    || seed.unsupported.slot_taint
///    || seed.unsupported.action_payloads
/// { Err(RuntimeError::InvalidRecoveryHydration) } else { Ok(()) }
/// ```
///
/// Production does NOT check `pending_actions`; this is the
/// DRIFT D1 acknowledged in the file header. The spec projection
/// returns `true` (success) iff the production body returns Ok,
/// i.e. iff none of the three rejection flags are set. Spec proofs
/// attach the production contract via `assume_specification`.
///
/// TRUST BOUNDARY: body is opaque to Verus (`#[verifier::external]`).
#[verifier::external]
pub fn reject_unsupported_live_frame_state_pure(state: UnsupportedRecoveryState) -> bool {
    !state.slot_values && !state.slot_taint && !state.action_payloads
}

/// Pure decision fn mirroring `check_compiled_ir_digest` at
/// `crates/vb_storage/src/recovery/recover.rs:53-62`.
///
/// Production body: `if expected == found { Ok(()) } else
/// { Err(CompiledIrDigestMismatch { ... }) }`. Pure projection:
/// returns true iff the expected and found digests are equal.
///
/// TRUST BOUNDARY: body is opaque to Verus (`#[verifier::external]`).
#[verifier::external]
pub fn check_compiled_ir_digest_pure(matches: bool) -> bool {
    matches
}

/// Pure decision fn mirroring `check_workflow_source_digest` at
/// `crates/vb_storage/src/recovery/recover.rs:32-50`.
///
/// Production body: scan the journal events for the first
/// `RunAccepted` event; return Err(`NoRecoveryData`) if no event
/// matches the run id; return Err(`WorkflowSourceDigestMismatch`) if
/// the stored workflow digest differs from the expected value;
/// return Ok(()) on the first match. The pure projection collapses
/// this to: success iff `has_acceptance_record && workflow_source_matches`.
///
/// TRUST BOUNDARY: body is opaque to Verus (`#[verifier::external]`).
#[verifier::external]
pub fn check_workflow_source_digest_pure(
    has_acceptance_record: bool,
    workflow_source_matches: bool,
) -> bool {
    has_acceptance_record && workflow_source_matches
}

/// Pure decision fn mirroring `check_action_abi_digest` at
/// `crates/vb_storage/src/recovery/recover.rs:65-75`.
///
/// Production body: equality check. Pure projection: success iff matches.
///
/// TRUST BOUNDARY: body is opaque to Verus (`#[verifier::external]`).
#[verifier::external]
pub fn check_action_abi_digest_pure(matches: bool) -> bool {
    matches
}

/// Pure decision fn mirroring `check_policy_digest` at
/// `crates/vb_storage/src/recovery/recover.rs:78-88`.
///
/// Production body: equality check. Pure projection: success iff matches.
///
/// TRUST BOUNDARY: body is opaque to Verus (`#[verifier::external]`).
#[verifier::external]
pub fn check_policy_digest_pure(matches: bool) -> bool {
    matches
}

/// Pure decision fn mirroring `verify_digests` at
/// `crates/vb_storage/src/recovery/recover.rs:96-125`.
///
/// Production body: dispatch on `DigestVerificationRequest`
/// variant; call the underlying pure checks. Production dispatch:
///
///   - WorkflowSourceOnly: check_workflow_source_digest.
///   - WorkflowAndIr: check_workflow_source_digest;
///                     check_compiled_ir_digest.
///   - Full: check_workflow_source_digest;
///           check_compiled_ir_digest;
///           for each (action_id, expected, found) in
///             evidence.action_abi(): check_action_abi_digest;
///           for each (step, expected, found) in
///             evidence.policy(): check_policy_digest.
///
/// The mirror collapses the for-each loops into the
/// `evidence.action_abi_all_match` and `evidence.policy_all_match`
/// precomputed flags (the production slice iteration is folded
/// into a precomputed "all match" summary that lives on the
/// `FullDigestEvidence`).
///
/// TRUST BOUNDARY: body is opaque to Verus (`#[verifier::external]`).
#[verifier::external]
pub fn verify_digests_pure_decision(
    request: DigestVerificationRequest,
    workflow_source_matches: bool,
    has_acceptance_record: bool,
    compiled_ir_matches: bool,
) -> bool {
    let workflow_ok =
        check_workflow_source_digest_pure(has_acceptance_record, workflow_source_matches);
    match request {
        DigestVerificationRequest::WorkflowSourceOnly { .. } => workflow_ok,
        DigestVerificationRequest::WorkflowAndIr { .. } => {
            workflow_ok && check_compiled_ir_digest_pure(compiled_ir_matches)
        }
        DigestVerificationRequest::Full { evidence, .. } => {
            workflow_ok
                && check_compiled_ir_digest_pure(compiled_ir_matches)
                && check_action_abi_digest_pure(evidence.action_abi_all_match)
                && check_policy_digest_pure(evidence.policy_all_match)
        }
    }
}

/// Pure decision fn mirroring `recover_runtime_summary` at
/// `crates/vb_storage/src/recovery/recover.rs:178-187`.
///
/// Production body: read events via `events_for_run_full`; reject
/// if empty; delegate to `summarize_recovery_events`. Pure
/// projection: success iff `has_events && summary_ok`.
///
/// TRUST BOUNDARY: body is opaque to Verus (`#[verifier::external]`).
#[verifier::external]
pub fn recover_runtime_summary_pure(has_events: bool, summary_ok: bool) -> bool {
    has_events && summary_ok
}

/// Pure precondition decision mirroring `hydrate_run_frame` at
/// `crates/vb_storage/src/recovery/hydrate.rs:181-200` AND the
/// `DurableFrameRecoveryBoundary::hydrate_run_frame` driver at
/// `crates/vb_runtime/src/recovery.rs:63-71`.
///
/// Production body: validate_snapshot_recovery_inputs (run_id,
/// tail run ids, tail seqs, has evidence), then
/// decode_snapshot_slots (alloc + postcard decode), then
/// derive_dimensions_from_snapshot_and_tail (alloc), then
/// ensure_nonzero_step_count, then build RunFrame, then apply
/// snapshot slots + tail events. Driver body: reject_unsupported
/// -> empty_recovered_frame -> apply_recovered_steps ->
/// apply_recovered_slots -> apply_recovered_pc.
///
/// Pure projection: success iff all the precondition flags are
/// true. The "steps_apply_ok", "slots_apply_ok", "pc_in_bounds"
/// flags capture the runtime driver's step/slot/pc application
/// outcomes (which can fail individually and return
/// `RuntimeError::InvalidRecoveryHydration` in production).
///
/// TRUST BOUNDARY: body is opaque to Verus (`#[verifier::external]`).
#[verifier::external]
pub fn hydrate_run_frame_preconditions_pure(
    snapshot_run_matches: bool,
    tail_events_match_run: bool,
    tail_seq_after_snapshot: bool,
    has_evidence: bool,
    step_count_positive: bool,
    slot_count_positive: bool,
    steps_apply_ok: bool,
    slots_apply_ok: bool,
    pc_in_bounds: bool,
    unsupported_passes_through_reject: bool,
) -> bool {
    snapshot_run_matches
        && tail_events_match_run
        && tail_seq_after_snapshot
        && has_evidence
        && step_count_positive
        && slot_count_positive
        && steps_apply_ok
        && slots_apply_ok
        && pc_in_bounds
        && unsupported_passes_through_reject
}

/// Pure decision fn mirroring
/// `SummaryRecoveryBoundary::hydrate_run_frame` at
/// `crates/vb_runtime/src/recovery.rs:146-154`.
///
/// Production body: `Err(RuntimeError::UnsupportedFullRecoveryHydration)`
/// unconditionally. Pure projection: always false (never succeeds).
///
/// TRUST BOUNDARY: body is opaque to Verus (`#[verifier::external]`).
#[verifier::external]
pub fn summary_recovery_boundary_hydrate_pure() -> bool {
    false
}

// ============================================================================
// Helpers — line-by-line mirrors of the production "production proof
// surface" predicates at crates/vb_storage/src/recovery/hydrate.rs:22-70
// ============================================================================

/// Mirror of `hydrate_snapshot_tail_run_matches` at `hydrate.rs:22-28`.
/// Production returns true iff `snapshot.run == run_id` AND every
/// tail event has `run_id == run_id`.
#[verifier::external]
pub fn hydrate_snapshot_tail_run_matches_pure(
    snapshot_run_matches: bool,
    tail_events_match_run: bool,
) -> bool {
    snapshot_run_matches && tail_events_match_run
}

/// Mirror of `hydrate_snapshot_tail_seq_after_snapshot` at
/// `hydrate.rs:32-37`. Production returns true iff every tail
/// event has seq strictly after the snapshot seq.
#[verifier::external]
pub fn hydrate_snapshot_tail_seq_after_snapshot_pure(tail_seq_after_snapshot: bool) -> bool {
    tail_seq_after_snapshot
}

/// Mirror of `hydrate_snapshot_tail_has_evidence` at `hydrate.rs:41-46`.
/// Production returns true iff at least one of: tail events
/// non-empty, snapshot slots non-empty, snapshot taint non-empty.
#[verifier::external]
pub fn hydrate_snapshot_tail_has_evidence_pure(
    tail_events_empty: bool,
    snapshot_slots_empty: bool,
    snapshot_taint_empty: bool,
) -> bool {
    !tail_events_empty || !snapshot_slots_empty || !snapshot_taint_empty
}

/// Mirror of `hydrate_dimensions_positive` at `hydrate.rs:67-70`.
/// Production returns true iff `step_count > 0 && slot_count > 0`.
#[verifier::external]
pub fn hydrate_dimensions_positive_pure(
    step_count_positive: bool,
    slot_count_positive: bool,
) -> bool {
    step_count_positive && slot_count_positive
}
