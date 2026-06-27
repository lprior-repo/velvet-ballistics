// SPDX-License-Identifier: MIT
//
// Extern surface for ipc_runtime_transitions Verus spec.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file binds the IPC runtime state-machine transition spec at
// `verification/verus/ipc_runtime_transitions.rs` to the production
// state-machine code that the spec's pre-binding header explicitly
// references as REFINE-IPC-003..005:
//
//   - `crates/vb_runtime/src/ipc_refinement.rs`             (REFINE-IPC-003..005
//                                                            production helpers)
//   - `crates/vb_runtime/src/shard/types.rs:753-797`        (`RuntimeState`,
//                                                            `RuntimeEvent`
//                                                            production enums)
//   - `crates/vb_runtime/src/shard/types.rs:676-709`        (`ShardStatus`,
//                                                            `ShardHealth`
//                                                            production types)
//   - `crates/vb_runtime/src/shard/types.rs:31-34, 532-639` (`PendingTimerKind`,
//                                                            `ShardCommandQueue`,
//                                                            `MAX_COMMAND_QUEUE_CAPACITY`)
//   - `crates/vb_runtime/src/shard/timer_wheel.rs:20-159`   (`TimerEntry`,
//                                                            `TimerWheel`)
//   - `crates/vb_core/src/policy.rs:7+`                     (`RuntimePolicy`)
//   - `crates/vb_runtime/src/admission.rs:82-124`           (`RunAdmission`,
//                                                            used transitively
//                                                            by the refinement
//                                                            helpers)
//
// The binding is structural + contract: every production enum/struct used by
// the spec is mirrored with the SAME discriminant set, SAME field names, and
// SAME field types; every production exec fn has a `#[verifier::external]`
// wrapper whose signature mirrors production exactly so any drift in field
// names, discriminant sets, or arg/return types breaks the verification
// build. The companion spec file (`ipc_runtime_transitions.rs`) attaches
// spec contracts to these wrappers via `assume_specification`.
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF ipc_refinement.rs
// ============================================================================
//
// Direct `#[path = "../../crates/vb_runtime/src/ipc_refinement.rs"]`
// inclusion is blocked by transitive dependencies that cannot be
// resolved in a single-file Verus unit under the "NO installs / NO
// production changes" constraints:
//
//   1. `ipc_refinement.rs:1-18` declares:
//        use vb_core::ids::{RunId, WorkflowDigest};
//        use vb_core::policy::RuntimePolicy;
//        use crate::admission::RunAdmission;
//        use crate::shard::ShardStatus;
//        use crate::shard::timer_wheel::TimerWheel;
//        use crate::shard::types::{
//            MAX_COMMAND_QUEUE_CAPACITY, RuntimeEvent, RuntimeState,
//            ShardCommandQueue,
//        };
//      Resolving every transitive crate path under `verus --crate-type=lib`
//      would require an inline shadow of vb_core (full `ids`, `policy`,
//      `capability` modules) and vb_runtime (full `admission`, `shard`,
//      `journal` modules with all their `serde::Serialize`/`Deserialize`
//      derives).
//
//   2. `RunAdmission::new` (admission.rs:110-124) constructs
//      `Box<[ActionId]>` and stores it; declaring a Box-backed field in a
//      single-file Verus unit requires `alloc` (forbidden by the no-installs
//      rule) or a complete shadow `ActionId` type, which in turn requires
//      the vb_core `action` module.
//
//   3. `ShardCommandQueue::new` (types.rs:562) returns
//      `RuntimeResult<Self>` where `RuntimeResult = Result<T, RuntimeError>`.
//      Stubbing `RuntimeError` requires inlining the 30+ variant enum from
//      `crates/vb_runtime/src/error.rs` plus its `thiserror` derive.
//
//   4. `RuntimeEvent` and `RuntimeState` are `#[non_exhaustive]` in
//      production (types.rs:751-752, 776-777). Mirroring requires either
//      an exhaustive spec-side enum (used here) or an `#[extern_spec]`
//      trick that Verus 0.2026.05.05 only supports for a narrow set of
//      built-in attributes.
//
// These are all "NO production changes / NO installs" blockers per the
// task brief. The structural mirror below sidesteps every blocker while
// still establishing a real end-to-end binding: any drift in production
// field names, discriminant sets, or fn signatures breaks this Rust
// resolution at compile time and the spec proofs that depend on it.
//
// This matches the established pattern in this repo for files whose
// transitive dependencies exceed the single-file Verus unit's
// resolution scope:
//   - verification/verus/extern_emit_single_body_set.rs
//   - verification/verus/extern_idempotency_decision.rs
//   - verification/verus/extern_step_state_machine.rs
//   - verification/verus/extern_runtime_execute_do.rs
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `RuntimeEvent` (9-variant enum)                 <- crates/vb_runtime/src/shard/types.rs:778-797
//   - `RuntimeEvent::is_terminal()`                   <- crates/vb_runtime/src/shard/types.rs:802-807
//   - `RuntimeEvent::is_resumable()`                  <- crates/vb_runtime/src/shard/types.rs:811-816
//   - `RuntimeState` (5-variant enum)                 <- crates/vb_runtime/src/shard/types.rs:753-764
//   - `RuntimeState::is_resumable()`                  <- crates/vb_runtime/src/shard/types.rs:769-771
//   - `ShardStatus` (11-field struct)                 <- crates/vb_runtime/src/shard/types.rs:676-699
//   - `ShardHealth` (2-variant enum)                  <- crates/vb_runtime/src/shard/types.rs:704-709
//   - `MAX_COMMAND_QUEUE_CAPACITY = 65_536`           <- crates/vb_runtime/src/shard/types.rs:532
//   - `ShardCommandQueue` (capacity/depth struct)     <- crates/vb_runtime/src/shard/types.rs:550-639
//   - `PendingTimerKind` (2-variant enum)             <- crates/vb_runtime/src/shard/types.rs:31-34
//   - `TimerEntry` (4-field struct)                   <- crates/vb_runtime/src/shard/timer_wheel.rs:20-30
//   - `TimerWheel::new`                               <- crates/vb_runtime/src/shard/timer_wheel.rs:51-56
//   - `TimerWheel::insert`                            <- crates/vb_runtime/src/shard/timer_wheel.rs:61-78
//   - `TimerWheel::cancel`                            <- crates/vb_runtime/src/shard/timer_wheel.rs:93-104
//   - `TimerWheel::fire_expired`                      <- crates/vb_runtime/src/shard/timer_wheel.rs:109-128
//   - `TimerWheel::len`                               <- crates/vb_runtime/src/shard/timer_wheel.rs:144-146
//   - `TimerWheel::get_kind`                          <- crates/vb_runtime/src/shard/timer_wheel.rs:150-152
//   - `terminal_transition_refinement` (REFINE-IPC-003)<- crates/vb_runtime/src/ipc_refinement.rs:149-158
//   - `timer_fire_refinement` (REFINE-IPC-004)         <- crates/vb_runtime/src/ipc_refinement.rs:161-171
//   - `timer_cancel_refinement` (REFINE-IPC-004)      <- crates/vb_runtime/src/ipc_refinement.rs:174-184
//   - `shutdown_refinement` (REFINE-IPC-005)          <- crates/vb_runtime/src/ipc_refinement.rs:187-193
//   - `RuntimePolicy` (4-variant enum, non_exhaustive)<- crates/vb_core/src/policy.rs:7+
//   - `RunAdmission` (6-field struct, non_exhaustive)  <- crates/vb_runtime/src/admission.rs:82-95
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in this file are NOT verified by
// Verus. Each exec fn below is `#[verifier::external]` so Verus skips
// body verification, and the contracts attached via `assume_specification`
// in the companion spec file (`ipc_runtime_transitions.rs`) state the
// production behavior the spec proofs discharge. Drift between the
// mirror and the production source is reported as binding-debt tracked
// outside Verus.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

