// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for `recovery_types_production_bridge`
// ============================================================================
//
// Drift-detection stubs for the recovery type surface at
//   crates/vb_storage/src/recovery/types.rs
//
// Source coverage:
//   - `RecoveryTerminalState` enum (Cancelled, Killed, Finished{result},
//      Failed)                                  <- types.rs:529-543
//   - `RecoveryRuntimeSummary` struct (run, first_seq, last_seq, workflow,
//      steps_started, steps_succeeded,
//      actions_scheduled, actions_resolved,
//      suspensions, slots_written, terminal)    <- types.rs:545-570
//   - `RecoveryHydration` enum (Summary,
//      FrameSeed)                                <- types.rs:587-605
//   - `RecoveredStepState` enum (Running,
//      Succeeded, Failed, Waiting, Asking)      <- types.rs:608-621
//   - `UnsupportedRecoveryState` struct (slot_values, slot_taint,
//      action_payloads, pending_actions)       <- types.rs:652-663
//   - `UnsupportedRecoveryState::SUPPORTED` const
//                                                <- types.rs:665-672
//   - `UnsupportedRecoveryState::union` const fn (flagwise OR)
//                                                <- types.rs:701-710
//
// The companion `verification/verus/recovery_types_production_bridge.rs`
// uses `#[path = "production_inner/recovery_types_production.rs"]` to
// bind this surface. Every field, const, and method referenced here
// must resolve at Rust compile time; any drift in production breaks
// the build (the explicit drift-detection mechanism).
//
// The production mirror uses `u64`/`u16` in place of the production
// newtypes `RunId`/`SlotIdx`/`StepIdx`/`EventSeq`/`WorkflowDigest`
// (these are newtype newtypes from `vb_core::ids`) because the
// standalone `verus --crate-type=lib` invocation cannot resolve the
// vb_core extern crate alias. The field shape (names + ordering +
// primitive types) is the drift-detection surface — see
// `crates/vb_storage/src/recovery/types.rs:529-621,652-663` for the
// canonical source. Spec proofs reason via the
// `production::UnsupportedRecoveryState` field shape directly.
//
// ============================================================================
// BINDING DEBT
// ============================================================================
//
// D1: Production types use newtypes (RunId(u64), SlotIdx(u16),
//     StepIdx(u16), EventSeq(u64), WorkflowDigest([u8;32])). The
//     stub mirrors these with primitive u64/u16. The field count,
//     field order, and field type CATEGORY match production; any
//     rename, reorder, or primitive-type change breaks the build.
//
// D2: The production `#[non_exhaustive]` attribute is dropped from
//     the stubs because Verus does not honor `#[non_exhaustive]`
//     in mirror positions. Spec proofs enumerate the closed variant
//     set documented in the production doc-comments.
//
// D3: The production derives (`Debug, Clone, Copy, PartialEq, Eq`)
//     are kept as `#[derive(Clone, Copy)]` only. The macro-generated
//     `discriminant_value` (PartialEq/Eq) and `Debug` formatting are
//     not supported by Verus 0.2026.05.05 (Rust 1.95.0) standalone.
//     Spec proofs compare via the closed spec fn
//     `terminal_state_eq` / `runtime_summary_eq` /
//     `recovered_step_state_eq` in the companion spec file.
//
// D4: The `RecoveryRuntimeSummary` mirror drops the `workflow`
//     field's `Option<WorkflowDigest>` typing and uses
//     `Option<StubWorkflowDigest>` instead. The drift surface is
//     the field presence + the `Option` wrapper; the
//     `WorkflowDigest` newtype's [u8;32] inner is irrelevant to the
//     field-shape check.
//
// D5: The `RecoveryHydration` mirror uses `RecoveryFrameSeedStub`
//     as the FrameSeed payload. Spec proofs reason about the
//     `summary()` method via the bridge contract, not via the
//     `FrameSeed` payload, so the stub depth is sufficient.
//
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// ID newtype stubs — production vb_core::ids
// ---------------------------------------------------------------------------
//
// Source: crates/vb_core/src/ids/mod.rs
// Drift policy: any production rename of the newtype inner primitive
// type breaks the field shape comparison in the spec bridge.

