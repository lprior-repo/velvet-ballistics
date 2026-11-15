use vb_core::errors::CoreError;
use vb_core::workflow::{SlotFlowError, SlotFlowFailure, node};
use vb_core::{
    AccessorIdx, CompiledNodeKind, CompiledWorkflow, ConstIdx, ExprIdx, ExprOp, ExprProgram,
    SlotIdx, StepIdx, WorkflowError, WorkflowParts,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const SOURCE: &str = "version: velvet-ballistics/v1\nname: expression_bounds\nwhen: {manual: {}}\nsteps:\n  - id: seeded\n    set: {output: saved, value: '1'}\n  - id: done\n    finish: {result: saved}\n";

fn parts() -> Result<WorkflowParts, Box<dyn std::error::Error>> {
    let parts = vb_compile::compile_workflow(SOURCE.as_bytes())?.to_parts();
    assert_eq!(
        (
            parts.slot_count,
            parts.constants.len(),
            parts.expressions.len()
        ),
        (1, 1, 0)
    );
    Ok(parts)
}

fn construct(parts: WorkflowParts, concurrent: bool) -> Result<CompiledWorkflow, WorkflowError> {
    if concurrent {
        CompiledWorkflow::try_from_concurrent_parts(parts)
    } else {
        CompiledWorkflow::try_from_parts(parts)
    }
}

fn accepted(parts: &WorkflowParts) -> TestResult {
    for concurrent in [false, true] {
        assert_eq!(construct(parts.clone(), concurrent)?.to_parts(), *parts);
    }
    Ok(())
}

fn refused(parts: &WorkflowParts, expected: WorkflowError) {
    for concurrent in [false, true] {
        assert_eq!(
            construct(parts.clone(), concurrent).err(),
            Some(expected.clone())
        );
    }
}

fn with_program(mut parts: WorkflowParts, op: ExprOp) -> Result<WorkflowParts, CoreError> {
    parts.expressions =
        vec![ExprProgram::try_from_ops(vec![op].into_boxed_slice())?].into_boxed_slice();
    Ok(parts)
}

#[test]
fn unused_expression_pool_operands_require_in_range_indices() -> TestResult {
    let base = parts()?;
    accepted(&base)?;
    accepted(&with_program(
        base.clone(),
        ExprOp::LoadSlot(SlotIdx::ZERO),
    )?)?;
    accepted(&with_program(
        base.clone(),
        ExprOp::LoadConst(ConstIdx::new(0)),
    )?)?;
    for index in [1, u16::MAX] {
        let slot = SlotIdx::new(index);
        refused(
            &with_program(base.clone(), ExprOp::LoadSlot(slot))?,
            WorkflowError::SlotOutOfBounds { slot },
        );
        let constant = ConstIdx::new(index);
        refused(
            &with_program(base.clone(), ExprOp::LoadConst(constant))?,
            WorkflowError::ConstOutOfBounds { constant },
        );
    }
    Ok(())
}

#[test]
fn empty_pools_do_not_admit_unused_zero_operands() -> TestResult {
    let mut base = parts()?;
    base.nodes = vec![node(StepIdx::ZERO, CompiledNodeKind::Nop, None, None)].into_boxed_slice();
    base.slot_count = 0;
    base.constants = Box::new([]);
    base.step_names = Box::new([]);
    base.source_metadata = None;
    base.semantic_source_identity = None;
    base.input_slots = Box::new([]);
    accepted(&base)?;
    refused(
        &with_program(base.clone(), ExprOp::LoadSlot(SlotIdx::ZERO))?,
        WorkflowError::SlotOutOfBounds {
            slot: SlotIdx::ZERO,
        },
    );
    refused(
        &with_program(base, ExprOp::LoadConst(ConstIdx::new(0)))?,
        WorkflowError::ConstOutOfBounds {
            constant: ConstIdx::new(0),
        },
    );
    Ok(())
}

#[test]
fn every_unused_program_and_operand_is_checked() -> TestResult {
    let base = parts()?;
    let valid =
        ExprProgram::try_from_ops(vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice())?;
    let valid_pair = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(SlotIdx::ZERO),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Eq,
        ]
        .into_boxed_slice(),
    )?;
    let mut control = base.clone();
    control.expressions = vec![valid.clone(), valid_pair].into_boxed_slice();
    accepted(&control)?;
    for (operand, expected) in [
        (
            ExprOp::LoadSlot(SlotIdx::new(u16::MAX)),
            WorkflowError::SlotOutOfBounds {
                slot: SlotIdx::new(u16::MAX),
            },
        ),
        (
            ExprOp::LoadConst(ConstIdx::new(u16::MAX)),
            WorkflowError::ConstOutOfBounds {
                constant: ConstIdx::new(u16::MAX),
            },
        ),
    ] {
        let mut later_program = base.clone();
        later_program.expressions = vec![
            valid.clone(),
            ExprProgram::try_from_ops(vec![operand].into_boxed_slice())?,
        ]
        .into_boxed_slice();
        refused(&later_program, expected.clone());
        let mut later_operand = base.clone();
        later_operand.expressions = vec![ExprProgram::try_from_ops(
            vec![ExprOp::LoadConst(ConstIdx::new(0)), operand, ExprOp::Eq].into_boxed_slice(),
        )?]
        .into_boxed_slice();
        refused(&later_operand, expected);
    }
    Ok(())
}

#[test]
fn existing_stack_accessor_and_active_flow_errors_keep_precedence() -> TestResult {
    let invalid = ExprProgram::try_from_ops(
        vec![ExprOp::LoadSlot(SlotIdx::new(u16::MAX))].into_boxed_slice(),
    )?;
    let mut stack = parts()?;
    stack.expressions = vec![
        invalid.clone(),
        ExprProgram {
            ops: vec![ExprOp::Eq].into_boxed_slice(),
            max_stack: 1,
        },
    ]
    .into_boxed_slice();
    refused(
        &stack,
        WorkflowError::Expression(CoreError::ExpressionStackUnderflow),
    );
    let mut accessor = parts()?;
    accessor.expressions = vec![
        invalid,
        ExprProgram::try_from_ops(
            vec![ExprOp::LoadAccessor(AccessorIdx::new(0))].into_boxed_slice(),
        )?,
    ]
    .into_boxed_slice();
    refused(
        &accessor,
        WorkflowError::Expression(CoreError::InvalidCompiledWorkflow {
            reason: "accessor index out of bounds",
        }),
    );
    let mut active = with_program(parts()?, ExprOp::LoadConst(ConstIdx::new(u16::MAX)))?;
    active.nodes.get_mut(0).ok_or("entry")?.kind = CompiledNodeKind::EvalExpr {
        expr: ExprIdx::new(0),
    };
    refused(
        &active,
        WorkflowError::SlotFlow(SlotFlowError {
            step: StepIdx::ZERO,
            slot: None,
            reason: SlotFlowFailure::InvalidReference,
        }),
    );
    Ok(())
}
