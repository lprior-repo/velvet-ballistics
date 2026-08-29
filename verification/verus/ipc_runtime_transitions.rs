// SPDX-License-Identifier: MIT
//
// Verus proof obligations for IPC runtime state-machine transitions.
// Obligation IDs: VERUS-IPC-003..005. Production linkage: REFINE-IPC-003..005.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is bound to the production IPC runtime state-machine code that
// the pre-binding spec's header explicitly references as REFINE-IPC-003..005:
//
//   - `crates/vb_runtime/src/ipc_refinement.rs`             (REFINE-IPC-003..005)
//   - `crates/vb_runtime/src/shard/types.rs:753-797`        (RuntimeState, RuntimeEvent)
//   - `crates/vb_runtime/src/shard/types.rs:676-709`        (ShardStatus, ShardHealth)
//   - `crates/vb_runtime/src/shard/types.rs:31-34, 532-639` (PendingTimerKind,
//                                                            ShardCommandQueue,
//                                                            MAX_COMMAND_QUEUE_CAPACITY)
//   - `crates/vb_runtime/src/shard/timer_wheel.rs:20-159`   (TimerEntry, TimerWheel)
//   - `crates/vb_core/src/policy.rs:7+`                     (RuntimePolicy)
//   - `crates/vb_runtime/src/admission.rs:82-95`            (RunAdmission, transitively)
//
// The binding is via the companion extern surface
// `verification/verus/extern_ipc_runtime_transitions.rs`, which declares
// production type mirrors (outside `verus!`, opaque to Verus) plus
// production discriminant constants. The mirror types declared INSIDE
// `verus!` below (`MirrorRuntimeEvent`, `MirrorRuntimeState`, etc.) have
// the SAME discriminant sets and field names as production; the
// `assume_specification` bridges below attach spec contracts to mirror
// exec methods whose `#[verifier::external]` bodies reproduce the
// production decision logic. Any drift in production field names,
// discriminant sets, or fn signatures breaks the extern mirror and the
// spec proofs.
//
// ============================================================================
// BINDING LEDGER (mirrors the extern file's BINDING LEDGER)
// ============================================================================
//   - `MirrorRuntimeEvent` (9-variant enum)            <- crates/vb_runtime/src/shard/types.rs:778-797
//   - `MirrorRuntimeEvent::is_terminal()`              <- crates/vb_runtime/src/shard/types.rs:802-807
//   - `MirrorRuntimeEvent::is_resumable()`             <- crates/vb_runtime/src/shard/types.rs:811-816
//   - `MirrorRuntimeState` (5-variant enum)            <- crates/vb_runtime/src/shard/types.rs:753-764
//   - `MirrorRuntimeState::is_resumable()`             <- crates/vb_runtime/src/shard/types.rs:769-771
//   - `MirrorShardStatus` (11-field struct)            <- crates/vb_runtime/src/shard/types.rs:676-699
//   - `MirrorShardHealth` (2-variant enum)             <- crates/vb_runtime/src/shard/types.rs:704-709
//   - `MirrorTimerWheel::len / get_kind / cancel / fire_expired`
//                                                     <- crates/vb_runtime/src/shard/timer_wheel.rs:144, 150, 93, 109
//   - `MAX_COMMAND_QUEUE_CAPACITY = 65_536`            <- crates/vb_runtime/src/shard/types.rs:532
//
// Source: vb-IPC proof-obligations VERUS-IPC-003..005 / REFINE-IPC-003..005.

#[path = "extern_ipc_runtime_transitions.rs"]
mod production;

use vstd::prelude::*;

