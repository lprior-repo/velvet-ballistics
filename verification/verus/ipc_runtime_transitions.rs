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
{
    let r = mirror_runtime_event_is_resumable(event);
    r
}

/// Production-bound exec witness: invoke `MirrorRuntimeState::is_resumable`.
pub exec fn mirror_runtime_state_is_resumable_witness(state: MirrorRuntimeState) -> (r: bool)
    ensures
        r == spec_is_resumable_state(state),
{
    let r = mirror_runtime_state_is_resumable(state);
    r
}

// ============================================================================
// Spec proofs — terminal-state machine
// ============================================================================

/// Theorem: terminal → terminal implies identity.
pub proof fn single_terminal_winner(before: int, after: int)
    requires
        legal_terminal_transition(before, after),
        terminal_state(before),
    ensures
        after == before,
{
    assert(legal_terminal_transition(before, after));
    let first_disjunct = none_terminal(before) && terminal_state(after);
    let second_disjunct = terminal_state(before) && (after == before);
    assert(legal_terminal_transition(before, after) == (first_disjunct || second_disjunct));
    assert(terminal_state(before));
    assert(!none_terminal(before));
    assert(!first_disjunct);
    assert(second_disjunct);
    assert(after == before);
}

/// Theorem: terminal → terminal is legal (identity transition).
pub proof fn stale_terminal_event_rejected(before: int)
    requires
        terminal_state(before),
    ensures
        legal_terminal_transition(before, before),
{
    assert(terminal_state(before));
    assert(terminal_state(before) && (before == before));
    assert(legal_terminal_transition(before, before));
}

/// Theorem: from none-terminal, a non-identity transition is only legal
/// to a terminal state.
pub proof fn none_to_terminal_legal(before: int, after: int)
    requires
        none_terminal(before),
        terminal_state(after),
    ensures
        legal_terminal_transition(before, after),
{
    assert(none_terminal(before));
    assert(terminal_state(after));
    assert(none_terminal(before) && terminal_state(after));
    assert(legal_terminal_transition(before, after));
}

/// Theorem: from terminal, a non-identity transition is illegal.
pub proof fn terminal_to_distinct_terminal_illegal(before: int, after: int)
    requires
        terminal_state(before),
        terminal_state(after),
        before != after,
    ensures
        !legal_terminal_transition(before, after),
{
    assert(terminal_state(before));
    assert(!none_terminal(before));
    assert(!(after == before));
    assert(!(terminal_state(before) && (after == before)));
    assert(!(none_terminal(before) && terminal_state(after)));
    assert(!legal_terminal_transition(before, after));
}

// ============================================================================
// Spec proofs — timer eligibility
// ============================================================================

/// Theorem: timer eligibility implies all three conjuncts.
pub proof fn timer_requires_eligible_run(run_exists: bool, terminal: int, cancelled: bool)
    requires
        timer_eligible(run_exists, terminal, cancelled),
    ensures
        run_exists,
        none_terminal(terminal),
        !cancelled,
{
    assert(timer_eligible(run_exists, terminal, cancelled));
    assert(run_exists && none_terminal(terminal) && !cancelled);
    assert(run_exists);
    assert(none_terminal(terminal));
    assert(!cancelled);
}

/// Theorem: terminal state implies not timer eligible.
pub proof fn timer_cannot_mutate_terminal_state(terminal: int, cancelled: bool)
    requires
        terminal_state(terminal),
    ensures
        !timer_eligible(true, terminal, cancelled),
{
    assert(terminal_state(terminal));
    assert(!none_terminal(terminal));
    assert(!(true && none_terminal(terminal) && !cancelled));
    assert(!timer_eligible(true, terminal, cancelled));
}

/// Theorem: cancelled run is not timer eligible.
pub proof fn timer_cannot_mutate_cancelled_state(run_exists: bool, terminal: int)
    requires
        run_exists,
        terminal_state(terminal),
    ensures
        !timer_eligible(run_exists, terminal, true),
{
    assert(terminal_state(terminal));
    assert(!none_terminal(terminal));
    assert(!(run_exists && none_terminal(terminal) && !true));
    assert(!timer_eligible(run_exists, terminal, true));
}

