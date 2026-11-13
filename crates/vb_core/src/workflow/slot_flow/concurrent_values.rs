use super::{Fact, Facts, FlowResult, failure};
use crate::workflow::SlotFlowFailure::{InvalidReference, UninitializedRead, UnsupportedSemantics};
use crate::workflow::{CompiledNode, CompiledNodeKind as K, WorkflowParts};

pub(super) fn lane_result(node: &CompiledNode, input: &[Fact], checking: bool) -> FlowResult<Fact> {
    match &node.kind {
        K::ConcurrentLaneEnd {
            result: Some(slot), ..
        } => {
            let fact = input
                .get(slot.as_usize())
                .copied()
                .ok_or_else(|| failure(node.id, Some(*slot), InvalidReference))?;
            if checking && fact.absent {
                return Err(failure(node.id, Some(*slot), UninitializedRead));
            }
            Ok(fact)
        }
        K::ConcurrentLaneEnd { result: None, .. } => Ok(Fact::assigned(Fact::NULL)),
        _ => Err(failure(node.id, None, UnsupportedSemantics)),
    }
}

pub(super) fn join_value(owner: &CompiledNode, successful: Option<Fact>) -> FlowResult<Fact> {
    match owner.kind {
        K::ForEachConcurrentStart { .. } | K::TogetherConcurrentStart { .. } => {
            Ok(Fact::assigned(Fact::LIST))
        }
        K::FirstSuccessStart { .. } => Ok(successful.map_or(Fact::UNSET, core::convert::identity)),
        _ => Err(failure(owner.id, None, UnsupportedSemantics)),
    }
}

pub(super) fn parent_after_join(
    parent: &[Fact],
    join: &CompiledNode,
    value: Fact,
) -> FlowResult<Facts> {
    let mut copy = super::facts::copy(parent)?;
    if let Some(slot) = join.output.slot() {
        let idx = slot.as_usize();
        *copy
            .get_mut(idx)
            .ok_or_else(|| failure(join.id, Some(slot), InvalidReference))? = value;
    }
    Ok(copy)
}

pub(super) fn fork_item(
    parts: &WorkflowParts,
    owner: &CompiledNode,
    parent: &[Fact],
    checking: bool,
) -> FlowResult<Facts> {
    match owner.kind {
        K::ForEachConcurrentStart {
            input, item_slot, ..
        } => {
            let mut ctx = super::super::solve::context(parts, owner, parent, checking)?;
            ctx.typed_read(input, Fact::LIST)?;
            ctx.write(item_slot, Fact::assigned(Fact::DYNAMIC), false)?;
            Ok(ctx.state)
        }
        _ => Err(failure(owner.id, None, UnsupportedSemantics)),
    }
}