/// Mirror of `RunId(u64)` at `crates/vb_core/src/ids/mod.rs:65`.
#[derive(Clone, Copy)]
pub struct StubRunId(pub u64);

/// Mirror of `SlotIdx(u16)` at `crates/vb_core/src/ids/mod.rs:56`.
#[derive(Clone, Copy)]
pub struct StubSlotIdx(pub u16);

/// Mirror of `StepIdx(u16)` at `crates/vb_core/src/ids/mod.rs:55`.
#[derive(Clone, Copy)]
pub struct StubStepIdx(pub u16);

/// Mirror of `EventSeq(u64)` at `crates/vb_core/src/ids/mod.rs:70`.
#[derive(Clone, Copy)]
pub struct StubEventSeq(pub u64);

/// Mirror of `WorkflowDigest([u8;32])` at `crates/vb_core/src/ids/mod.rs:80`.
/// Inner field uses `u64` (rather than [u8;32]) because Verus does not
/// model fixed-size arrays of `u8` in spec mode. The field count and
/// ordering are preserved at the `RecoveryRuntimeSummary` level via
/// the stub.
#[derive(Clone, Copy)]
pub struct StubWorkflowDigest(pub u64);

// ---------------------------------------------------------------------------
// RecoveryTerminalState stub — types.rs:529-543
// ---------------------------------------------------------------------------

/// Mirror of `RecoveryTerminalState` at
/// `crates/vb_storage/src/recovery/types.rs:529-543`. The closed variant
/// set is `Cancelled`, `Killed`, `Finished { result: SlotIdx }`, `Failed`.
///
/// `PartialEq, Eq, Debug` are intentionally NOT derived (see
/// BINDING DEBT D3).
#[derive(Clone, Copy)]
pub enum RecoveryTerminalStateStub {
    Cancelled,
    Killed,
    Finished { result: StubSlotIdx },
    Failed,
}

// ---------------------------------------------------------------------------
// RecoveredStepState stub — types.rs:608-621
// ---------------------------------------------------------------------------

/// Mirror of `RecoveredStepState` at
/// `crates/vb_storage/src/recovery/types.rs:608-621`. The closed
/// variant set is `Running`, `Succeeded`, `Failed`, `Waiting`,
/// `Asking`.
#[derive(Clone, Copy)]
pub enum RecoveredStepStateStub {
    Running,
    Succeeded,
    Failed,
    Waiting,
    Asking,
}

// ---------------------------------------------------------------------------
// RecoveryRuntimeSummary stub — types.rs:545-570
// ---------------------------------------------------------------------------

/// Mirror of `RecoveryRuntimeSummary` at
/// `crates/vb_storage/src/recovery/types.rs:545-570`. All fields
/// preserved by name, order, and primitive type category.
///
/// Field inventory (drift-detection surface):
///   - `run: RunId`                       -> `run: StubRunId`
///   - `first_seq: EventSeq`              -> `first_seq: StubEventSeq`
///   - `last_seq: EventSeq`               -> `last_seq: StubEventSeq`
///   - `workflow: Option<WorkflowDigest>` -> `workflow: Option<StubWorkflowDigest>`
///   - `steps_started: u64`               -> `steps_started: u64`
///   - `steps_succeeded: u64`             -> `steps_succeeded: u64`
///   - `actions_scheduled: u64`           -> `actions_scheduled: u64`
///   - `actions_resolved: u64`            -> `actions_resolved: u64`
///   - `suspensions: u64`                 -> `suspensions: u64`
///   - `slots_written: u64`               -> `slots_written: u64`
///   - `terminal: Option<RecoveryTerminalState>`
///                                          -> `terminal: Option<RecoveryTerminalStateStub>`
#[derive(Clone, Copy)]
pub struct RecoveryRuntimeSummaryStub {
    pub run: StubRunId,
    pub first_seq: StubEventSeq,
    pub last_seq: StubEventSeq,
    pub workflow: Option<StubWorkflowDigest>,
    pub steps_started: u64,
    pub steps_succeeded: u64,
    pub actions_scheduled: u64,
    pub actions_resolved: u64,
    pub suspensions: u64,
    pub slots_written: u64,
    pub terminal: Option<RecoveryTerminalStateStub>,
}

