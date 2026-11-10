use super::facts::{self, invalid};
use super::graph::{self, Route};
use super::{Context, Fact, Facts, FlowResult, capacity};
use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind, WorkflowParts};
use std::collections::VecDeque;

pub(super) fn validate(parts: &WorkflowParts) -> FlowResult<()> {
    if parts.nodes.len() > crate::limits::MAX_STEPS_PER_WORKFLOW {
        return Err(capacity());
    }
    let mut budget = super::budget::Budget::new(parts)?;
    facts::constant_writers(parts)?;
    let mut incoming = Vec::new();
    incoming
        .try_reserve_exact(parts.nodes.len())
        .map_err(|_| capacity())?;
    incoming.resize_with(parts.nodes.len(), || None);
    let entry = incoming
        .get_mut(parts.entry.as_usize())
        .ok_or_else(|| invalid(parts.entry))?;
    budget.retain()?;
    *entry = Some(facts::seed(parts)?);
    if forward_only(parts)? {
        return forward(parts, &mut incoming, &mut budget);
    }
    let mut queue = VecDeque::new();
    queue
        .try_reserve_exact(parts.nodes.len())
        .map_err(|_| capacity())?;
    queue.push_back(parts.entry);
    let mut queued = Vec::new();
    queued
        .try_reserve_exact(parts.nodes.len())
        .map_err(|_| capacity())?;
    queued.resize(parts.nodes.len(), false);
    *queued
        .get_mut(parts.entry.as_usize())
        .ok_or_else(|| invalid(parts.entry))? = true;

    // Nine kind bits and one absence bit can only grow at each incoming slot.
    let height = usize::from(parts.slot_count)
        .checked_mul(10)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(capacity)?;
    let bound = parts
        .nodes
        .len()
        .checked_mul(height)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(capacity)?;
    for _ in 0..bound {
        let Some(step) = queue.pop_front() else {
            return check(parts, &incoming, &mut budget);
        };
        *queued
            .get_mut(step.as_usize())
            .ok_or_else(|| invalid(step))? = false;
        let node = parts
            .nodes
            .get(step.as_usize())
            .ok_or_else(|| invalid(step))?;
        let state = incoming
            .get(step.as_usize())
            .and_then(Option::as_ref)
            .ok_or_else(|| invalid(step))?;
        budget.transfer(parts, node)?;
        let transitions = evaluate(parts, node, state, false)?;
        for (target, facts) in transitions {
            let dest = incoming
                .get_mut(target.as_usize())
                .ok_or_else(|| invalid(target))?;
            let changed = if let Some(prior) = dest {
                facts::merge(prior, &facts)
            } else {
                budget.retain()?;
                *dest = Some(facts);
                true
            };
            let scheduled = queued
                .get_mut(target.as_usize())
                .ok_or_else(|| invalid(target))?;
            if changed && !*scheduled {
                queue.push_back(target);
                *scheduled = true;
            }
        }
    }
    Err(capacity())
}

