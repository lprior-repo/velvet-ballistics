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

        let state = run.step_state(step).map_err(RuntimeEngineError::Core)?;

        match state {
            StepState::Pending => {}
            StepState::Succeeded => {}
            _ => {
                return Err(invalid("loop body step not in rearmable state"));
            }
        }
    }
    validate_nested_completion(plan, run, body, control)
}

fn validate_nested_completion(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    body: StepIdx,
    control: StepIdx,
) -> RuntimeEngineResult<()> {
    for raw in 0..plan.node_count() {
        let id = StepIdx::new(raw);
        let Some(region) = plan.loop_region(id) else { continue; };
        if region.start() < body || region.control() >= control { continue; }
        let start = run.step_state(region.start()).map_err(RuntimeEngineError::Core)?;
        let latch = run.step_state(region.control()).map_err(RuntimeEngineError::Core)?;
        let untouched = (region.body().get()..=region.control().get())
            .map(StepIdx::new)
            .try_fold(true, |pending, step| {
                run.step_state(step).map(|state| pending && state == StepState::Pending)
                    .map_err(RuntimeEngineError::Core)
            })?;
        let valid = match start {
            StepState::Pending => untouched,
            StepState::Succeeded => {
                let done = run.step_state(region.done()).map_err(RuntimeEngineError::Core)?;
                let may_skip = plan.node(region.start()).is_some_and(|node| matches!(node.kind,
                    CompiledNodeKind::ForEachStart { .. } | CompiledNodeKind::CollectStart { .. }
                        | CompiledNodeKind::ReduceStart { .. }));
                done == StepState::Succeeded
                    && (latch == StepState::Succeeded || (may_skip && untouched))
            }
            _ => false,
        };
        if !valid { return Err(invalid("nested loop region not consistently completed")); }
    }
    for raw in body.get()..control.get() {
        let id = StepIdx::new(raw);
        let node = plan.node(id).ok_or_else(|| invalid("nested node missing"))?;
        if let CompiledNodeKind::TogetherStart { ref branches, join } = node.kind {
            let start = run.step_state(id).map_err(RuntimeEngineError::Core)?;
            let expected = if start == StepState::Succeeded { StepState::Succeeded } else { StepState::Pending };
            if run.step_state(join).map_err(RuntimeEngineError::Core)? != expected {
                return Err(invalid("nested together join not consistently completed"));
            }
            for branch in branches {
                if run.step_state(*branch).map_err(RuntimeEngineError::Core)? != expected {
                    return Err(invalid("nested together branch not consistently completed"));
                }
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
    run: &RunFrame,
) -> RuntimeEngineResult<()> {
    // Verify current node's step state
    let state = run.step_state(node.id).map_err(RuntimeEngineError::Core)?;
    if state == StepState::Running {
        return Err(invalid("node already running"));
    }

    // Validate every containing loop region start has Succeeded.
    // For nodes in a region's body/control interval, the region's start
    // must have completed before that node can be entered.
    for raw in 0..plan.node_count() {
        let control = StepIdx::new(raw);
        if let Some(region) = plan.loop_region(control) {
            if region.body() <= node.id && node.id <= region.control() {
                let start_state = run
                    .step_state(region.start())
                    .map_err(RuntimeEngineError::Core)?;
                if start_state != StepState::Succeeded {
                    return Err(invalid("containing loop region start not succeeded"));
                }
            }
        }
    }

    // Original validation for region structure and metadata
    match plan.nested_activation_blocked(node.id) {
        Some(true) => {
            // Blocked nodes are only valid if all containing starts succeeded,
            // which we've already checked above.
        }
        Some(false) => {}
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
