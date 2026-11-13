use super::super::{evidence, graph::Route, solve};
use super::engine::Solver;
use super::{Fact, FlowResult, SlotFlowFailure, failure};
use super::{dispatch, values};
use crate::workflow::{CompiledNode, CompiledNodeKind as K};

fn one(solver: &mut Solver<'_>, node: &CompiledNode, input: &[Fact]) -> FlowResult<()> {
    match node.kind {
        K::ForEachConcurrentStart { .. } => {
            values::fork_item(solver.parts, node, input, true)?;
            Ok(())
        }
        K::TogetherConcurrentStart { .. } | K::FirstSuccessStart { .. } => Ok(()),
        K::ConcurrentLaneEnd { .. } => {
            values::lane_result(node, input, true)?;
            Ok(())
        }
        K::ConcurrentGroupJoin { .. } => join(solver, node, input),
        _ => {
            solve::evaluate(solver.parts, node, input, true)?;
            Ok(())
        }
    }
}

fn join(solver: &mut Solver<'_>, node: &CompiledNode, input: &[Fact]) -> FlowResult<()> {
    let state = match dispatch::output(solver, node, input)? {
        Some(s) => s,
        None => {
            return Err(failure(
                node.id,
                node.output.slot(),
                SlotFlowFailure::UninitializedRead,
            ));
        }
    };
    let ctx = solve::context(solver.parts, node, &state, true)?;
    evidence::check(&ctx, Route::Next)
}

pub(super) fn validate(solver: &mut Solver<'_>) -> FlowResult<()> {
    let parts = solver.parts;
    for node in parts.nodes.iter() {
        let state = solver.store.state(node.id)?;
        if state.is_none() {
            continue;
        }
        solver.budget.transfer(parts, node)?;
        let frame = solver.store.frame_of(node.id)?;
        let input = solver.store.snapshot(frame, node.id)?;
        one(solver, node, &input)?;
    }
    Ok(())
}