// Numeric order is topological only when every execution and failure edge
// points forward. A back edge keeps the fixed-point solver and final checks.
fn forward_only(parts: &WorkflowParts) -> FlowResult<bool> {
    for (index, node) in parts.nodes.iter().enumerate() {
        if node
            .routing
            .error
            .handler()
            .is_some_and(|target| target.as_usize() <= index)
            || graph::successors(node)?
                .iter()
                .any(|edge| edge.target.as_usize() <= index)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn forward(
    parts: &WorkflowParts,
    incoming: &mut [Option<Facts>],
    budget: &mut super::budget::Budget,
) -> FlowResult<()> {
    for (entry, node) in parts.nodes.iter().enumerate() {
        let Some(state) = incoming.get_mut(entry).and_then(Option::take) else {
            continue;
        };
        budget.release()?;
        budget.transfer(parts, node)?;
        // All predecessors have contributed; check before releasing this state.
        for (target, facts) in evaluate(parts, node, &state, true)? {
            let dest = incoming
                .get_mut(target.as_usize())
                .ok_or_else(|| invalid(target))?;
            if let Some(prior) = dest {
                facts::merge(prior, &facts);
            } else {
                budget.retain()?;
                *dest = Some(facts);
            }
        }
    }
    Ok(())
}

fn check(
    parts: &WorkflowParts,
    incoming: &[Option<Facts>],
    budget: &mut super::budget::Budget,
) -> FlowResult<()> {
    for (node, state) in parts.nodes.iter().zip(incoming) {
        if let Some(state) = state {
            budget.transfer(parts, node)?;
            evaluate(parts, node, state, true)?;
        }
    }
    Ok(())
}

fn evaluate(
    parts: &WorkflowParts,
    node: &CompiledNode,
    input: &[Fact],
    checking: bool,
) -> FlowResult<Vec<(StepIdx, Facts)>> {
    let edges = graph::successors(node)?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(edges.len().checked_add(1).ok_or_else(capacity)?)
        .map_err(|_| capacity())?;
    let mut partial = facts::copy(input)?;
    if edges.is_empty() {
        let mut ctx = context(parts, node, input, checking)?;
        super::transfer::apply(&mut ctx, Route::Next)?;
        super::evidence::check(&ctx, Route::Next)?;
        facts::merge(&mut partial, &ctx.partial);
    }
    for edge in edges {
        let mut ctx = context(parts, node, input, checking)?;
        super::transfer::apply(&mut ctx, edge.route)?;
        super::evidence::check(&ctx, edge.route)?;
        facts::merge(&mut partial, &ctx.partial);
        result.push((edge.target, ctx.state));
    }
    if let Some(handler) = node.routing.error.handler() {
        let error_slot = node.routing.error.error_slot().or_else(|| {
            parts
                .nodes
                .iter()
                .find(|candidate| {
                    matches!(candidate.kind, CompiledNodeKind::ErrorHandler { body, .. } if body == node.id)
                })
                .and_then(|wrapper| wrapper.routing.error.error_slot())
        });
        if let Some(slot) = error_slot {
            *partial
                .get_mut(slot.as_usize())
                .ok_or_else(|| invalid(node.id))? = Fact::assigned(Fact::I64);
        }
        result.push((handler, partial));
    }
    Ok(result)
}

fn context<'a>(
    parts: &'a WorkflowParts,
    node: &'a CompiledNode,
    input: &[Fact],
    checking: bool,
) -> FlowResult<Context<'a>> {
    Ok(Context {
        parts,
        node,
        state: facts::copy(input)?,
        partial: facts::copy(input)?,
        checking,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ConstIdx, SlotIdx, WorkflowDigest};
    use crate::value::ConstValue;
    use crate::workflow::{ResourceContract, SlotFlowFailure, node};

    #[test]
    fn default_contract_dense_chain_checks_the_last_read() -> FlowResult<()> {
        let contract = ResourceContract::DEFAULT;
        let last = contract.max_steps.checked_sub(1).ok_or_else(capacity)?;
        let mut nodes = Vec::new();
        for id in 0..last {
            nodes.push(node(
                StepIdx::new(id),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(id.checked_add(1).ok_or_else(capacity)?)),
            ));
        }
        nodes.push(node(
            StepIdx::new(last),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ));
        let mut parts = WorkflowParts {
            name: "dense-chain".into(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            semantic_source_identity: None,
            source_metadata: None,
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([ConstValue::I64(1)]),
            slot_count: contract.max_slots,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: contract,
            step_names: Box::new([]),
            input_slots: Box::new([]),
        };
        validate(&parts)?;
        parts.nodes.last_mut().ok_or_else(capacity)?.kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        };
        assert_eq!(
            validate(&parts),
            Err(super::super::failure(
                StepIdx::new(last),
                Some(SlotIdx::new(1)),
                SlotFlowFailure::UninitializedRead
            ))
        );
        Ok(())
    }
}
