//! Constructor-level compiled-IR slot-flow regressions.
mod slot_flow_support;
use slot_flow_support::*;
use vb_core::workflow::{
    CompiledNodeKind as K, CompiledWorkflow, SlotFlowError, SlotFlowFailure, WorkflowError,
    WorkflowParts,
};

#[path = "slot_flow_support/action_execution.rs"]
mod action_execution;
#[path = "slot_flow_support/capacity.rs"]
mod capacity;

#[path = "slot_flow_support/ask.rs"]
mod ask;
#[cfg(feature = "test-util")]
#[path = "slot_flow_support/concrete_oracle.rs"]
mod concrete_oracle;
#[path = "slot_flow_support/errors.rs"]
mod errors;
#[path = "slot_flow_support/expressions.rs"]
mod expressions;
#[path = "slot_flow_support/loops.rs"]
mod loops;
#[path = "slot_flow_support/numeric.rs"]
mod numeric;
#[path = "slot_flow_support/together.rs"]
mod together;

#[test]
fn undeclared_external_slot_is_not_guessed_as_input() -> TestResult {
    let parts = parts(vec![finish(0, 0)], 1);
    assert_constructor_error(
        parts,
        step(0),
        Some(slot(0)),
        SlotFlowFailure::UninitializedRead,
    )
}

#[test]
fn required_declared_input_is_available() -> TestResult {
    let mut p = parts(vec![finish(0, 0)], 1);
    declare(&mut p, 0, vb_core::workflow::InputSlotKind::I64);
    CompiledWorkflow::try_from_parts(p)?;
    Ok(())
}

#[test]
fn optional_input_is_not_definitely_assigned() -> TestResult {
    let mut p = parts(vec![finish(0, 0)], 1);
    declare(&mut p, 0, vb_core::workflow::InputSlotKind::I64);
    if let Some(input) = p.input_slots.first_mut() {
        input.constraint.optional = true;
    }
    assert_constructor_error(
        p,
        step(0),
        Some(slot(0)),
        SlotFlowFailure::UninitializedRead,
    )
}

#[test]
fn branch_only_write_does_not_initialize_join() -> TestResult {
    let mut p = parts(
        vec![
            choose(0, 1, 2),
            set(1, 0, 1, 3),
            node(2, K::Nop, None, Some(3)),
            finish(3, 1),
        ],
        2,
    );
    declare(&mut p, 0, vb_core::workflow::InputSlotKind::Bool);
    assert_constructor_error(
        p,
        step(3),
        Some(slot(1)),
        SlotFlowFailure::UninitializedRead,
    )
}

#[test]
fn both_branch_writes_initialize_join() -> TestResult {
    let mut p = parts(
        vec![
            choose(0, 1, 2),
            set(1, 0, 1, 3),
            set(2, 0, 1, 3),
            finish(3, 1),
        ],
        2,
    );
    declare(&mut p, 0, vb_core::workflow::InputSlotKind::Bool);
    CompiledWorkflow::try_from_parts(p)?;
    Ok(())
}

#[test]
fn wrong_boolean_operand_is_rejected() -> TestResult {
    use vb_core::workflow::{ExprOp, ExprProgram};
    let mut p = parts(
        vec![
            set(0, 0, 0, 1),
            node(
                1,
                K::EvalExpr {
                    expr: vb_core::ids::ExprIdx::new(0),
                },
                Some(1),
                Some(2),
            ),
            finish(2, 1),
        ],
        2,
    );
    p.expressions = vec![ExprProgram::try_from_ops(
        vec![ExprOp::LoadSlot(slot(0)), ExprOp::Not].into_boxed_slice(),
    )?]
    .into_boxed_slice();
    assert_constructor_error(p, step(1), None, SlotFlowFailure::IncompatibleType)
}

fn assert_constructor_error(
    p: WorkflowParts,
    expected_step: vb_core::StepIdx,
    expected_slot: Option<vb_core::SlotIdx>,
    expected_reason: SlotFlowFailure,
) -> TestResult {
    let Err(WorkflowError::SlotFlow(SlotFlowError { step, slot, reason })) =
        CompiledWorkflow::try_from_parts(p)
    else {
        return Err("expected typed constructor slot-flow failure".into());
    };
    assert_eq!(step, expected_step);
    assert_eq!(slot, expected_slot);
    assert_eq!(reason, expected_reason);
    Ok(())
}
