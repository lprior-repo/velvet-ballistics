#![forbid(unsafe_code)]

use crate::engine::types::{RuntimeEngineError, RuntimeEngineResult};
use vb_core::engine::EngineError;
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

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

        let blocked = plan
            .nested_activation_blocked(step)
            .ok_or_else(|| invalid("loop activation metadata missing"))?;

        let state = run.step_state(step).map_err(RuntimeEngineError::Core)?;

        match state {
            StepState::Pending => {}
            StepState::Succeeded if !blocked => {}
            StepState::Succeeded => {
                return Err(invalid("nested loop activation unsupported"));
            }
            _ => {
                return Err(invalid("loop body step not in rearmable state"));
            }
        }
    }
    Ok(())
}

fn reset_succeeded(run: &mut RunFrame, body: StepIdx, control: StepIdx) -> RuntimeEngineResult<()> {
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
    reset_succeeded(run, body, control)
}

pub(super) fn validate_activation(
    plan: &CompiledWorkflow,
    node: &CompiledNode,
) -> RuntimeEngineResult<()> {
    match plan.nested_activation_blocked(node.id) {
        Some(true) => Err(invalid("nested loop activation unsupported")),
        Some(false) => Ok(()),
        None if requires_activation_metadata(&node.kind) => {
            Err(invalid("loop activation metadata missing"))
        }
        None => Ok(()),
    }?;

    if let Some(body) = super::transitions::loop_body_target(node)
        && body < node.id
    {
        check_region(plan, node.id, body)?;
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