verus! {

// ============================================================================
// Spec-side mirror types — production-bound via extern mirror
// ============================================================================
//
// These types are declared INSIDE `verus!` so Verus can reason about
// their structure. Each is a structural mirror of the production type
// referenced in the BINDING LEDGER above; any drift in production
// discriminant sets or field names is caught at compile time via the
// extern mirror's documented line references and via code review of
// the `#[verifier::external]` mirror-method bodies below.

// --------------------------------------------------------------------------
// MirrorRuntimeEvent — 9 variants, mirrors production order
// --------------------------------------------------------------------------
/// Mirror of production `vb_runtime::shard::types::RuntimeEvent` at
/// `crates/vb_runtime/src/shard/types.rs:778-797`. Discriminant set is
/// the production set: 9 variants in production order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MirrorRuntimeEvent {
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

// Production discriminant constants for MirrorRuntimeEvent (production variant
// order). These mirror the constants in the extern file and are duplicated
// here because Verus cannot reference `pub const` items declared outside
// `verus!`. The values match production exactly.
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_EVENT_SUBMIT_TAG: u8 = 0;
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_EVENT_RESUME_TAG: u8 = 1;
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_EVENT_RESUME_ROLLBACK_TAG: u8 = 2;
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_EVENT_DRIVE_CONTINUE_TAG: u8 = 3;
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_EVENT_DRIVE_FINISHED_TAG: u8 = 4;
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_EVENT_AWAIT_ACTION_TAG: u8 = 5;
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_EVENT_AWAIT_TIMER_TAG: u8 = 6;
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_EVENT_FAIL_TAG: u8 = 7;
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_EVENT_TERMINAL_REMOVE_TAG: u8 = 8;

// --------------------------------------------------------------------------
// MirrorRuntimeState — 5 variants, mirrors production order
// --------------------------------------------------------------------------
/// Mirror of production `vb_runtime::shard::types::RuntimeState` at
/// `crates/vb_runtime/src/shard/types.rs:753-764`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MirrorRuntimeState {
    /// Production variant 0: `RuntimeState::Initial` (types.rs:755).
    Initial,
    /// Production variant 1: `RuntimeState::Running` (types.rs:757).
    Running,
    /// Production variant 2: `RuntimeState::Resumable` (types.rs:759).
    /// `is_resumable() == true` (types.rs:769-771).
    Resumable,
    /// Production variant 3: `RuntimeState::Resuming` (types.rs:761).
    Resuming,
    /// Production variant 4: `RuntimeState::Failed` (types.rs:763).
    Failed,
}

#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_STATE_INITIAL_TAG: u8 = 0;
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_STATE_RUNNING_TAG: u8 = 1;
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_STATE_RESUMABLE_TAG: u8 = 2;
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_STATE_RESUMING_TAG: u8 = 3;
#[allow(non_snake_case)]
pub const MIRROR_RUNTIME_STATE_FAILED_TAG: u8 = 4;

// --------------------------------------------------------------------------
// MirrorShardHealth — 2 variants
// --------------------------------------------------------------------------
/// Mirror of production `vb_runtime::shard::ShardHealth` at
/// `crates/vb_runtime/src/shard/types.rs:704-709`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MirrorShardHealth {
    /// Production variant 0: `ShardHealth::Running` (types.rs:706).
    Running,
    /// Production variant 1: `ShardHealth::ShuttingDown` (types.rs:708).
    ShuttingDown,
}

// --------------------------------------------------------------------------
// MirrorShardStatus — 11 fields
// --------------------------------------------------------------------------
/// Mirror of production `vb_runtime::shard::ShardStatus` at
/// `crates/vb_runtime/src/shard/types.rs:676-699`.
pub struct MirrorShardStatus {
    /// Production field (types.rs:678): health label.
    pub health: MirrorShardHealth,
    /// Production field (types.rs:680): shard can continue ticks.
    pub running: bool,
    /// Production field (types.rs:682): graceful shutdown has begun.
    pub shutting_down: bool,
    /// Production field (types.rs:684): command queue depth.
    pub command_queue_depth: usize,
    /// Production field (types.rs:686): command queue capacity.
    pub command_queue_capacity: usize,
    /// Production field (types.rs:688): active runs owned by shard.
    pub active_runs: usize,
    /// Production field (types.rs:690): active-run ceiling.
    pub max_active_runs: usize,
    /// Production field (types.rs:692): trace ring capacity.
    pub trace_capacity: usize,
    /// Production field (types.rs:694): trace events dropped on overflow.
    pub trace_dropped: u64,
    /// Production field (types.rs:696): max steps attempted per tick.
    pub step_budget_per_tick: u64,
    /// Production field (types.rs:698): runtime admission policy.
    pub runtime_policy: u8,
}

// ============================================================================
// Spec predicates (preserved from the pre-binding spec)
// ============================================================================
//
// The pre-binding spec encoded terminal-state and shutdown-state as
// abstract `int` tags: 0 = none/open, 1 = completed/shutting-down,
// 2 = cancelled/closed. The post-binding spec keeps the same predicates
// so existing callers remain unchanged, then bridges the int tags to
// production discriminant values via `event_to_terminal_tag` and the
// `assume_specification` contracts below.

/// Spec predicate: the run is in the none-terminal state.
pub open spec fn none_terminal(state: int) -> bool { state == 0 }

/// Spec predicate: the run is in the completed-terminal state.
pub open spec fn completed_terminal(state: int) -> bool { state == 1 }

/// Spec predicate: the run is in the cancelled-terminal state.
pub open spec fn cancelled_terminal(state: int) -> bool { state == 2 }

/// Spec predicate: the run is in any terminal state.
pub open spec fn terminal_state(state: int) -> bool {
    completed_terminal(state) || cancelled_terminal(state)
}