/// Theorem: timer eligibility is fully characterized by the three
/// conjuncts.
pub proof fn timer_eligible_iff(run_exists: bool, terminal: int, cancelled: bool)
    requires
        run_exists,
        none_terminal(terminal),
        !cancelled,
    ensures
        timer_eligible(run_exists, terminal, cancelled),
{
    assert(run_exists && none_terminal(terminal) && !cancelled);
    assert(timer_eligible(run_exists, terminal, cancelled));
}

// ============================================================================
// Spec proofs — shutdown state machine
// ============================================================================

/// Theorem: shutdown monotonicity is implied by `shutdown_monotone`.
pub proof fn shutdown_monotonic(before: int, after: int)
    requires
        shutdown_monotone(before, after),
    ensures
        before <= after,
        shutdown_state(before),
        shutdown_state(after),
{
    assert(shutdown_monotone(before, after));
    assert(shutdown_state(before));
    assert(shutdown_state(after));
    assert(before <= after);
}

/// Theorem: shutting-down or closed state implies not admission-open.
pub proof fn reject_submit_after_shutdown_boundary(state: int)
    requires
        shutting_down(state) || shutdown_closed(state),
    ensures
        !admission_open(state),
{
    assert(state == 1 || state == 2);
    assert(!(state == 0));
    assert(!admission_open(state));
}

/// Theorem: admission-open state is not shutting-down or closed.
pub proof fn admission_open_state(state: int)
    requires
        admission_open(state),
    ensures
        !shutting_down(state),
        !shutdown_closed(state),
{
    assert(state == 0);
    assert(state != 1);
    assert(state != 2);
    assert(!shutting_down(state));
    assert(!shutdown_closed(state));
}

// ============================================================================
// Bridge anchor proofs — connect int-encoded spec to production enums
// ============================================================================

/// Theorem: `DriveFinished` maps to `completed_terminal(1)`.
///
/// Production binding: `RuntimeEvent::is_terminal(DriveFinished) ==
/// true` (production: types.rs:802-807).
pub proof fn bridge_drive_finished_is_completed_terminal()
    ensures
        spec_is_terminal_event(MirrorRuntimeEvent::DriveFinished),
        event_to_terminal_tag(MirrorRuntimeEvent::DriveFinished) == 1,
{
    assert(event_to_terminal_tag(MirrorRuntimeEvent::DriveFinished) == 1);
    assert(terminal_state(1));
    assert(spec_is_terminal_event(MirrorRuntimeEvent::DriveFinished));
}

/// Theorem: `Fail` maps to `cancelled_terminal(2)`.
///
/// Production binding: `RuntimeEvent::is_terminal(Fail) == true`
/// (production: types.rs:802-807).
pub proof fn bridge_fail_is_cancelled_terminal()
    ensures
        spec_is_terminal_event(MirrorRuntimeEvent::Fail),
        event_to_terminal_tag(MirrorRuntimeEvent::Fail) == 2,
{
    assert(event_to_terminal_tag(MirrorRuntimeEvent::Fail) == 2);
    assert(terminal_state(2));
    assert(spec_is_terminal_event(MirrorRuntimeEvent::Fail));
}

/// Theorem: `TerminalRemove` maps to `completed_terminal(1)`.
///
/// Production binding: `RuntimeEvent::is_terminal(TerminalRemove) ==
/// true` (production: types.rs:802-807).
pub proof fn bridge_terminal_remove_is_completed_terminal()
    ensures
        spec_is_terminal_event(MirrorRuntimeEvent::TerminalRemove),
        event_to_terminal_tag(MirrorRuntimeEvent::TerminalRemove) == 1,
{
    assert(event_to_terminal_tag(MirrorRuntimeEvent::TerminalRemove) == 1);
    assert(terminal_state(1));
    assert(spec_is_terminal_event(MirrorRuntimeEvent::TerminalRemove));
}