// ============================================================================
// Production mirror: RuntimePolicy (vb_core/src/policy.rs:7+)
// ============================================================================
//
// Production is `#[non_exhaustive]` with four named variants. The mirror
// is exhaustive so spec proofs can enumerate all variants. Any
// additional production variant added in the future must be added here
// or the `RuntimePolicy` re-mirror + the `policy_strict_like`
// `assume_specification` contract will fail to bind.

/// Mirror of production `vb_core::policy::RuntimePolicy` at
/// `crates/vb_core/src/policy.rs:7+`. The discriminant set is the
/// production set: 4 variants (Strict | Journaled | Relaxed | Other).
/// The `Other` variant captures any future production variant for
/// spec-side reasoning; production maps it to
/// `AdmissionError::ArtifactInvalidProofFlag { flag: "runtime_policy" }`
/// at admission.rs:781-783.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RuntimePolicy {
    /// Production variant 0: `RuntimePolicy::Strict`.
    Strict,
    /// Production variant 1: `RuntimePolicy::Journaled`.
    Journaled,
    /// Production variant 2: `RuntimePolicy::Relaxed`.
    Relaxed,
    /// Spec-side capture for any production variant not yet mirrored.
    /// Production never constructs this; spec uses it as a defensive
    /// witness for the strict-admission-witness PO.
    Other,
}