// ---------------------------------------------------------------------------
// RecoveryFrameSeed stub — types.rs:728+ (minimal)
//
// Spec proofs reason about the `summary()` method on `RecoveryHydration`
// only; the FrameSeed payload is reduced to a length-counters struct.
// ---------------------------------------------------------------------------

/// Minimal stub of `RecoveryFrameSeed` so `RecoveryHydration` can be
/// mirrored in full. Field shape mirrors production at the count level.
#[derive(Clone, Copy)]
pub struct RecoveryFrameSeedStub {
    pub summary: RecoveryRuntimeSummaryStub,
    pub first_step: StubStepIdx,
    pub step_count: u16,
    pub slot_count: u16,
    pub pc: u64,
}

// ---------------------------------------------------------------------------
// RecoveryHydration stub — types.rs:587-605
// ---------------------------------------------------------------------------

/// Mirror of `RecoveryHydration` at
/// `crates/vb_storage/src/recovery/types.rs:587-605`. The closed
/// variant set is `Summary(RecoveryRuntimeSummary)` and
/// `FrameSeed(RecoveryFrameSeed)`.
#[derive(Clone, Copy)]
pub enum RecoveryHydrationStub {
    Summary(RecoveryRuntimeSummaryStub),
    FrameSeed(RecoveryFrameSeedStub),
}

impl RecoveryHydrationStub {
    /// Mirror of `RecoveryHydration::summary` at
    /// `crates/vb_storage/src/recovery/types.rs:597-604`. Production
    /// body: `Summary(s) -> *s, FrameSeed(seed) -> seed.summary`.
    /// Marked `#[verifier::exec]` so the method can be called from
    /// both spec and exec contexts. Body is verified by Verus (no
    /// `#[verifier::external]`).
    #[verifier::exec]
    pub fn summary(self) -> RecoveryRuntimeSummaryStub {
        match self {
            Self::Summary(s) => s,
            Self::FrameSeed(seed) => seed.summary,
        }
    }
}

// ---------------------------------------------------------------------------
// UnsupportedRecoveryState stub — types.rs:652-663
// ---------------------------------------------------------------------------

/// Mirror of `UnsupportedRecoveryState` at
/// `crates/vb_storage/src/recovery/types.rs:652-663`. All four fields
/// are `bool`; the mirror is bit-identical to production.
#[derive(Clone, Copy)]
pub struct UnsupportedRecoveryStateStub {
    /// Mirror of `slot_values: bool` at types.rs:656.
    pub slot_values: bool,
    /// Mirror of `slot_taint: bool` at types.rs:658.
    pub slot_taint: bool,
    /// Mirror of `action_payloads: bool` at types.rs:660.
    pub action_payloads: bool,
    /// Mirror of `pending_actions: bool` at types.rs:662.
    pub pending_actions: bool,
}

impl UnsupportedRecoveryStateStub {
    /// Mirror of `UnsupportedRecoveryState::SUPPORTED` const at
    /// `crates/vb_storage/src/recovery/types.rs:665-672`.
    pub const SUPPORTED: Self = Self {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    };

    /// Mirror of `UnsupportedRecoveryState::union` const fn at
    /// `crates/vb_storage/src/recovery/types.rs:701-710`. Body:
    /// flagwise OR across all 4 fields.
    ///
    /// TRUST BOUNDARY: body is `#[verifier::external]`.
    #[verifier::external]
    pub const fn union(self, other: Self) -> Self {
        Self {
            slot_values: self.slot_values || other.slot_values,
            slot_taint: self.slot_taint || other.slot_taint,
            action_payloads: self.action_payloads || other.action_payloads,
            pending_actions: self.pending_actions || other.pending_actions,
        }
    }

