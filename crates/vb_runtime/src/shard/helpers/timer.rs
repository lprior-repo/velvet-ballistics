#![forbid(unsafe_code)]
//! Timer registration, validation, and firing helpers.

use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNodeKind, NodeContinuation};

use crate::shard::types::{PendingTimer, PendingTimerKind};
use crate::{RuntimeError, RuntimeResult};

/// Returns true if a timer must be registered for the given step.
pub fn timer_registration_required(state: &crate::shard::types::RunState, step: StepIdx) -> bool {
    let Some(node) = state.workflow.node(step) else {
        return false;
    };
    match node.kind {
        CompiledNodeKind::WaitUntil { .. } => true,
        CompiledNodeKind::WaitEvent { timeout_slot, .. }
        | CompiledNodeKind::Ask { timeout_slot, .. } => timeout_slot.slot().is_some(),
        _ => false,
    }
}

/// Advances state after a timer fires.
pub fn advance_after_timer_fire(
    state: &mut crate::shard::types::RunState,
    timer: PendingTimer,
) -> RuntimeResult<()> {
    validate_timer_fire(state, timer)?;
    if state
        .workflow
        .node(timer.step)
        .is_some_and(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
    {
        return Ok(());
    }
    let next = timer_next_step(state, timer.step)?;
    state
        .frame
        .mark_running(timer.step)
        .map_err(|_| RuntimeError::InvalidTimerFire)?;
    state
        .frame
        .mark_succeeded(timer.step)
        .map_err(|_| RuntimeError::InvalidTimerFire)?;
    state
        .frame
        .set_pc(next)
        .map_err(|_| RuntimeError::InvalidTimerFire)?;
    Ok(())
}

/// Validates a timer fire without mutating the run frame.
pub fn validate_timer_fire(
    state: &crate::shard::types::RunState,
    timer: PendingTimer,
) -> RuntimeResult<()> {
    let Some(node) = state.workflow.node(timer.step) else {
        return Err(RuntimeError::InvalidTimerFire);
    };
    match (timer.kind, &node.kind) {
        (
            PendingTimerKind::Wait,
            CompiledNodeKind::WaitUntil { .. } | CompiledNodeKind::WaitEvent { .. },
        )
        | (PendingTimerKind::Ask, CompiledNodeKind::Ask { .. }) => {}
        (PendingTimerKind::Wait, CompiledNodeKind::Do { .. })
            if node.retry.is_some_and(|policy| policy.delay_ms > 0)
                && state.frame.pc() == timer.step
                && state.frame.step_state(timer.step).ok()
                    == Some(vb_core::frame::StepState::Waiting) =>
        {
            return Ok(());
        }
        _ => return Err(RuntimeError::InvalidTimerFire),
    }
    if matches!(node.routing.next, NodeContinuation::End) {
        return Err(RuntimeError::InvalidTimerFire);
    }
    Ok(())
}
fn timer_next_step(state: &crate::shard::types::RunState, step: StepIdx) -> RuntimeResult<StepIdx> {
    state
        .workflow
        .node(step)
        .and_then(|node| node.routing.next.step())
        .ok_or(RuntimeError::InvalidTimerFire)
}

/// Current wall-clock time in milliseconds since the UNIX epoch, or None when
/// the system clock is before the epoch or the value does not fit in a u64.
pub fn wall_clock_millis_since_epoch() -> Option<u64> {
    #[cfg(test)]
    if let Some(millis) = test_wall_clock_millis() {
        return millis;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

/// Runtime clock reading as a result; an unavailable clock is an invariant violation.
pub fn submission_time() -> RuntimeResult<u64> {
    wall_clock_millis_since_epoch().ok_or_else(|| RuntimeError::Core {
        source: Box::new(vb_core::errors::CoreError::InternalInvariantViolation {
            reason: "wall_clock_unavailable".into(),
        }),
    })
}

#[cfg(test)]
thread_local! {
    static TEST_WALL_CLOCK_MILLIS: core::cell::Cell<Option<Option<u64>>> =
        const { core::cell::Cell::new(None) };
}

#[cfg(test)]
fn test_wall_clock_millis() -> Option<Option<u64>> {
    TEST_WALL_CLOCK_MILLIS.with(|cell| cell.get())
}

/// Installs a deterministic wall clock for the current test thread.
#[cfg(test)]
pub(crate) fn install_wall_clock_millis_for_test(millis: u64) {
    TEST_WALL_CLOCK_MILLIS.with(|cell| cell.set(Some(Some(millis))));
}

#[cfg(test)]
pub(crate) fn install_wall_clock_unavailable_for_test() {
    TEST_WALL_CLOCK_MILLIS.with(|cell| cell.set(Some(None)));
}

/// Restores the real wall clock for the current test thread.
#[cfg(test)]
pub(crate) fn restore_wall_clock_for_test() {
    TEST_WALL_CLOCK_MILLIS.with(|cell| cell.set(None));
}

/// Restores the real wall clock when dropped, including on test panic.
#[cfg(test)]
pub(crate) struct WallClockGuard;

#[cfg(test)]
impl Drop for WallClockGuard {
    fn drop(&mut self) {
        restore_wall_clock_for_test();
    }
}

/// Installs a deterministic wall clock and returns a guard that restores it on drop.
#[cfg(test)]
pub(crate) fn install_wall_clock_millis_guard(millis: u64) -> WallClockGuard {
    install_wall_clock_millis_for_test(millis);
    WallClockGuard
}