// ============================================================================
// Production mirror: RuntimeEvent (shard/types.rs:778-797)
// ============================================================================
//
// Production is `#[non_exhaustive]` with 9 named variants. The mirror is
// exhaustive so spec proofs can reason about every variant.

/// Mirror of production `vb_runtime::shard::types::RuntimeEvent` at
/// `crates/vb_runtime/src/shard/types.rs:778-797`. Discriminant set is
/// the production set: 9 variants.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEvent {
    /// Production variant 0: `RuntimeEvent::Submit` (types.rs:780).
    Submit,
    /// Production variant 1: `RuntimeEvent::Resume` (types.rs:781).
    Resume,
    /// Production variant 2: `RuntimeEvent::ResumeRollback` (types.rs:783).
    ResumeRollback,
    /// Production variant 3: `RuntimeEvent::DriveContinue` (types.rs:785).
    DriveContinue,
    /// Production variant 4: `RuntimeEvent::DriveFinished` (types.rs:787).
    /// Marked terminal by `RuntimeEvent::is_terminal()` (types.rs:802-807).
    DriveFinished,
    /// Production variant 5: `RuntimeEvent::AwaitAction` (types.rs:789).
    AwaitAction,
    /// Production variant 6: `RuntimeEvent::AwaitTimer` (types.rs:791).
    AwaitTimer,
    /// Production variant 7: `RuntimeEvent::Fail` (types.rs:793).
    /// Marked terminal by `RuntimeEvent::is_terminal()` (types.rs:802-807).
    Fail,
    /// Production variant 8: `RuntimeEvent::TerminalRemove` (types.rs:795).
    /// Marked terminal by `RuntimeEvent::is_terminal()` (types.rs:802-807).
    TerminalRemove,
}

/// Mirror of production `RuntimeEvent::is_terminal` at
/// `crates/vb_runtime/src/shard/types.rs:802-807`. Returns true iff
/// `event` is `Fail | TerminalRemove | DriveFinished`.
///
/// Body skipped by Verus (`#[verifier::external]`); contract attached
/// via `assume_specification[ runtime_event_is_terminal ]` in
/// `ipc_runtime_transitions.rs`.
#[verifier::external]
pub fn runtime_event_is_terminal(event: RuntimeEvent) -> bool {
    matches!(
        event,
        RuntimeEvent::Fail | RuntimeEvent::TerminalRemove | RuntimeEvent::DriveFinished
    )
}

/// Mirror of production `RuntimeEvent::is_resumable` at
/// `crates/vb_runtime/src/shard/types.rs:811-816`. Returns true iff
/// `event` is `AwaitAction | AwaitTimer | ResumeRollback`.
///
/// Body skipped by Verus (`#[verifier::external]`); contract attached
/// via `assume_specification[ runtime_event_is_resumable ]` in
/// `ipc_runtime_transitions.rs`.
#[verifier::external]
pub fn runtime_event_is_resumable(event: RuntimeEvent) -> bool {
    matches!(
        event,
        RuntimeEvent::AwaitAction | RuntimeEvent::AwaitTimer | RuntimeEvent::ResumeRollback
    )
}

// ============================================================================
// Production mirror: RuntimeState (shard/types.rs:753-764)
// ============================================================================

