#![forbid(unsafe_code)]
//! Timer-related helpers.

use vb_core::ids::StepIdx;
use vb_core::workflow::CompiledNodeKind;

use crate::shard::types::{PendingTimer, PendingTimerKind, RunState};
use crate::{RuntimeError, RuntimeResult};

/// Returns true if a timer must be registered for the given step.
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

/// Advances state after a timer fires.
///
/// The successor PC is validated BEFORE any state mutation. Mutating
/// `mark_running` / `mark_succeeded` first and only then discovering that
/// the node has no `next` successor leaves the step transitioned to
/// `Succeeded` while the PC was never advanced, corrupting run state and
/// silently breaking resume. All preconditions are checked up front; if
/// any precondition fails the run state is returned unchanged.
pub fn advance_after_timer_fire(state: &mut RunState, timer: PendingTimer) -> RuntimeResult<()> {
    let Some(node) = state.workflow.node(timer.step) else {
        return Err(RuntimeError::InvalidTimerFire);
    };
    match (timer.kind, &node.kind) {
        (
            PendingTimerKind::Wait,
            CompiledNodeKind::WaitUntil { .. } | CompiledNodeKind::WaitEvent { .. },
        )
        | (PendingTimerKind::Ask, CompiledNodeKind::Ask { .. }) => {}
        _ => return Err(RuntimeError::InvalidTimerFire),
    }
    // Validate successor PC BEFORE mutating any frame state. If the node has
    // no successor there is no legal PC transition and any prior step-state
    // mutation would leave the frame in an inconsistent Succeeded-but-no-PC
    // condition. Fail closed with state untouched.
    let Some(next) = node.next else {
        return Err(RuntimeError::InvalidTimerFire);
    };
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