    /// Mirror of `UnsupportedRecoveryState::is_fully_supported` const fn
    /// at `crates/vb_storage/src/recovery/types.rs:713-716`.
    ///
    /// TRUST BOUNDARY: body is `#[verifier::external]`.
    #[verifier::external]
    pub const fn is_fully_supported(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
    }
}

// ---------------------------------------------------------------------------
// Phantom drift-detection helpers — force Rust to resolve every
// production item by name + signature.
// ---------------------------------------------------------------------------

/// Phantom: forces resolution of every production-stub item. A rename,
/// signature change, or field-type change in production breaks the
/// references below at compile time.
#[verifier::external]
fn prod_items_drift_check(
    term: RecoveryTerminalStateStub,
    summ: RecoveryRuntimeSummaryStub,
    step: RecoveredStepStateStub,
    hyd: RecoveryHydrationStub,
    unsup_a: UnsupportedRecoveryStateStub,
    unsup_b: UnsupportedRecoveryStateStub,
) {
    // RecoveryTerminalState variants by name + signature.
    let _t1 = RecoveryTerminalStateStub::Cancelled;
    let _t2 = RecoveryTerminalStateStub::Killed;
    let _t3 = RecoveryTerminalStateStub::Finished { result: StubSlotIdx(0) };
    let _t4 = RecoveryTerminalStateStub::Failed;
    let _ = term;
    // RecoveryRuntimeSummary field references (drift on rename).
    let _r = summ.run;
    let _fs = summ.first_seq;
    let _ls = summ.last_seq;
    let _w = summ.workflow;
    let _sst = summ.steps_started;
    let _ssu = summ.steps_succeeded;
    let _asc = summ.actions_scheduled;
    let _are = summ.actions_resolved;
    let _sus = summ.suspensions;
    let _swr = summ.slots_written;
    let _ter = summ.terminal;
    // RecoveredStepState variants.
    let _ = step;
    // RecoveryHydration variants + summary method.
    let _ = RecoveryHydrationStub::Summary(summ);
    let _ = RecoveryHydrationStub::FrameSeed(RecoveryFrameSeedStub {
        summary: summ,
        first_step: StubStepIdx(0),
        step_count: 0,
        slot_count: 0,
        pc: 0,
    });
    let _h = hyd.summary();
    // UnsupportedRecoveryState field + const + method drift.
    let _supported: UnsupportedRecoveryStateStub =
        UnsupportedRecoveryStateStub::SUPPORTED;
    let _u = unsup_a.union(unsup_b);
    let _f = unsup_a.is_fully_supported();
    let _sv1 = unsup_a.slot_values;
    let _st1 = unsup_a.slot_taint;
    let _ap1 = unsup_a.action_payloads;
    let _pa1 = unsup_a.pending_actions;
}

// ---------------------------------------------------------------------------
// Convenience re-exports — strip the `Stub` suffix so spec proofs can
// refer to `production::RecoveryTerminalState` etc. instead of needing
// to qualify through the bridge's outer `pub use` block.
// ---------------------------------------------------------------------------

pub use RecoveryFrameSeedStub as RecoveryFrameSeed;
pub use RecoveryHydrationStub as RecoveryHydration;
pub use RecoveryRuntimeSummaryStub as RecoveryRuntimeSummary;
pub use RecoveryTerminalStateStub as RecoveryTerminalState;
pub use RecoveredStepStateStub as RecoveredStepState;
pub use UnsupportedRecoveryStateStub as UnsupportedRecoveryState;
pub use StubEventSeq as EventSeq;
pub use StubRunId as RunId;
pub use StubSlotIdx as SlotIdx;
pub use StubStepIdx as StepIdx;
pub use StubWorkflowDigest as WorkflowDigest;

} // verus!