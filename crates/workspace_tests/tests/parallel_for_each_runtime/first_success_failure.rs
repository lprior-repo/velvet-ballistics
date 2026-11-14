use super::*;
use vb_core::replay::step::{ReplayCollectStates, replay_step_with_collect};

fn checked_plan(expression: bool) -> Result<(CompiledWorkflow, SlotIdx), String> {
    let plan = concurrent_ir(
        "first_success_missing_slot",
        two_lane_nodes(true),
        vec![vb_core::ConstValue::I64(10), vb_core::ConstValue::I64(20)],
        3,
    )?;
    let mut parts = plan.to_parts();
    let source = SlotIdx::new(3);
    parts.nodes.get_mut(1).ok_or("missing lane")?.kind = if expression {
        parts.expressions = Box::new([vb_core::ExprProgram {
            ops: Box::new([vb_core::ExprOp::LoadSlot(source)]),
            max_stack: 1,
        }]);
        vb_core::CompiledNodeKind::EvalExpr {
            expr: vb_core::ExprIdx::new(0),
        }
    } else {
        vb_core::CompiledNodeKind::Copy { source }
    };
    parts.slot_count = 4;
    parts.input_slots = Box::new([vb_core::workflow::CompiledInputSlot {
        slot: source,
        kind: vb_core::workflow::InputSlotKind::I64,
        constraint: vb_core::workflow::CompiledInputConstraint::required(),
    }]);
    Ok((
        CompiledWorkflow::try_from_concurrent_parts(parts).map_err(|e| e.to_string())?,
        source,
    ))
}

fn frame(plan: &CompiledWorkflow) -> Result<RunFrame, String> {
    RunFrame::new(
        RunId::new(5),
        StepIdx::ZERO,
        plan.node_count(),
        plan.slot_count(),
    )
    .map_err(|e| e.to_string())
}

fn runtime_failure(plan: &CompiledWorkflow, source: SlotIdx, quantum: u64) -> Result<(), String> {
    let mut run = frame(plan)?;
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    for _ in 0..8 {
        let result = drive_deterministic_full(
            plan,
            &mut run,
            &mut StepBudget::new(quantum),
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect,
            &vb_core::capability::CapabilitySet::empty(),
            1,
        );
        if result == Ok(RuntimeSignal::StepBudgetExhausted) {
            continue;
        }
        assert_eq!(
            result,
            Err(vb_runtime::engine::RuntimeEngineError::Core(
                vb_core::EngineError::SlotUninitialized { slot: source }
            ))
        );
        assert_eq!(
            run.step_state(StepIdx::new(3)).map_err(|e| e.to_string())?,
            vb_core::StepState::Pending
        );
        return Ok(());
    }
    Err("missing-slot failure was not returned".into())
}

#[test]
fn first_success_propagates_invalid_slot_instead_of_selecting_next_lane() -> Result<(), String> {
    for expression in [false, true] {
        let (plan, source) = checked_plan(expression)?;
        for quantum in [1, 100] {
            runtime_failure(&plan, source, quantum)?;
        }
        let mut run = frame(&plan)?;
        let node = plan.node(plan.entry()).ok_or("missing entry")?;
        let result = replay_step_with_collect(
            node,
            &mut run,
            &mut ValueStore::new(),
            &plan,
            &mut ReplayCollectStates::new(),
        );
        assert!(
            matches!(result, Err(vb_core::replay::ReplayError::SlotNotAvailable { slot }) if slot == source)
        );
    }
    Ok(())
}
