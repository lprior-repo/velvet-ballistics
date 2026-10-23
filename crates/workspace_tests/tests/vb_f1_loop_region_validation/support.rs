#![forbid(unsafe_code)]
//! Cold constructor/cache regressions for vb-8eocj.3.7.
//! These public-IR cases prove topology only. Nested activation, state preflight,
//! source execution and cleanup are separate runtime acceptance obligations.

pub(crate) use vb_core::{
    ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ExprBranch,
    ExprIdx, ExprOp, ExprProgram, ResourceContract, SlotBranch, SlotIdx, StepIdx, WorkflowDigest,
    WorkflowError, WorkflowParts,
};

pub(crate) type TestResult<T = ()> = Result<T, String>;

#[derive(Clone, Copy)]
pub(crate) enum Family {
    ForEach,
    Collect,
    Reduce,
    Repeat,
}

pub(crate) const FAMILIES: [Family; 4] = [
    Family::ForEach,
    Family::Collect,
    Family::Reduce,
    Family::Repeat,
];

pub(crate) fn step(value: u16) -> StepIdx {
    StepIdx::new(value)
}

pub(crate) fn slot(value: u16) -> SlotIdx {
    SlotIdx::new(value)
}

pub(crate) fn linear_parts(count: u16) -> TestResult<WorkflowParts> {
    Ok(WorkflowParts {
        name: "loop-region-validation".into(),
        digest: WorkflowDigest::from_bytes([0x37; 32]),
        semantic_source_identity: None,
        nodes: linear_nodes(count)?.into_boxed_slice(),
        expressions: vec![true_expression()?].into_boxed_slice(),
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

fn true_expression() -> TestResult<ExprProgram> {
    ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice())
        .map_err(|error| format!("test expression: {error:?}"))
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

pub(crate) fn edit(parts: &mut WorkflowParts, id: u16) -> TestResult<&mut CompiledNode> {
    parts
        .nodes
        .get_mut(usize::from(id))
        .ok_or_else(|| format!("missing test node {id}"))
}

pub(crate) fn accept(parts: WorkflowParts) -> TestResult<CompiledWorkflow> {
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|error| format!("unexpected rejection: {error:?}"))
}

pub(crate) fn reject(parts: WorkflowParts, expected: WorkflowError) -> TestResult {
    match CompiledWorkflow::try_from_parts(parts) {
        Err(actual) => assert_eq!(actual, expected),
        Ok(_) => return Err(format!("accepted invalid region, expected {expected:?}")),
    }
    Ok(())
}

pub(crate) fn crossing(control: u16, from: u16, to: u16) -> WorkflowError {
    WorkflowError::LoopRegionCrossing {
        control: step(control),
        from: step(from),
        to: step(to),
    }
}

pub(crate) fn unpaired(control: u16) -> WorkflowError {
    WorkflowError::LoopRegionUnpaired {
        control: step(control),
    }
}

pub(crate) fn assert_region(
    plan: &CompiledWorkflow,
    start: u16,
    body: u16,
    control: u16,
    done: u16,
) -> TestResult {
    let region = plan
        .loop_region(step(control))
        .ok_or("missing validated region")?;
    assert_eq!(
        (
            region.start(),
            region.body(),
            region.control(),
            region.done()
        ),
        (step(start), step(body), step(control), step(done))
    );
    Ok(())
}

pub(crate) fn choose(first: u16, otherwise: u16, expressions: bool) -> CompiledNodeKind {
    if expressions {
        CompiledNodeKind::Choose {
            branches: vec![ExprBranch {
                condition: ExprIdx::new(0),
                target: step(first),
            }]
            .into_boxed_slice(),
            otherwise: Some(step(otherwise)),
        }
    } else {
        CompiledNodeKind::ChooseSlot {
            branches: vec![SlotBranch {
                condition: slot(0),
                target: step(first),
            }]
            .into_boxed_slice(),
            otherwise: Some(step(otherwise)),
        }
    }
}

#[path = "builders.rs"]
mod builders;
pub(crate) use builders::*;
