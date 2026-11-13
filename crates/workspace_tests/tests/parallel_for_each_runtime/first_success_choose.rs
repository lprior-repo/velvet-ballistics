use super::*;
use vb_core::replay::ReplayError;
use vb_core::replay::step::{ReplayCollectStates, replay_step_with_collect};

fn checked_plan(missing_input: bool) -> Result<CompiledWorkflow, String> {
    use vb_core::workflow::{CompiledNodeKind as K, node};
    use vb_core::{ConstIdx, ExprIdx, ExprOp as Op};
    let mut nodes = two_lane_nodes(true);
    *nodes.get_mut(1).ok_or("missing first lane")? = node(
        StepIdx::new(1),
        K::Choose {
            branches: Box::new([vb_core::workflow::ExprBranch {
                condition: ExprIdx::new(0),
                target: StepIdx::new(2),
            }]),
            otherwise: Some(StepIdx::new(2)).into(),
        },
        None,
        None,
    );
    nodes.get_mut(2).ok_or("missing lane end")?.kind = K::ConcurrentLaneEnd {
        owner: StepIdx::ZERO,
        result: None,
    };
    let ops = if missing_input {
        vec![
            Op::LoadSlot(SlotIdx::new(3)),
            Op::LoadConst(ConstIdx::new(2)),
            Op::Eq,
        ]
    } else {
        vec![
            Op::LoadConst(ConstIdx::new(0)),
            Op::LoadConst(ConstIdx::new(2)),
            Op::Div,
            Op::LoadConst(ConstIdx::new(2)),
            Op::Eq,
        ]
    };
    // Add the required input before checked construction, including for the missing seed case.
    let base = concurrent_ir(
        "choose_failure",
        two_lane_nodes(true),
        vec![vb_core::ConstValue::I64(10), vb_core::ConstValue::I64(20)],
        3,
    )?;
    let mut parts = base.to_parts();
    parts.nodes = nodes.into_boxed_slice();
    parts.constants = Box::new([
        vb_core::ConstValue::I64(10),
        vb_core::ConstValue::I64(20),
        vb_core::ConstValue::I64(0),
    ]);
    parts.expressions = Box::new([vb_core::ExprProgram {
        ops: ops.into_boxed_slice(),
        max_stack: 2,
    }]);
    parts.slot_count = 4;
    parts.input_slots = Box::new([vb_core::workflow::CompiledInputSlot {
        slot: SlotIdx::new(3),
        kind: vb_core::workflow::InputSlotKind::I64,
        constraint: vb_core::workflow::CompiledInputConstraint::required(),
    }]);
    CompiledWorkflow::try_from_concurrent_parts(parts).map_err(|e| format!("{e:?}"))
}

fn frame(plan: &CompiledWorkflow) -> Result<RunFrame, String> {
    RunFrame::new(
        RunId::new(6),
        plan.entry(),
        plan.node_count(),
        plan.slot_count(),
    )
    .map_err(|e| e.to_string())
}

fn no_winner(run: &RunFrame, store: &ValueStore) -> Result<(), String> {
    assert_eq!(
        run.step_state(StepIdx::new(3)).map_err(|e| e.to_string())?,
        vb_core::StepState::Pending
    );
    let slot = SlotIdx::new(1);
    assert_eq!(
        run.read_slot(slot),
        Err(vb_core::EngineError::SlotUninitialized { slot })
    );
    match run.read_slot(SlotIdx::new(2)) {
        Ok(SlotValue::List(id)) => assert_eq!(store.list(*id).map_err(|e| e.to_string())?, &[]),
        Err(vb_core::EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(2) => {}
        value => return Err(format!("unexpected winner/progress: {value:?}")),
    }
    Ok(())
}

fn live_failure(plan: &CompiledWorkflow, missing: bool, quantum: u64) -> Result<(), String> {
    let mut run = frame(plan)?;
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    let expected = if missing {
        vb_core::EngineError::SlotUninitialized {
            slot: SlotIdx::new(3),
        }
    } else {
        vb_core::EngineError::DivisionByZero
    };
    let mut resumed = false;
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
            resumed = true;
            no_winner(&run, &store)?;
            continue;
        }
        assert_eq!(
            result,
            Err(vb_runtime::engine::RuntimeEngineError::Core(expected))
        );
        assert_eq!(resumed, quantum == 1);
        assert!(!evidence.drain().iter().any(|event| matches!(event,
            vb_runtime::engine::types::EvidenceEvent::StepStarted { step } if *step == StepIdx::new(3))));
        return no_winner(&run, &store);
    }
    Err("Choose did not return its failure".into())
}

fn check_failure(missing: bool) -> Result<(), String> {
    let plan = checked_plan(missing)?;
    for quantum in [1, 100] {
        live_failure(&plan, missing, quantum)?;
    }
    let mut run = frame(&plan)?;
    let mut store = ValueStore::new();
    let result = replay_step_with_collect(
        plan.node(plan.entry()).ok_or("entry")?,
        &mut run,
        &mut store,
        &plan,
        &mut ReplayCollectStates::new(),
    );
    let expected = if missing {
        ReplayError::SlotNotAvailable {
            slot: SlotIdx::new(3),
        }
    } else {
        ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1),
        }
    };
    assert_eq!(result.err(), Some(expected));
    no_winner(&run, &store)
}

#[test]
fn first_success_choose_division_by_zero_is_fatal_live_and_replay() -> Result<(), String> {
    check_failure(false)
}

#[test]
fn first_success_choose_missing_input_is_fatal_live_and_replay() -> Result<(), String> {
    check_failure(true)
}

#[path = "first_success_choose_nested.rs"]
mod nested;