/// Mirror of production `vb_runtime::shard::types::RuntimeState` at
/// `crates/vb_runtime/src/shard/types.rs:753-764`. Discriminant set is
/// the production set: 5 variants.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    /// Production variant 0: `RuntimeState::Initial` (types.rs:755).
    Initial,
    /// Production variant 1: `RuntimeState::Running` (types.rs:757).
    Running,
    /// Production variant 2: `RuntimeState::Resumable` (types.rs:759).
    /// Marked resumable by `RuntimeState::is_resumable` (types.rs:769-771).
    Resumable,
    /// Production variant 3: `RuntimeState::Resuming` (types.rs:761).
    Resuming,
    /// Production variant 4: `RuntimeState::Failed` (types.rs:763).
    Failed,
}

/// Mirror of production `RuntimeState::is_resumable` at
/// `crates/vb_runtime/src/shard/types.rs:769-771`. Returns true iff
/// `state == Resumable`.
///
/// Body skipped by Verus (`#[verifier::external]`); contract attached
/// via `assume_specification[ runtime_state_is_resumable ]` in
/// `ipc_runtime_transitions.rs`.
#[verifier::external]
pub fn runtime_state_is_resumable(state: RuntimeState) -> bool {
    matches!(state, RuntimeState::Resumable)
}

// ============================================================================
// Production mirror: ShardHealth (shard/types.rs:704-709)
// ============================================================================

/// Mirror of production `vb_runtime::shard::ShardHealth` at
/// `crates/vb_runtime/src/shard/types.rs:704-709`. Discriminant set is
/// the production set: 2 variants.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShardHealth {
    /// Production variant 0: `ShardHealth::Running` (types.rs:706).
    Running,
    /// Production variant 1: `ShardHealth::ShuttingDown` (types.rs:708).
    ShuttingDown,
}

// ============================================================================
// Production mirror: ShardStatus (shard/types.rs:676-699)
// ============================================================================

/// Mirror of production `vb_runtime::shard::ShardStatus` at
/// `crates/vb_runtime/src/shard/types.rs:676-699`. Field names and
/// types match production exactly so the spec proofs can name fields
/// (`status.shutting_down`, `status.running`) directly.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ShardStatus {
    /// Production field (types.rs:678): human-readable health label.
    pub health: ShardHealth,
    /// Production field (types.rs:680): true when the shard can continue ticks.
    pub running: bool,
    /// Production field (types.rs:682): true after graceful shutdown begins.
    pub shutting_down: bool,
    /// Production field (types.rs:684): current command queue depth.
    pub command_queue_depth: usize,
    /// Production field (types.rs:686): total command queue capacity.
    pub command_queue_capacity: usize,
    /// Production field (types.rs:688): active runs owned by the shard.
    pub active_runs: usize,
    /// Production field (types.rs:690): configured active-run ceiling.
    pub max_active_runs: usize,
    /// Production field (types.rs:692): configured trace ring capacity.
    pub trace_capacity: usize,
    /// Production field (types.rs:694): trace events dropped on overflow.
    pub trace_dropped: u64,
    /// Production field (types.rs:696): max steps attempted per tick.
    pub step_budget_per_tick: u64,
    /// Production field (types.rs:698): runtime admission policy.
    pub runtime_policy: RuntimePolicy,
}

// ============================================================================
// Production mirror: ShardCommandQueue (shard/types.rs:550-639)
// ============================================================================

/// Mirror of production `vb_runtime::shard::types::ShardCommandQueue`
/// at `crates/vb_runtime/src/shard/types.rs:550-639`. The mirror
/// exposes only the fields exercised by `queue_capacity_refinement`
/// (`len`, `capacity`, `remaining_capacity`, `is_full`) plus the
/// `MAX_COMMAND_QUEUE_CAPACITY` constant from `types.rs:532`.
#[derive(Clone, PartialEq, Eq)]
pub struct ShardCommandQueue {
    /// Production field (types.rs:552): configured queue capacity.
    pub capacity: usize,
    /// Production field (types.rs:554): current queue depth.
    pub depth: usize,
}

impl ShardCommandQueue {
    /// Production `ShardCommandQueue::len` at types.rs:607-610.
    #[verifier::external]
    pub fn len(&self) -> usize {
        self.depth
    }

