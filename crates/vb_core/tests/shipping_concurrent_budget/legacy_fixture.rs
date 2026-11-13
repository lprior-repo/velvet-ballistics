use super::K;
use super::fixtures::nested;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, NodeOutput, WorkflowError};

fn set_start(
    nodes: &mut [CompiledNode],
    idx: usize,
    body: u16,
    done: u16,
    limit: u32,
) -> Result<(), WorkflowError> {
    let node = nodes.get_mut(idx).ok_or(WorkflowError::EmptyNodes)?;
    node.kind = K::ForEachStart {
        input: SlotIdx::new(0),
        item_slot: SlotIdx::new(1),
        limit: limit.into(),
        body: StepIdx::new(body),
        done: StepIdx::new(done),
    };
    node.output = NodeOutput::Slot(SlotIdx::new(3));
    Ok(())
}

fn set_next(
    nodes: &mut [CompiledNode],
    idx: usize,
    body: u16,
    done: u16,
) -> Result<(), WorkflowError> {
    let node = nodes.get_mut(idx).ok_or(WorkflowError::EmptyNodes)?;
    node.kind = K::ForEachNext {
        iterator_slot: SlotIdx::new(3),
        body: StepIdx::new(body),
        done: StepIdx::new(done),
    };
    node.routing.next = None.into();
    Ok(())
}

fn set_join(nodes: &mut [CompiledNode], idx: usize) -> Result<(), WorkflowError> {
    let node = nodes.get_mut(idx).ok_or(WorkflowError::EmptyNodes)?;
    node.kind = K::ForEachJoin {
        output: SlotIdx::new(4),
    };
    Ok(())
}

pub(super) fn outer_legacy() -> Result<Vec<CompiledNode>, WorkflowError> {
    let mut nodes = nested(3, 2, 5, 4);
    set_start(&mut nodes, 0, 1, 6, 3)?;
    set_next(&mut nodes, 5, 1, 6)?;
    set_join(&mut nodes, 6)?;
    Ok(nodes)
}

pub(super) fn inner_legacy() -> Result<Vec<CompiledNode>, WorkflowError> {
    let mut nodes = nested(3, 2, 5, 4);
    set_start(&mut nodes, 1, 2, 4, 5)?;
    set_next(&mut nodes, 3, 2, 4)?;
    set_join(&mut nodes, 4)?;
    Ok(nodes)
}
