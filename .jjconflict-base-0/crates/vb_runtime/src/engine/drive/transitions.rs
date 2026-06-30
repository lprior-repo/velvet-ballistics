#![forbid(unsafe_code)]

//! Drive-loop state transitions: begin/finish step and signal classification.

use vb_core::engine::{EngineError, StepBudget};
use vb_core::frame::RunFrame;
use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNode, CompiledWorkflow};

use crate::engine::drive::recovery::DriveState;
use crate::engine::drive::timers::try_consume_step_budget;
use crate::engine::helpers::mark_step_after_signal;
use crate::engine::types::{
    EvidenceCollector, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal,
};
use crate::primitives::collect::CollectStates;

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
    evidence
        .push_step_started(pc)
        .map_err(RuntimeEngineError::Core)?;
    run.mark_running(pc).map_err(RuntimeEngineError::Core)?;
    Ok(Some(DriveStep { pc, node }))
}

/// Emits slot evidence for the just-finished step, then commits the
/// matching run-frame step state (Succeeded for success-class signals,
/// Waiting/Asking/etc for suspension signals).
pub(super) fn finish_drive_step(
    run: &mut RunFrame,
    evidence: &mut EvidenceCollector,
    collect_states: &CollectStates,
    step: DriveStep<'_>,
    signal: &RuntimeSignal,
    drive_state: &mut DriveState,
) -> RuntimeEngineResult<()> {
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
            .push_step_succeeded(step.pc, step.node.output)
            .map_err(RuntimeEngineError::Core)?;
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
