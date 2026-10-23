#![forbid(unsafe_code)]

use crate::workflow::CompiledWorkflow;
pub(super) use crate::{
    CompiledNode, CompiledNodeKind, ConstIdx, ConstValue, ResourceContract, SlotIdx, StepIdx,
    WorkflowDigest, WorkflowParts,
};

pub(super) type TestResult<T = ()> = Result<T, String>;

#[derive(Clone, Copy)]
pub(super) enum Family {
    ForEach,
    Collect,
    Reduce,
    Repeat,
}

pub(super) const FAMILIES: [Family; 4] = [
    Family::ForEach,
    Family::Collect,
    Family::Reduce,
    Family::Repeat,
];

pub(super) fn step(value: u16) -> StepIdx {
    StepIdx::new(value)
}

pub(super) fn slot(value: u16) -> SlotIdx {
    SlotIdx::new(value)
}

pub(super) fn linear_parts(count: u16) -> TestResult<WorkflowParts> {
    Ok(WorkflowParts {
        name: "nested-entry-boundaries".into(),
        digest: WorkflowDigest::from_bytes([0x38; 32]),
        semantic_source_identity: None,
        nodes: linear_nodes(count)?.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Bool(true)].into_boxed_slice(),
        slot_count: 8,
        symbols_count: 0,
        entry: step(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    })
}

fn linear_nodes(count: u16) -> TestResult<Vec<CompiledNode>> {
    let mut nodes = Vec::new();
    for id in 0..count {
        let next = id.checked_add(1).ok_or("test step overflow")?;
        let terminal = next == count;
        nodes.push(CompiledNode {
            id: step(id),
            output: None,
            next: (!terminal).then_some(step(next)),
            on_error: None,
            error_slot: None,
            kind: if terminal {
                CompiledNodeKind::Finish { result: slot(0) }
            } else {
                CompiledNodeKind::Nop
            },
        });
    }
    Ok(nodes)
}

pub(super) fn edit(parts: &mut WorkflowParts, id: u16) -> TestResult<&mut CompiledNode> {
    parts
        .nodes
        .get_mut(usize::from(id))
        .ok_or_else(|| format!("missing node {id}"))
}

pub(super) fn check_flags(parts: WorkflowParts, expected: &[u16]) -> TestResult {
    let plan = CompiledWorkflow::try_from_parts(parts.clone())
        .map_err(|error| format!("invalid test fixture: {error:?}"))?;
    let flags = super::super::derive(&parts, &plan.loop_regions)
        .map_err(|error| format!("derive flags: {error:?}"))?;
    assert_eq!(flags.len(), parts.nodes.len());
    for (index, actual) in flags.iter().copied().enumerate() {
        let index = u16::try_from(index).map_err(|error| format!("test index: {error}"))?;
        assert_eq!(actual, expected.contains(&index), "flag at step {index}");
    }
    assert_eq!(plan.to_parts(), parts);
    Ok(())
}
