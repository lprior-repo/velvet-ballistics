#![forbid(unsafe_code)]

use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult};
use vb_core::engine::EngineError;
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};
mod nested;
use self::nested::validate_nested_completion;

fn invalid(reason: &'static str) -> RuntimeEngineError {
    RuntimeEngineError::Core(EngineError::InvalidCompiledWorkflow { reason })
}

fn check_region(
    plan: &CompiledWorkflow,
    control: StepIdx,
    body: StepIdx,
) -> RuntimeEngineResult<()> {
    let region = plan
        .loop_region(control)
        .ok_or_else(|| invalid("loop_region cache miss for control step"))?;

    if region.body() != body || region.control() != control {
        return Err(invalid("loop_region cache body/control mismatch"));
    }

    let start = region.start();
    let done = region.done();

    if !(start.get() < body.get() && body.get() < control.get() && control.get() < done.get()) {
        return Err(invalid("loop_region index ordering violated"));
    }

    Ok(())
}

fn preflight(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    body: StepIdx,
    control: StepIdx,
) -> RuntimeEngineResult<()> {
    for raw in body.get()..=control.get() {
        let step = StepIdx::new(raw);
        let _node = plan.node(step).ok_or(RuntimeEngineError::Core(
            EngineError::InvalidProgramCounter { step },
        ))?;

        // Presence check only: nested entries are rearmable, so a missing
        // entry means the loop-region cache was not populated.
        if plan.nested_activation_blocked(step).is_none() {
            return Err(invalid("loop activation metadata missing"));
        }

        let state = run.step_state(step).map_err(RuntimeEngineError::Core)?;

        match state {
            StepState::Pending => {}
            StepState::Succeeded => {}
            StepState::Failed
                if run
                    .validate_handled_failure_rearm(plan, control, step)
                    .is_ok() => {}
            _ => {
                return Err(invalid("loop body step not in rearmable state"));
            }
        }
    }
    validate_nested_completion(plan, run, body, control)
}

fn reset_completed(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    body: StepIdx,
    control: StepIdx,
) -> RuntimeEngineResult<()> {
    // Discharge failures before resetting their successful handler witnesses.
    // Forward order preserves any later failed-handler witnesses as well.
    for raw in body.get()..control.get() {
        let step = StepIdx::new(raw);
        if run.step_state(step).map_err(RuntimeEngineError::Core)? == StepState::Failed {
            run.rearm_handled_failure(plan, control, step)
                .map_err(RuntimeEngineError::Core)?;
        }
    }
    for raw in body.get()..=control.get() {
        let step = StepIdx::new(raw);
        let state = run.step_state(step).map_err(RuntimeEngineError::Core)?;
        if state == StepState::Succeeded {
            run.mark_pending(step).map_err(RuntimeEngineError::Core)?;
        }
    }
    Ok(())
}

pub(super) fn loop_body_rearm(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    control: StepIdx,
    body: StepIdx,
) -> RuntimeEngineResult<()> {
    check_region(plan, control, body)?;

    if run.pc() != body
        || run.step_state(control).map_err(RuntimeEngineError::Core)? != StepState::Succeeded
    {
        return Err(invalid("loop rearm requires successful owned backedge"));
    }

    preflight(plan, run, body, control)?;
    reset_completed(plan, run, body, control)
}

/// Activation validation that can accept a step whose `StepStarted` is already
/// durable. `allow_running` is set only for a journal-authorized handler entry
/// (see `transitions::resumes_journaled_handler`); every other structural check
/// still applies.
pub(super) fn validate_activation(
    plan: &CompiledWorkflow,
    node: &CompiledNode,
    run: &RunFrame,
    allow_running: bool,
) -> RuntimeEngineResult<()> {
    validate_running_state(run, node, allow_running)?;
    validate_containing_loop_starts(plan, node.id, run)?;
    match plan.nested_activation_blocked(node.id) {
        Some(true) | Some(false) => {}
        None if requires_activation_metadata(&node.kind) => {
            return Err(invalid("loop activation metadata missing"));
        }
        None => {}
    }
    if let Some(body) = super::transitions::loop_body_target(node)
        && body < node.id
    {
        check_region(plan, node.id, body)?;
    }
    Ok(())
}
fn validate_running_state(
    run: &RunFrame,
    node: &CompiledNode,
    allow_running: bool,
) -> RuntimeEngineResult<()> {
    let state = run.step_state(node.id).map_err(RuntimeEngineError::Core)?;
    if state == StepState::Running && !allow_running {
        return Err(invalid("node already running"));
    }
    Ok(())
}

fn validate_containing_loop_starts(
    plan: &CompiledWorkflow,
    node_id: StepIdx,
    run: &RunFrame,
) -> RuntimeEngineResult<()> {
    for raw in 0..plan.node_count() {
        let control = StepIdx::new(raw);
        if let Some(region) = plan.loop_region(control)
            && region.body() <= node_id
            && node_id <= region.control()
        {
            let start_state = run
                .step_state(region.start())
                .map_err(RuntimeEngineError::Core)?;
            if start_state != StepState::Succeeded {
                return Err(invalid("containing loop region start not succeeded"));
            }
        }
    }
    Ok(())
}

fn requires_activation_metadata(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
        CompiledNodeKind::ForEachStart { .. }
            | CompiledNodeKind::ForEachNext { .. }
            | CompiledNodeKind::CollectStart { .. }
            | CompiledNodeKind::CollectNext { .. }
            | CompiledNodeKind::ReduceStart { .. }
            | CompiledNodeKind::ReduceNext { .. }
            | CompiledNodeKind::RepeatStart { .. }
            | CompiledNodeKind::RepeatCheck { .. }
            | CompiledNodeKind::TogetherStart { .. }
    )
}