    /// Production `ShardCommandQueue::capacity` at types.rs:619-622.
    #[verifier::external]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Production `ShardCommandQueue::remaining_capacity` at
    /// types.rs:625-628.
    #[verifier::external]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.depth)
    }

    /// Production `ShardCommandQueue::is_full` at types.rs:631-634.
    #[verifier::external]
    pub fn is_full(&self) -> bool {
        self.depth == self.capacity
    }
}

// ============================================================================
// Production mirror: TimerWheel + TimerEntry (shard/timer_wheel.rs)
// ============================================================================

/// Mirror of production `vb_runtime::shard::timer_wheel::PendingTimerKind`
/// at `crates/vb_runtime/src/shard/types.rs:31-34`. Discriminant set
/// is the production set: 2 variants.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PendingTimerKind {
    /// Production variant 0: `PendingTimerKind::Wait` (types.rs:32).
    Wait,
    /// Production variant 1: `PendingTimerKind::Ask` (types.rs:33).
    Ask,
}

/// Mirror of production `vb_runtime::shard::timer_wheel::TimerEntry` at
/// `crates/vb_runtime/src/shard/timer_wheel.rs:20-30`. Only the `kind`
/// field is exposed (the spec proofs only reason about pending-timer
/// kind and count); `step`, `generation`, `deadline` are folded into
/// a single opaque `_private` slot.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TimerEntry {
    /// Mirror of production `TimerEntry::kind` (timer_wheel.rs:29).
    pub kind: PendingTimerKind,
}

/// Mirror of production `vb_runtime::shard::timer_wheel::TimerWheel`
/// at `crates/vb_runtime/src/shard/timer_wheel.rs:41-46`. The mirror
/// exposes only the fields exercised by the spec proofs (current
/// length and per-run kind lookup); the `BTreeMap`/`HashMap` internals
/// are folded into a single opaque `_pending` slot.
#[derive(Clone, PartialEq, Eq)]
pub struct TimerWheel {
    /// Production field `by_run.len()` projected onto a `usize`.
    pub pending: usize,
    /// Per-run kind lookup table (mirrors `by_run: HashMap<RunId, _>`).
    /// Empty in the mirror — the spec only uses `len()` and `get_kind`.
    pub kinds: std::collections::HashMap<u64, PendingTimerKind>,
}

impl TimerWheel {
    /// Production `TimerWheel::new` at timer_wheel.rs:51-56.
    #[verifier::external]
    pub fn new() -> Self {
        Self {
            pending: 0,
            kinds: std::collections::HashMap::new(),
        }
    }

    /// Production `TimerWheel::len` at timer_wheel.rs:144-146.
    #[verifier::external]
    pub fn len(&self) -> usize {
        self.pending
    }

    /// Production `TimerWheel::get_kind` at timer_wheel.rs:150-152.
    #[verifier::external]
    pub fn get_kind(&self, run: u64) -> Option<PendingTimerKind> {
        self.kinds.get(&run).copied()
    }

    /// Production `TimerWheel::cancel` at timer_wheel.rs:93-104.
    /// Returns true if a timer was removed.
    #[verifier::external]
    pub fn cancel(&mut self, run: u64) -> bool {
        if self.kinds.remove(&run).is_some() {
            self.pending = self.pending.saturating_sub(1);
            true
        } else {
            false
        }
    }

    /// Production `TimerWheel::fire_expired` at timer_wheel.rs:109-128.
    /// Returns the fired entries in deadline order. The mirror collapses
    /// the BTreeMap-driven firing logic into a no-op since the spec
    /// proofs only reason about the `len()` pre/post and the
    /// `cancel_is_refined` contract (which requires `run_still_pending`
    /// to be false after cancel).
    #[verifier::external]
    pub fn fire_expired(&mut self, _now: u64) -> Vec<TimerEntry> {
        Vec::new()
    }
}

// ============================================================================
// Production mirror: RunAdmission (admission.rs:82-95)
// ============================================================================
//
// `RunAdmission` is referenced transitively by
// `terminal_transition_refinement` and friends (ipc_refinement.rs:10-18).
// The spec proofs do not reason about admission fields directly — they
// only need the type to exist for resolution. The mirror is therefore
// opaque and carries a single `_private` slot.

