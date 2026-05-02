//! Timer wheel and timer-related state machine helpers.

use vb_core::ids::StepIdx;
use vb_core::workflow::CompiledNodeKind;

use crate::command::PendingTimer;
use crate::run_state::RunState;
use crate::RuntimeError;
use crate::RuntimeResult;

/// Returns true if a timer registration is required for the given step.
pub fn timer_registration_required(state: &RunState, step: StepIdx) -> bool {
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

/// Advances run state after a timer fires.
pub fn advance_after_timer_fire(state: &mut RunState, timer: PendingTimer) -> RuntimeResult<()> {
    let Some(node) = state.workflow.node(timer.step) else {
        return Err(RuntimeError::InvalidTimerFire);
    };
    match (timer.kind, &node.kind) {
        (crate::command::PendingTimerKind::Wait, CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. })
        | (crate::command::PendingTimerKind::Ask, CompiledNodeKind::Ask { .. }) => {}
        _ => return Err(RuntimeError::InvalidTimerFire),
    }
    state
        .frame
        .mark_running(timer.step)
        .map_err(|_| RuntimeError::InvalidTimerFire)?;
    state
        .frame
        .mark_succeeded(timer.step)
        .map_err(|_| RuntimeError::InvalidTimerFire)?;
    let Some(next) = node.next else {
        return Err(RuntimeError::InvalidTimerFire);
    };
    state
        .frame
        .set_pc(next)
        .map_err(|_| RuntimeError::InvalidTimerFire)?;
    Ok(())
}
