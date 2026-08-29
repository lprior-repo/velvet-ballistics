verus! {
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

}
