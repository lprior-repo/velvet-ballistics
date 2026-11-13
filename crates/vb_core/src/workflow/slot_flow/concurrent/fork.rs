use super::super::facts;
use super::engine::Solver;
use super::values;
use super::{Fact, Facts, FlowResult, capacity, invalid};
use crate::workflow::{CompiledNode, CompiledNodeKind as K};

pub(super) fn run(
    solver: &mut Solver<'_>,
    frame: usize,
    node: &CompiledNode,
    input: &[Fact],
) -> FlowResult<()> {
    let index = solver.layout.group(node.id)?;
    let region = solver.layout.regions.get(index).ok_or_else(capacity)?;
    let join = region.join;

    solver
        .store
        .propagate(frame, join, facts::copy(input)?, &mut solver.budget)?;

    let lanes = solver
        .layout
        .child_frames
        .get(index)
        .ok_or_else(capacity)?
        .len();

    for lane in 0..lanes {
        let child = child_frame(solver, index, lane)?;

        seed(solver, node, child, input)?;
    }

    Ok(())
}

fn child_frame(solver: &Solver<'_>, index: usize, lane: usize) -> FlowResult<usize> {
    solver
        .layout
        .child_frames
        .get(index)
        .ok_or_else(capacity)?
        .get(lane)
        .copied()
        .ok_or_else(capacity)
}

fn seed(
    solver: &mut Solver<'_>,
    node: &CompiledNode,
    child: usize,
    input: &[Fact],
) -> FlowResult<()> {
    let owner = solver.layout.scope(child)?;
    let (owner_id, entry) = match owner {
        Some(o) => o,
        None => return Err(invalid(node.id)),
    };
    if owner_id != node.id {
        return Err(invalid(node.id));
    }

    let seed_state = select_seed(solver, node, input)?;
    solver
        .store
        .propagate(child, entry, seed_state, &mut solver.budget)?;

    Ok(())
}

fn select_seed(solver: &mut Solver<'_>, node: &CompiledNode, input: &[Fact]) -> FlowResult<Facts> {
    match node.kind {
        K::ForEachConcurrentStart { .. } => values::fork_item(solver.parts, node, input, false),
        K::TogetherConcurrentStart { .. } | K::FirstSuccessStart { .. } => facts::copy(input),
        _ => Err(invalid(node.id)),
    }
}
