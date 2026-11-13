#![forbid(unsafe_code)]

//! Drive-loop state transitions: begin/finish step and signal classification.

use vb_core::engine::{EngineError, StepBudget};
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::drive::recovery::DriveState;
use crate::engine::drive::timers::try_consume_step_budget;
use crate::engine::helpers::mark_step_after_signal;
use crate::engine::types::{
    EvidenceCollector, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal,
};
use crate::primitives::collect::CollectStates;
use crate::recovery::handler_route_for_failure;

/// Carries the per-step node reference plus its program counter through
/// the begin/finish transition pair.
pub(super) struct DriveStep<'a> {
    pub pc: StepIdx,
    pub node: &'a CompiledNode,
}

/// Consumes one step from the budget, fetches the current node, records
/// `StepStarted`, and marks the step as running on the run frame.
pub(super) fn begin_drive_step<'a>(
    plan: &'a CompiledWorkflow,
    run: &mut RunFrame,
    budget: &mut StepBudget,
    evidence: &mut EvidenceCollector,
) -> RuntimeEngineResult<Option<DriveStep<'a>>> {
    if !try_consume_step_budget(budget)? {
        return Ok(None);
    }
    let pc = run.pc();
    let node = plan
        .node(pc)
        .ok_or(EngineError::InvalidProgramCounter { step: pc })?;
    let resumed = resumes_journaled_handler(plan, node, run)?;
    super::loop_rearm::validate_activation(plan, node, run, resumed)?;
    if !resumed {
        evidence
            .push_step_started(pc)
            .map_err(RuntimeEngineError::Core)?;
        run.mark_running(pc).map_err(RuntimeEngineError::Core)?;
    }
    Ok(Some(DriveStep { pc, node }))
}

/// Reports whether `node` is a compiled error-handler target whose
/// `StepStarted` is already durable.
///
/// A failure routed to an authored handler journals `ActionFailed` and then
/// the resolved handler's `StepStarted` (and the `ErrorHandler` body's, when
/// the handler is a legacy wrapper) before executing it. A crash in that
/// window leaves those steps `Running` with no handler effect, so recovery
/// resumes the journaled step in place: re-emitting `StepStarted` would
/// duplicate durable evidence, and no action ticket or completion exists for
/// it.
///
/// Targets are resolved with the same table the failure path and the replay
/// repair use ([`handler_route_for_failure`]), so direct `on_error` routes and
/// legacy wrapper jump targets are treated identically. The candidate step must
/// already be `Failed`, so an unrelated `Running` node is never exempted.
fn resumes_journaled_handler(
    plan: &CompiledWorkflow,
    node: &CompiledNode,
    run: &RunFrame,
) -> RuntimeEngineResult<bool> {
    if run.step_state(node.id).map_err(RuntimeEngineError::Core)? != StepState::Running {
        return Ok(false);
    }
    for raw in 0..plan.node_count() {
        let Some(candidate) = plan.node(StepIdx::new(raw)) else {
            continue;
        };
        if run
            .step_state(candidate.id)
            .map_err(RuntimeEngineError::Core)?
            != StepState::Failed
        {
            continue;
        }
        let Some(route) = handler_route_for_failure(plan, candidate.id) else {
            continue;
        };
        if route.handler == node.id || route.handler_body == Some(node.id) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Extracts the body step index from an owning-loop control node, or
/// `None` for non-loop or join/finish nodes that cannot rearm.
pub(super) fn loop_body_target(node: &CompiledNode) -> Option<StepIdx> {
    match node.kind {
        CompiledNodeKind::ForEachNext { body, .. }
        | CompiledNodeKind::ReduceNext { body, .. }
        | CompiledNodeKind::CollectNext { body, .. } => Some(body),
        CompiledNodeKind::RepeatCheck { .. } => node.routing.next.step(),
        _ => None,
    }
}

/// After normal success and evidence commit, rearm the validated owned body
/// only when the latch takes its backward body edge. The helper checks the
/// complete region before resetting any succeeded body or latch state.
pub(super) fn owned_continuation_rearm(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    step: DriveStep<'_>,
) -> RuntimeEngineResult<()> {
    let Some(body) = loop_body_target(step.node) else {
        return Ok(());
    };
    if run.pc() != body {
        return Ok(());
    }
    if body >= step.pc {
        return Ok(());
    }
    super::loop_rearm::loop_body_rearm(plan, run, step.pc, body)
}

/// Emits slot evidence for the just-finished step, then commits the
/// matching run-frame step state (Succeeded for success-class signals,
/// Waiting/Asking/etc for suspension signals).
pub(super) fn finish_drive_step(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    evidence: &mut EvidenceCollector,
    collect_states: &CollectStates,
    step: DriveStep<'_>,
    signal: &RuntimeSignal,
    drive_state: &mut DriveState,
) -> RuntimeEngineResult<()> {
    if run.step_state(step.pc).map_err(RuntimeEngineError::Core)? == vb_core::StepState::Failed {
        super::actions::emit_error_slot_evidence(plan, run, evidence, step.node)?;
        evidence
            .push_step_failed(step.pc)
            .map_err(RuntimeEngineError::Core)?;
        return Ok(());
    }
    // RE-011: emit slot evidence BEFORE committing the step state. If
    // emit fails (e.g. CollectEvidenceCapacityExceeded), the step must
    // remain in its pre-success state so the caller sees a single
    // fail-closed Err rather than a half-committed Succeeded step.
    // Slot evidence is emitted regardless of signal class because slot
    // writes are observable even on AwaitingAction/AwaitingWait paths.
    //
    // RE-004: emit_slot_evidence now records a gap (and returns Ok) when
    // the underlying read_slot fails, instead of silently dropping both
    // the evidence and the error.
    super::actions::emit_slot_evidence(run, evidence, collect_states, step.node, drive_state)?;
    if signal_is_success(signal) {
        mark_step_after_signal(run, step.pc, signal)?;
        evidence
            .push_step_succeeded(step.pc, step.node.output.slot())
            .map_err(RuntimeEngineError::Core)?;
        owned_continuation_rearm(plan, run, step)?;
    } else {
        // AwaitingWait/AwaitingAsk/AwaitingAction/StepBudgetExhausted:
        // no StepSucceeded event, but slot evidence above is still valid
        // (or recorded as a gap if read_slot failed).
        mark_step_after_signal(run, step.pc, signal)?;
    }
    Ok(())
}

/// Returns true when the signal represents a step that completed
/// (Continue) or the workflow finished (Finished); false for every
/// suspension / exhaustion signal.
pub(super) const fn signal_is_success(signal: &RuntimeSignal) -> bool {
    matches!(signal, RuntimeSignal::Continue | RuntimeSignal::Finished(_))
}
