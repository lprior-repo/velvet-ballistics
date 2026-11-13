// C1 physical frames isolate parent captures and sibling constant writers.
use super::super::{budget::Budget, facts};
use super::layout::Layout;
use super::store::Store;
use super::{Fact, FlowResult, SlotFlowFailure, failure};
use crate::ids::SlotIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind as K, WorkflowParts};

pub(super) fn validate(
    parts: &WorkflowParts,
    layout: &Layout<'_>,
    store: &Store,
    budget: &mut Budget,
) -> FlowResult<()> {
    let slot_count = usize::from(parts.slot_count);
    for frame in 0..layout.scopes.len() {
        budget.scratch(1)?;
        budget.charge(slot_count)?;
        let mut writers = super::slots(slot_count, Fact::UNSET)?;
        for step in store.steps(frame)? {
            budget.charge(1)?;
            let node = parts
                .nodes
                .get(step.as_usize())
                .ok_or_else(|| failure(step, None, SlotFlowFailure::InvalidReference))?;
            record(parts, node, &mut writers)?;
        }
    }
    Ok(())
}

fn record(parts: &WorkflowParts, node: &CompiledNode, writers: &mut [Fact]) -> FlowResult<()> {
    let K::SetConst { value } = node.kind else {
        return Ok(());
    };
    let Some(constant) = parts.constants.get(value.as_usize()) else {
        return Ok(());
    };
    let Some(slot) = node.output.slot() else {
        return Ok(());
    };
    let dest = writers
        .get_mut(slot.as_usize())
        .ok_or_else(|| failure(node.id, Some(slot), SlotFlowFailure::InvalidReference))?;
    let value_fact = Fact::constant(constant);
    assign(dest, value_fact, node, slot)
}

fn assign(dest: &mut Fact, value: Fact, node: &CompiledNode, slot: SlotIdx) -> FlowResult<()> {
    if facts::constant_conflict(*dest, value) {
        return Err(failure(
            node.id,
            Some(slot),
            SlotFlowFailure::ConflictingConstantWrites,
        ));
    }
    *dest = value;
    Ok(())
}