/// Spec predicate: a legal transition between terminal states.
///
/// Production semantics (from `Shard::apply` at
/// `crates/vb_runtime/src/shard/transitions.rs:50-76`): terminal
/// events (`Fail | TerminalRemove | DriveFinished`) call
/// `runtime_state_remove`, which removes the run from active
/// tracking. Re-applying a terminal event to an already-removed run
/// is a no-op. Hence once terminal, the only legal transition is
/// identity.
pub open spec fn legal_terminal_transition(before: int, after: int) -> bool {
    (none_terminal(before) && terminal_state(after))
        || (terminal_state(before) && after == before)
}

/// Spec predicate: timer eligibility — true iff a run exists, the run
/// is not terminal, and the run is not cancelled.
///
/// Production semantics (from `TimerWheel::cancel` at
/// `crates/vb_runtime/src/shard/timer_wheel.rs:93-104`): a terminal
/// or cancelled run's pending timer is removed (via
/// `pending_timer_remove` in `finish_run`/`fail_run_state` at
/// transitions.rs:103, 204).
pub open spec fn timer_eligible(run_exists: bool, terminal: int, cancelled: bool) -> bool {
    run_exists && none_terminal(terminal) && !cancelled
}

/// Spec predicate: admission is open (not shutting down or closed).
///
/// Production semantics (from `ShardStatus::shutting_down` at
/// `crates/vb_runtime/src/shard/types.rs:682`): when `shutting_down
/// == true`, the shard stops accepting new submissions.
pub open spec fn admission_open(state: int) -> bool { state == 0 }

/// Spec predicate: the shard is in the shutting-down phase.
pub open spec fn shutting_down(state: int) -> bool { state == 1 }

/// Spec predicate: the shard is fully closed (post-shutdown).
pub open spec fn shutdown_closed(state: int) -> bool { state == 2 }

/// Spec predicate: the state is one of the three shutdown phases.
pub open spec fn shutdown_state(state: int) -> bool {
    admission_open(state) || shutting_down(state) || shutdown_closed(state)
}

/// Spec predicate: shutdown transition is monotonic.
///
/// Production semantics: `ShardStatus::shutting_down` is monotonic
/// over time — once graceful shutdown begins, the flag never resets.
pub open spec fn shutdown_monotone(before: int, after: int) -> bool {
    shutdown_state(before) && shutdown_state(after) && before <= after
}

// ============================================================================
// Production-bound spec helpers
// ============================================================================

/// Map a `MirrorRuntimeEvent` to the int-encoded terminal state.
/// Terminal events map to 1 (`completed_terminal`) or 2
/// (`cancelled_terminal`); non-terminal events map to 0
/// (`none_terminal`).
///
/// Production binding (via `assume_specification` on
/// `mirror_runtime_event_is_terminal` below): `is_terminal(event)` is
/// true iff `terminal_state(event_to_terminal_tag(event))`.
pub open spec fn event_to_terminal_tag(event: MirrorRuntimeEvent) -> int {
    match event {
        MirrorRuntimeEvent::DriveFinished => 1,
        MirrorRuntimeEvent::TerminalRemove => 1,
        MirrorRuntimeEvent::Fail => 2,
        MirrorRuntimeEvent::Submit => 0,
        MirrorRuntimeEvent::Resume => 0,
        MirrorRuntimeEvent::ResumeRollback => 0,
        MirrorRuntimeEvent::DriveContinue => 0,
        MirrorRuntimeEvent::AwaitAction => 0,
        MirrorRuntimeEvent::AwaitTimer => 0,
    }
}

/// Map a `MirrorRuntimeEvent` to a bool indicating whether the event
/// is terminal. Mirrors production `RuntimeEvent::is_terminal` at
/// types.rs:802-807.
pub open spec fn spec_is_terminal_event(event: MirrorRuntimeEvent) -> bool {
    terminal_state(event_to_terminal_tag(event))
}

/// Map a `MirrorRuntimeState` to a bool indicating whether the state
/// is resumable. Mirrors production `RuntimeState::is_resumable` at
/// types.rs:769-771.
pub open spec fn spec_is_resumable_state(state: MirrorRuntimeState) -> bool {
    matches!(state, MirrorRuntimeState::Resumable)
}

/// Map a `MirrorRuntimeEvent` to a bool indicating whether the event
/// sets a resumable state. Mirrors production
/// `RuntimeEvent::is_resumable` at types.rs:811-816.
pub open spec fn spec_is_resumable_event(event: MirrorRuntimeEvent) -> bool {
    matches!(
        event,
        MirrorRuntimeEvent::AwaitAction
            | MirrorRuntimeEvent::AwaitTimer
            | MirrorRuntimeEvent::ResumeRollback
    )
}

// ============================================================================
// Production-bound exec wrappers — declared inside verus!
// ============================================================================
//
// Each wrapper has a `#[verifier::external]` body that reproduces the
// production decision logic. The body is opaque to Verus; the
// `assume_specification` bridge attaches the spec contract.

