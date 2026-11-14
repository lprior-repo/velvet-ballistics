use super::invalid;
use crate::engine::types::RuntimeEngineResult;
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledLoopRegion, CompiledNodeKind, CompiledWorkflow};

pub(super) fn validate_nested_completion(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    body: StepIdx,
    control: StepIdx,
) -> RuntimeEngineResult<()> {
    validate_nested_regions(plan, run, body, control)?;
    validate_nested_together(plan, run, body, control)
}

fn validate_nested_regions(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    body: StepIdx,
    control: StepIdx,
) -> RuntimeEngineResult<()> {
    for raw in 0..plan.node_count() {
        let id = StepIdx::new(raw);
        let Some(region) = plan.loop_region(id) else {
            continue;
        };
        if region.start() < body || region.control() >= control {
            continue;
        }
        validate_nested_region(plan, run, control, region)?;
    }
    Ok(())
}

fn validate_nested_region(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    control: StepIdx,
    region: &CompiledLoopRegion,
) -> RuntimeEngineResult<()> {
    let start = run
        .step_state(region.start())
        .map_err(crate::engine::types::RuntimeEngineError::Core)?;
    if matches!(start, StepState::Succeeded | StepState::Failed)
        && recovered_exit(plan, run, control, region.start(), region.control())?
    {
        return Ok(());
    }
    let latch = run
        .step_state(region.control())
        .map_err(crate::engine::types::RuntimeEngineError::Core)?;
    let untouched = nested_region_untouched(run, region.body(), region.control())?;
    let valid = match start {
        StepState::Pending => untouched,
        StepState::Succeeded => nested_region_completed(plan, run, region, latch, untouched)?,
        _ => false,
    };
    if !valid {
        return Err(invalid("nested loop region not consistently completed"));
    }
    Ok(())
}

fn nested_region_untouched(
    run: &RunFrame,
    body: StepIdx,
    control: StepIdx,
) -> RuntimeEngineResult<bool> {
    (body.get()..=control.get())
        .map(StepIdx::new)
        .try_fold(true, |pending, step| {
            run.step_state(step)
                .map(|state| pending && state == StepState::Pending)
                .map_err(crate::engine::types::RuntimeEngineError::Core)
        })
}

fn nested_region_completed(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    region: &CompiledLoopRegion,
    latch: StepState,
    untouched: bool,
) -> RuntimeEngineResult<bool> {
    let done = run
        .step_state(region.done())
        .map_err(crate::engine::types::RuntimeEngineError::Core)?;
    let may_skip = plan.node(region.start()).is_some_and(|node| {
        matches!(
            node.kind,
            CompiledNodeKind::ForEachStart { .. }
                | CompiledNodeKind::CollectStart { .. }
                | CompiledNodeKind::ReduceStart { .. }
        )
    });
    Ok(done == StepState::Succeeded && (latch == StepState::Succeeded || (may_skip && untouched)))
}

fn validate_nested_together(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    body: StepIdx,
    control: StepIdx,
) -> RuntimeEngineResult<()> {
    for raw in body.get()..control.get() {
        let id = StepIdx::new(raw);
        let node = plan
            .node(id)
            .ok_or_else(|| invalid("nested node missing"))?;
        if let CompiledNodeKind::TogetherStart { ref branches, join } = node.kind {
            validate_together_completion(plan, run, control, id, join, branches)?;
        }
    }
    Ok(())
}

fn validate_together_completion(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    control: StepIdx,
    start: StepIdx,
    join: StepIdx,
    branches: &[StepIdx],
) -> RuntimeEngineResult<()> {
    let state = run
        .step_state(start)
        .map_err(crate::engine::types::RuntimeEngineError::Core)?;
    if matches!(state, StepState::Succeeded | StepState::Failed)
        && recovered_exit(plan, run, control, start, join)?
    {
        return Ok(());
    }
    let expected = if state == StepState::Succeeded {
        StepState::Succeeded
    } else {
        StepState::Pending
    };
    validate_together_join(run, join, expected)?;
    validate_together_branches(run, branches, expected)
}

fn validate_together_join(
    run: &RunFrame,
    join: StepIdx,
    expected: StepState,
) -> RuntimeEngineResult<()> {
    if run
        .step_state(join)
        .map_err(crate::engine::types::RuntimeEngineError::Core)?
        != expected
    {
        return Err(invalid("nested together join not consistently completed"));
    }
    Ok(())
}

fn validate_together_branches(
    run: &RunFrame,
    branches: &[StepIdx],
    expected: StepState,
) -> RuntimeEngineResult<()> {
    for branch in branches {
        if run
            .step_state(*branch)
            .map_err(crate::engine::types::RuntimeEngineError::Core)?
            != expected
        {
            return Err(invalid("nested together branch not consistently completed"));
        }
    }
    Ok(())
}
fn recovered_exit(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    control: StepIdx,
    start: StepIdx,
    end: StepIdx,
) -> RuntimeEngineResult<bool> {
    for raw in start.get()..=end.get() {
        let failed = StepIdx::new(raw);
        if run
            .step_state(failed)
            .map_err(crate::engine::types::RuntimeEngineError::Core)?
            == StepState::Failed
            && plan
                .node(failed)
                .and_then(|node| node.routing.error.handler())
                .is_some_and(|handler| handler > end)
            && run
                .validate_handled_failure_rearm(plan, control, failed)
                .is_ok()
        {
            return Ok(true);
        }
    }
    Ok(false)
}
