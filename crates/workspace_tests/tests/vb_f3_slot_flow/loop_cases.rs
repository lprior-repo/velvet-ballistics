use super::*;
use vb_core::ids::ExprIdx;
use vb_core::workflow::{
    ExprOp, ExprProgram, SlotFlowError, SlotFlowFailure, WorkflowError, validate_slot_flow,
};

fn incrementing_reduction() -> Result<WorkflowParts, Box<dyn std::error::Error>> {
    let mut p = reduction();
    p.expressions = vec![ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadSlot(slot(2)),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Add,
        ]
        .into_boxed_slice(),
    )?]
    .into_boxed_slice();
    if let Some(first) = p.nodes.get_mut(1) {
        first.kind = K::EvalExpr {
            expr: ExprIdx::new(0),
        };
    }
    Ok(p)
}

fn assert_public_flow_failure(p: WorkflowParts, expected: SlotFlowError) -> TestResult {
    let Err(error) = validate_slot_flow(&p) else {
        return Err("invalid loop flow was accepted".into());
    };
    assert_eq!(error.reason, expected.reason);
    assert_eq!(error.step, expected.step);
    assert_eq!(error.slot, expected.slot);
    assert_cycle_error(
        shared::validate_gate_13_no_slot_cycles(&p),
        1,
        "slot 1 -> slot 0",
    )?;
    assert_validation_flow_error(
        shared::validate_gate_14_slot_type_consistency(&p),
        &expected,
    )?;
    assert_constructor_flow_error(p, &expected)
}

pub(super) fn assert_cycle_error(
    result: Result<(), vb_validate::ValidationError>,
    expected_slot: usize,
    expected_chain: &str,
) -> TestResult {
    let Err(vb_validate::ValidationError::SlotDependencyCycle { slot, chain }) = result else {
        return Err("expected typed slot dependency cycle".into());
    };
    assert_eq!(slot, expected_slot);
    assert_eq!(chain, expected_chain);
    Ok(())
}

pub(super) fn assert_validation_flow_error(
    result: Result<(), vb_validate::ValidationError>,
    error: &SlotFlowError,
) -> TestResult {
    let Err(vb_validate::ValidationError::TypeMismatch { expected, found }) = result else {
        return Err("expected typed validation slot-flow failure".into());
    };
    assert_eq!(expected, "valid compiled slot flow");
    assert_eq!(found, error.to_string());
    Ok(())
}

pub(super) fn assert_constructor_flow_error(
    p: WorkflowParts,
    expected: &SlotFlowError,
) -> TestResult {
    let Err(WorkflowError::SlotFlow(error)) = CompiledWorkflow::try_from_parts(p) else {
        return Err("expected typed constructor slot-flow failure".into());
    };
    assert_eq!(error.reason, expected.reason);
    assert_eq!(error.step, expected.step);
    assert_eq!(error.slot, expected.slot);
    Ok(())
}

#[test]
fn reduce_feedback_preserves_known_numeric_type_and_executes_all_iterations() -> TestResult {
    use vb_core::capability::CapabilitySet;
    use vb_core::{RunFrame, StepBudget, ValueStore};
    use vb_runtime::engine::{
        EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full,
    };
    use vb_runtime::primitives::collect::CollectStates;

    let p = incrementing_reduction()?;
    shared::validate(&p)?;
    let plan = CompiledWorkflow::try_from_parts(p)?;
    for (items, expected) in [
        (vec![], 1),
        (vec![SlotValue::I64(3)], 2),
        (vec![SlotValue::I64(3), SlotValue::I64(7)], 3),
    ] {
        let mut run = RunFrame::new(
            RunId::new(1),
            plan.entry(),
            plan.node_count(),
            plan.slot_count(),
        )?;
        let mut store = ValueStore::new();
        let input = store.insert_list(items.into_boxed_slice())?;
        run.write_slot(slot(0), SlotValue::List(input))?;
        assert_eq!(
            drive_deterministic_full(
                &plan,
                &mut run,
                &mut StepBudget::new(64),
                &mut store,
                &[],
                RetryPolicy::NEVER,
                &mut EvidenceCollector::new(),
                &mut CollectStates::new(),
                &CapabilitySet::empty(),
                1,
            )?,
            RuntimeSignal::Finished(SlotValue::I64(expected))
        );
    }
    Ok(())
}

#[test]
fn initialized_reduce_cycle_with_bad_carried_type_is_rejected() -> TestResult {
    let mut p = incrementing_reduction()?;
    p.constants = vec![ConstValue::I64(1), ConstValue::Bool(true)].into_boxed_slice();
    if let Some(second) = p.nodes.get_mut(2) {
        second.kind = K::SetConst {
            value: ConstIdx::new(1),
        };
    }
    assert_public_flow_failure(
        p,
        SlotFlowError {
            step: step(1),
            slot: None,
            reason: SlotFlowFailure::IncompatibleType,
        },
    )
}

#[test]
fn initialized_accumulator_does_not_initialize_other_body_reads() -> TestResult {
    let mut p = reduction();
    if let Some(first) = p.nodes.get_mut(1) {
        first.kind = K::Copy { source: slot(3) };
    }
    assert_public_flow_failure(
        p,
        SlotFlowError {
            step: step(1),
            slot: Some(slot(3)),
            reason: SlotFlowFailure::UninitializedRead,
        },
    )
}

#[test]
fn empty_reduce_exit_does_not_initialize_iterator_output() -> TestResult {
    let mut p = reduction();
    if let Some(last) = p.nodes.last_mut() {
        last.kind = K::Finish { result: slot(1) };
    }
    assert_public_flow_failure(
        p,
        SlotFlowError {
            step: step(6),
            slot: Some(slot(1)),
            reason: SlotFlowFailure::UninitializedRead,
        },
    )
}
