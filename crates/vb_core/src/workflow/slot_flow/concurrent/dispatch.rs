use super::super::{facts, solve};
use super::engine::Solver;
use super::{Fact, Facts, FlowResult, capacity, invalid};
use super::{fork, values};

use crate::workflow::concurrent_regions::ConcurrentMode;
use crate::workflow::{CompiledNode, CompiledNodeKind as K};

pub(super) fn step(
    solver: &mut Solver<'_>,
    frame: usize,
    node: &CompiledNode,
    input: &[Fact],
) -> FlowResult<()> {
    match node.kind {
        K::ForEachConcurrentStart { .. }
        | K::TogetherConcurrentStart { .. }
        | K::FirstSuccessStart { .. } => fork::run(solver, frame, node, input)?,
        K::ConcurrentLaneEnd { .. } => end(solver, frame, node, input)?,
        K::ConcurrentGroupJoin { .. } => join(solver, frame, node, input)?,
        _ => return old(solver, frame, node, input),
    }
    error(solver, frame, node, input)
}
fn old(
    solver: &mut Solver<'_>,
    frame: usize,
    node: &CompiledNode,
    input: &[Fact],
) -> FlowResult<()> {
    for (target, state) in solve::evaluate(solver.parts, node, input, false)? {
        solver
            .store
            .propagate(frame, target, state, &mut solver.budget)?;
    }
    Ok(())
}

fn error(
    solver: &mut Solver<'_>,
    frame: usize,
    node: &CompiledNode,
    input: &[Fact],
) -> FlowResult<()> {
    let partial = facts::copy(input)?;
    if let Some((target, state)) = solve::error_edge(solver.parts, node, partial)? {
        solver
            .store
            .propagate(frame, target, state, &mut solver.budget)?;
    }
    Ok(())
}

fn end(
    solver: &mut Solver<'_>,
    frame: usize,
    node: &CompiledNode,
    input: &[Fact],
) -> FlowResult<()> {
    let owner = match node.kind {
        K::ConcurrentLaneEnd { owner, .. } => owner,
        _ => return Err(invalid(node.id)),
    };

    let scope = solver.layout.scope(frame)?;
    if scope.as_ref().map(|(o, _)| *o) != Some(owner) {
        return Err(invalid(node.id));
    }

    let group = solver.layout.group(owner)?;
    let result = values::lane_result(node, input, false)?;

    let changed = observe(solver, group, result)?;

    if changed {
        let region = solver.layout.regions.get(group).ok_or_else(capacity)?;
        let parent_frame = solver.layout.frame(region.owner)?;
        solver.store.schedule(parent_frame, region.join)?;
    }

    Ok(())
}

fn observe(solver: &mut Solver<'_>, group: usize, result: Fact) -> FlowResult<bool> {
    let cell = solver.results.get_mut(group).ok_or_else(capacity)?;
    match cell {
        Some(prior) => Ok(prior.join(result)),
        None => {
            *cell = Some(result);
            Ok(true)
        }
    }
}

pub(super) fn output(
    solver: &Solver<'_>,
    node: &CompiledNode,
    input: &[Fact],
) -> FlowResult<Option<Facts>> {
    let owner = match node.kind {
        K::ConcurrentGroupJoin { owner } => owner,
        _ => return Err(invalid(node.id)),
    };

    let group = solver.layout.group(owner)?;
    let region = solver.layout.regions.get(group).ok_or_else(capacity)?;
    if region.join != node.id {
        return Err(invalid(node.id));
    }

    let successful = solver.results.get(group).copied().ok_or_else(capacity)?;

    if matches!(region.mode, ConcurrentMode::FirstSuccess) && successful.is_none() {
        return Ok(None);
    }

    let owner_node = solver
        .parts
        .nodes
        .get(owner.as_usize())
        .ok_or_else(|| invalid(owner))?;
    let value = values::join_value(owner_node, successful)?;
    Ok(Some(values::parent_after_join(input, node, value)?))
}

fn join(
    solver: &mut Solver<'_>,
    frame: usize,
    node: &CompiledNode,
    input: &[Fact],
) -> FlowResult<()> {
    if let Some(state) = output(solver, node, input)? {
        let next = node.routing.next.step().ok_or_else(|| invalid(node.id))?;
        solver
            .store
            .propagate(frame, next, state, &mut solver.budget)?;
    }
    Ok(())
}
