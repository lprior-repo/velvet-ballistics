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
        | CompiledNodeKind::Ask { timeout_slot, .. } => timeout_slot.is_some(),
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
    let next = state
        .workflow
        .node(timer.step)
        .and_then(|node| node.routing.next.step())
        .ok_or(RuntimeError::InvalidTimerFire)?;
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

/// Current wall-clock time in milliseconds since the UNIX epoch, or None when
/// the system clock is before the epoch or the value does not fit in a u64.
pub fn wall_clock_millis_since_epoch() -> Option<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}