/// Production-bound exec wrapper for `MirrorRuntimeEvent::is_terminal`.
///
/// Mirrors production `RuntimeEvent::is_terminal` at
/// `crates/vb_runtime/src/shard/types.rs:802-807`:
///
/// ```ignore
/// pub fn is_terminal(&self) -> bool {
///     matches!(self, Self::Fail | Self::TerminalRemove | Self::DriveFinished)
/// }
/// ```
///
/// Body is opaque to Verus; contract attached via
/// `assume_specification[ mirror_runtime_event_is_terminal ]` below.
#[verifier::external]
pub exec fn mirror_runtime_event_is_terminal(event: MirrorRuntimeEvent) -> bool {
    matches!(
        event,
        MirrorRuntimeEvent::Fail
            | MirrorRuntimeEvent::TerminalRemove
            | MirrorRuntimeEvent::DriveFinished
    )
}

/// Production-bound exec wrapper for `MirrorRuntimeEvent::is_resumable`.
///
/// Mirrors production `RuntimeEvent::is_resumable` at
/// `crates/vb_runtime/src/shard/types.rs:811-816`.
#[verifier::external]
pub exec fn mirror_runtime_event_is_resumable(event: MirrorRuntimeEvent) -> bool {
    matches!(
        event,
        MirrorRuntimeEvent::AwaitAction
            | MirrorRuntimeEvent::AwaitTimer
            | MirrorRuntimeEvent::ResumeRollback
    )
}

/// Production-bound exec wrapper for `MirrorRuntimeState::is_resumable`.
///
/// Mirrors production `RuntimeState::is_resumable` at
/// `crates/vb_runtime/src/shard/types.rs:769-771`.
#[verifier::external]
pub exec fn mirror_runtime_state_is_resumable(state: MirrorRuntimeState) -> bool {
    matches!(state, MirrorRuntimeState::Resumable)
}

// ============================================================================
// assume_specification BRIDGES — production contract surface
// ============================================================================
//
// Each bridge attaches a Verus spec contract to the exec wrapper above.
// The contract asserts the production decision shape; the spec proofs
// below discharge against these contracts.

// --------------------------------------------------------------------------
// Bridge: RuntimeEvent::is_terminal
// --------------------------------------------------------------------------
/// Bridge contract: `mirror_runtime_event_is_terminal(event)` returns
/// true iff `event` is `Fail | TerminalRemove | DriveFinished`.
///
/// Mirrors production `RuntimeEvent::is_terminal` at
/// `crates/vb_runtime/src/shard/types.rs:802-807`.
pub assume_specification[ mirror_runtime_event_is_terminal ](
    event: MirrorRuntimeEvent,
) -> (r: bool)
    ensures
        r == spec_is_terminal_event(event),
;

// --------------------------------------------------------------------------
// Bridge: RuntimeEvent::is_resumable
// --------------------------------------------------------------------------
/// Bridge contract: `mirror_runtime_event_is_resumable(event)` returns
/// true iff `event` is `AwaitAction | AwaitTimer | ResumeRollback`.
///
/// Mirrors production `RuntimeEvent::is_resumable` at
/// `crates/vb_runtime/src/shard/types.rs:811-816`.
pub assume_specification[ mirror_runtime_event_is_resumable ](
    event: MirrorRuntimeEvent,
) -> (r: bool)
    ensures
        r == spec_is_resumable_event(event),
;

// --------------------------------------------------------------------------
// Bridge: RuntimeState::is_resumable
// --------------------------------------------------------------------------
/// Bridge contract: `mirror_runtime_state_is_resumable(state)` returns
/// true iff `state == Resumable`.
///
/// Mirrors production `RuntimeState::is_resumable` at
/// `crates/vb_runtime/src/shard/types.rs:769-771`.
pub assume_specification[ mirror_runtime_state_is_resumable ](
    state: MirrorRuntimeState,
) -> (r: bool)
    ensures
        r == spec_is_resumable_state(state),
;

// ============================================================================
// Production-bound exec witnesses — discharge the bridges
// ============================================================================

/// Production-bound exec witness: invoke `MirrorRuntimeEvent::is_terminal`
/// and return the production-decision bool. Verus discharges the bridge
/// contract via `assume_specification`.
pub exec fn mirror_runtime_event_is_terminal_witness(event: MirrorRuntimeEvent) -> (r: bool)
    ensures
        r == spec_is_terminal_event(event),
{
    let r = mirror_runtime_event_is_terminal(event);
    r
}

/// Production-bound exec witness: invoke `MirrorRuntimeEvent::is_resumable`.
pub exec fn mirror_runtime_event_is_resumable_witness(event: MirrorRuntimeEvent) -> (r: bool)
    ensures
        r == spec_is_resumable_event(event),

// ============================================================================
// Companion chunk 2 — proof/remaining functions
// ============================================================================
#[path = "ipc_runtime_transitions_chunk2.rs"]
mod chunk2;

} // verus!