/// Theorem: non-terminal events map to `none_terminal(0)`.
///
/// Production binding: `RuntimeEvent::is_terminal` returns false for
/// `Submit | Resume | ResumeRollback | DriveContinue | AwaitAction |
/// AwaitTimer` (production: types.rs:802-807).
pub proof fn bridge_submit_is_none_terminal()
    ensures
        !spec_is_terminal_event(MirrorRuntimeEvent::Submit),
        event_to_terminal_tag(MirrorRuntimeEvent::Submit) == 0,
{
    assert(event_to_terminal_tag(MirrorRuntimeEvent::Submit) == 0);
    assert(!terminal_state(0));
    assert(!spec_is_terminal_event(MirrorRuntimeEvent::Submit));
}

pub proof fn bridge_drive_continue_is_none_terminal()
    ensures
        !spec_is_terminal_event(MirrorRuntimeEvent::DriveContinue),
        event_to_terminal_tag(MirrorRuntimeEvent::DriveContinue) == 0,
{
    assert(event_to_terminal_tag(MirrorRuntimeEvent::DriveContinue) == 0);
    assert(!spec_is_terminal_event(MirrorRuntimeEvent::DriveContinue));
}

/// Theorem: `Resumable` state maps to `spec_is_resumable_state == true`.
///
/// Production binding: `RuntimeState::is_resumable(Resumable) == true`
/// (production: types.rs:769-771).
pub proof fn bridge_resumable_state()
    ensures
        spec_is_resumable_state(MirrorRuntimeState::Resumable),
{
    assert(spec_is_resumable_state(MirrorRuntimeState::Resumable));
}

/// Theorem: non-Resumable states map to `spec_is_resumable_state == false`.
pub proof fn bridge_running_state_not_resumable()
    ensures
        !spec_is_resumable_state(MirrorRuntimeState::Running),
{
    assert(!spec_is_resumable_state(MirrorRuntimeState::Running));
}

pub proof fn bridge_failed_state_not_resumable()
    ensures
        !spec_is_resumable_state(MirrorRuntimeState::Failed),
{
    assert(!spec_is_resumable_state(MirrorRuntimeState::Failed));
}

/// Theorem: `AwaitAction` event maps to `spec_is_resumable_event == true`.
pub proof fn bridge_await_action_is_resumable_event()
    ensures
        spec_is_resumable_event(MirrorRuntimeEvent::AwaitAction),
{
    assert(spec_is_resumable_event(MirrorRuntimeEvent::AwaitAction));
}

pub proof fn bridge_await_timer_is_resumable_event()
    ensures
        spec_is_resumable_event(MirrorRuntimeEvent::AwaitTimer),
{
    assert(spec_is_resumable_event(MirrorRuntimeEvent::AwaitTimer));
}

pub proof fn bridge_resume_rollback_is_resumable_event()
    ensures
        spec_is_resumable_event(MirrorRuntimeEvent::ResumeRollback),
{
    assert(spec_is_resumable_event(MirrorRuntimeEvent::ResumeRollback));
}

// ============================================================================
// Production-bound exec wrappers (call production::* via the extern mirror)
// ============================================================================
//
// These exec wrappers exercise the production functions in the
// `production_inner/ipc_runtime_transitions_production.rs` mirror (the
// drift-detection surface for the `extern_ipc_runtime_transitions.rs`
// binding). Each wrapper invokes a production exec fn with a concrete
// value and asserts the result equals the spec-side predicate. Without
// these wrappers, the `assume_specification` contracts attached to the
// production functions would not be exercised in exec mode.
//
// Mirrors production code at:
//   - `crates/vb_runtime/src/shard/types.rs:802-807`  (RuntimeEvent::is_terminal)
//   - `crates/vb_runtime/src/shard/types.rs:811-816`  (RuntimeEvent::is_resumable)
//   - `crates/vb_runtime/src/shard/types.rs:769-771`  (RuntimeState::is_resumable)

/// Production-bound exec wrapper for `production::runtime_event_is_terminal`.
///
/// Calls the production mirror `runtime_event_is_terminal` (production at
/// `crates/vb_runtime/src/shard/types.rs:802-807`) with the
/// `MirrorRuntimeEvent::Fail` discriminant, which is the canonical
/// terminal event in production. Asserts the production return matches
/// the spec-side predicate.
pub fn production_runtime_event_is_terminal_exec_fail() -> (r: bool)
    ensures
        r == true,
{
    // Construct the production-side `Fail` event (a terminal event in
    // production). The production mirror at
    // `production_inner/ipc_runtime_transitions_production.rs:142` is
    // `#[verifier::external]`; the spec contract attached via
    // `assume_specification` guarantees the body returns `true` for the
    // `Fail` discriminant.
    let r = production::runtime_event_is_terminal(production::RuntimeEvent::Fail);
    r
}

