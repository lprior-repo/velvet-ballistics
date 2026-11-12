use super::super::ContinuationAuthority;
use super::{TransportFailure, operation::Operation};
use vb_core::{AccessorIdx, ConstIdx, ExprIdx, StepIdx};

/// Accounts for the existing validators' accepted WorkflowParts copies without
/// first invoking to_parts. All iteration uses existing accepted resource caps.
pub(super) fn charge(
    op: &mut Operation,
    authority: &ContinuationAuthority,
) -> Result<(), TransportFailure> {
    let workflow = &authority.workflow;
    let resource = workflow.resource_contract();
    let nodes = usize::from(workflow.node_count());
    let scans = usize::from(resource.max_expressions)
        .checked_add(usize::from(resource.max_accessors))
        .and_then(|n| n.checked_add(usize::from(resource.max_constants)))
        .ok_or(vb_core::value_store::transport::CheckpointTransportError::Overflow)?;
    op.budget.work(scans)?;
    let square = nodes
        .checked_mul(nodes)
        .ok_or(vb_core::value_store::transport::CheckpointTransportError::Overflow)?;
    let work = square
        .checked_mul(nodes)
        .and_then(|n| n.checked_mul(64))
        .ok_or(vb_core::value_store::transport::CheckpointTransportError::Overflow)?;
    // Existing accepted budget validators copy WorkflowParts and retain bounded
    // region-pair caches. Charge their copies/caches and conservative cubic work.
    op.budget.reserve(nodes, 4096)?;
    op.budget.reserve(square, 256)?;
    op.budget.work(work)?;
    op.known(workflow.name())?;
    op.known(workflow.input_slots())?;
    for raw in 0..workflow.node_count() {
        let step = StepIdx::new(raw);
        if let Some(node) = workflow.node(step) {
            op.known(node)?;
        }
        if let Some(name) = workflow.step_name(step) {
            op.known(name)?;
        }
    }
    tables(op, workflow)?;
    Ok(())
}

fn tables(
    op: &mut Operation,
    workflow: &vb_core::CompiledWorkflow,
) -> Result<(), TransportFailure> {
    let resource = workflow.resource_contract();
    for raw in 0..resource.max_expressions {
        if let Some(value) = workflow.expression(ExprIdx::new(raw)) {
            op.known(value)?;
        }
    }
    for raw in 0..resource.max_accessors {
        if let Some(value) = workflow.accessor(AccessorIdx::new(raw)) {
            op.known(value)?;
        }
    }
    for raw in 0..resource.max_constants {
        if let Some(value) = workflow.constant(ConstIdx::new(raw)) {
            op.known(value)?;
        }
    }
    Ok(())
}