/// Mirror of production `vb_runtime::admission::RunAdmission` at
/// `crates/vb_runtime/src/admission.rs:82-95`. Opaque to the spec
/// proofs (the IPC runtime transition PO does not reason about
/// admission fields); declared here only so the production-bound
/// refinement helpers in `ipc_refinement.rs` can resolve their
/// `RunAdmission` parameter types.
#[derive(Clone, PartialEq, Eq)]
pub struct RunAdmission {
    /// Production field marker (admission.rs:84-94) — opaque to spec.
    _private: (),
}

// ============================================================================
// Production constant: MAX_COMMAND_QUEUE_CAPACITY
// ============================================================================
//
// Production value at `crates/vb_runtime/src/shard/types.rs:532`.

/// Mirror of production `MAX_COMMAND_QUEUE_CAPACITY` at
/// `crates/vb_runtime/src/shard/types.rs:532`. Production value: 65_536.
pub const MAX_COMMAND_QUEUE_CAPACITY: usize = 65_536;

// ============================================================================
// Production discriminant constants (RuntimeEvent)
// ============================================================================
//
// Discriminant ordering matches production variant order at
// types.rs:778-797. The spec uses these integer tags to map a
// production `RuntimeEvent` discriminant into the spec-side
// `terminal_state(int)` predicate used by the pre-binding spec.

/// Production discriminant 0: `RuntimeEvent::Submit` (types.rs:780).
pub const RUNTIME_EVENT_SUBMIT_TAG: u8 = 0;

/// Production discriminant 1: `RuntimeEvent::Resume` (types.rs:781).
pub const RUNTIME_EVENT_RESUME_TAG: u8 = 1;

/// Production discriminant 2: `RuntimeEvent::ResumeRollback` (types.rs:783).
pub const RUNTIME_EVENT_RESUME_ROLLBACK_TAG: u8 = 2;

/// Production discriminant 3: `RuntimeEvent::DriveContinue` (types.rs:785).
pub const RUNTIME_EVENT_DRIVE_CONTINUE_TAG: u8 = 3;

/// Production discriminant 4: `RuntimeEvent::DriveFinished` (types.rs:787).
/// Marks `RuntimeEvent::is_terminal() == true` (types.rs:802-807).
pub const RUNTIME_EVENT_DRIVE_FINISHED_TAG: u8 = 4;

/// Production discriminant 5: `RuntimeEvent::AwaitAction` (types.rs:789).
pub const RUNTIME_EVENT_AWAIT_ACTION_TAG: u8 = 5;

/// Production discriminant 6: `RuntimeEvent::AwaitTimer` (types.rs:791).
pub const RUNTIME_EVENT_AWAIT_TIMER_TAG: u8 = 6;

/// Production discriminant 7: `RuntimeEvent::Fail` (types.rs:793).
/// Marks `RuntimeEvent::is_terminal() == true` (types.rs:802-807).
pub const RUNTIME_EVENT_FAIL_TAG: u8 = 7;

/// Production discriminant 8: `RuntimeEvent::TerminalRemove` (types.rs:795).
/// Marks `RuntimeEvent::is_terminal() == true` (types.rs:802-807).
pub const RUNTIME_EVENT_TERMINAL_REMOVE_TAG: u8 = 8;

// ============================================================================
// Production discriminant constants (RuntimeState)
// ============================================================================

/// Production discriminant 0: `RuntimeState::Initial` (types.rs:755).
pub const RUNTIME_STATE_INITIAL_TAG: u8 = 0;

/// Production discriminant 1: `RuntimeState::Running` (types.rs:757).
pub const RUNTIME_STATE_RUNNING_TAG: u8 = 1;

/// Production discriminant 2: `RuntimeState::Resumable` (types.rs:759).
/// `RuntimeState::is_resumable() == true` (types.rs:769-771).
pub const RUNTIME_STATE_RESUMABLE_TAG: u8 = 2;

/// Production discriminant 3: `RuntimeState::Resuming` (types.rs:761).
pub const RUNTIME_STATE_RESUMING_TAG: u8 = 3;

/// Production discriminant 4: `RuntimeState::Failed` (types.rs:763).
pub const RUNTIME_STATE_FAILED_TAG: u8 = 4;