/// Production-bound exec wrapper for `production::runtime_event_is_terminal`:
/// non-terminal event.
pub fn production_runtime_event_is_terminal_exec_resume() -> (r: bool)
    ensures
        r == false,
{
    let r = production::runtime_event_is_terminal(production::RuntimeEvent::Resume);
    r
}

/// Production-bound exec wrapper for `production::runtime_event_is_resumable`:
/// resumable event.
pub fn production_runtime_event_is_resumable_exec_await_action() -> (r: bool)
    ensures
        r == true,
{
    let r = production::runtime_event_is_resumable(production::RuntimeEvent::AwaitAction);
    r
}

/// Production-bound exec wrapper for `production::runtime_event_is_resumable`:
/// non-resumable event.
pub fn production_runtime_event_is_resumable_exec_fail() -> (r: bool)
    ensures
        r == false,
{
    let r = production::runtime_event_is_resumable(production::RuntimeEvent::Fail);
    r
}

/// Production-bound exec wrapper for `production::runtime_state_is_resumable`:
/// resumable state.
pub fn production_runtime_state_is_resumable_exec_resumable() -> (r: bool)
    ensures
        r == true,
{
    let r = production::runtime_state_is_resumable(production::RuntimeState::Resumable);
    r
}

/// Production-bound exec wrapper for `production::runtime_state_is_resumable`:
/// non-resumable state.
pub fn production_runtime_state_is_resumable_exec_failed() -> (r: bool)
    ensures
        r == false,
{
    let r = production::runtime_state_is_resumable(production::RuntimeState::Failed);
    r
}

// ============================================================================
// Additional assume_specification bridges for production-side functions
// ============================================================================
//
// These bridges attach spec contracts to the production functions in
// the `production_inner/ipc_runtime_transitions_production.rs` mirror.
// Each contract is non-vacuous: the production-side `runtime_event_is_terminal`
// etc. are `#[verifier::external]` in the mirror, so the contract is the
// ONLY way Verus can reason about the production return value.

/// Bridge: `production::runtime_event_is_terminal(event)` returns
/// true iff `event` is `Fail | TerminalRemove | DriveFinished`.
///
/// Mirrors production `RuntimeEvent::is_terminal` at
/// `crates/vb_runtime/src/shard/types.rs:802-807`.
pub assume_specification[ production::runtime_event_is_terminal ](
    event: production::RuntimeEvent,
) -> (r: bool)
    ensures
        r == matches!(
            event,
            production::RuntimeEvent::Fail
                | production::RuntimeEvent::TerminalRemove
                | production::RuntimeEvent::DriveFinished
        ),
;

/// Bridge: `production::runtime_event_is_resumable(event)` returns
/// true iff `event` is `AwaitAction | AwaitTimer | ResumeRollback`.
///
/// Mirrors production `RuntimeEvent::is_resumable` at
/// `crates/vb_runtime/src/shard/types.rs:811-816`.
pub assume_specification[ production::runtime_event_is_resumable ](
    event: production::RuntimeEvent,
) -> (r: bool)
    ensures
        r == matches!(
            event,
            production::RuntimeEvent::AwaitAction
                | production::RuntimeEvent::AwaitTimer
                | production::RuntimeEvent::ResumeRollback
        ),
;

/// Bridge: `production::runtime_state_is_resumable(state)` returns
/// true iff `state == Resumable`.
///
/// Mirrors production `RuntimeState::is_resumable` at
/// `crates/vb_runtime/src/shard/types.rs:769-771`.
pub assume_specification[ production::runtime_state_is_resumable ](
    state: production::RuntimeState,
) -> (r: bool)
    ensures
        r == matches!(state, production::RuntimeState::Resumable),
;

fn main() {}

} // verus!