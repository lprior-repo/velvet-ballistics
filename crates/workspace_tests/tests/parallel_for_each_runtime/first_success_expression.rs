use super::*;

fn replay_first_success(plan: &CompiledWorkflow) -> Result<SlotValue, String> {
    use vb_core::replay::step::{ReplayAction, ReplayCollectStates, replay_step_with_collect};

    let mut run = RunFrame::new(
        RunId::new(4),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )
    .map_err(|error| format!("{error}"))?;
    let mut store = ValueStore::new();
    let mut states = ReplayCollectStates::new();
    let mut pc = StepIdx::new(0);
    for _ in 0..100 {
        let node = plan
            .node(pc)
            .ok_or_else(|| format!("missing node {pc:?}"))?;
        match replay_step_with_collect(node, &mut run, &mut store, plan, &mut states)
            .map_err(|error| format!("{error:?}"))?
        {
            ReplayAction::Continue(next) => pc = next,
            ReplayAction::Finished => {
                return run
                    .read_slot(SlotIdx::new(2))
                    .cloned()
                    .map_err(|error| format!("{error}"));
            }
            ReplayAction::Suspended { .. } => return Err("unexpected replay suspension".into()),
            _ => return Err("unexpected replay action".into()),
        }
    }
    Err("replay exceeded 100 steps".into())
}

#[test]
fn first_success_advances_after_real_expression_division_by_zero() -> Result<(), String> {
    let mut nodes = two_lane_nodes(true);
    let expr_node = vb_core::workflow::node(
        StepIdx::new(1),
        vb_core::CompiledNodeKind::EvalExpr {
            expr: vb_core::ExprIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    );
    *nodes
        .get_mut(1)
        .ok_or("missing first-success expression node")? = expr_node;
    let plan = concurrent_ir_with_expressions(
        "first_success_division_by_zero",
        nodes,
        vec![
            vb_core::ConstValue::I64(10),
            vb_core::ConstValue::I64(20),
            vb_core::ConstValue::I64(0),
        ],
        vec![vb_core::ExprProgram {
            ops: Box::from([
                vb_core::ExprOp::LoadConst(vb_core::ConstIdx::new(0)),
                vb_core::ExprOp::LoadConst(vb_core::ConstIdx::new(2)),
                vb_core::ExprOp::Div,
            ]),
            max_stack: 2,
        }],
        3,
    )?;

    let (signal, _) = drive_ir(&plan)?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(20)));
    assert_eq!(replay_first_success(&plan)?, SlotValue::I64(20));
    Ok(())
}

#[test]
fn first_success_budget_one_routes_real_expression_division_by_zero() -> Result<(), String> {
    let mut nodes = two_lane_nodes(true);
    let expr_node = vb_core::workflow::node(
        StepIdx::new(1),
        vb_core::CompiledNodeKind::EvalExpr {
            expr: vb_core::ExprIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    );
    *nodes
        .get_mut(1)
        .ok_or("missing first-success expression node")? = expr_node;
    let plan = concurrent_ir_with_expressions(
        "first_success_division_by_zero_budget_one",
        nodes,
        vec![
            vb_core::ConstValue::I64(10),
            vb_core::ConstValue::I64(20),
            vb_core::ConstValue::I64(0),
        ],
        vec![vb_core::ExprProgram {
            ops: Box::from([
                vb_core::ExprOp::LoadConst(vb_core::ConstIdx::new(0)),
                vb_core::ExprOp::LoadConst(vb_core::ConstIdx::new(2)),
                vb_core::ExprOp::Div,
            ]),
            max_stack: 2,
        }],
        3,
    )?;
    let mut run = RunFrame::new(
        RunId::new(3),
        StepIdx::new(0),
        plan.node_count(),
        plan.slot_count(),
    )
    .map_err(|error| format!("{error}"))?;
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    let granted = vb_core::capability::CapabilitySet::empty();
    for _ in 0..100 {
        let mut budget = vb_core::engine::StepBudget::new(1);
        let signal = drive_deterministic_full(
            &plan,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &granted,
            1,
        )
        .map_err(|error| format!("{error:?}"))?;
        if signal == RuntimeSignal::StepBudgetExhausted {
            continue;
        }
        assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(20)));
        assert_eq!(replay_first_success(&plan)?, SlotValue::I64(20));
        return Ok(());
    }
    Err("FirstSuccess did not finish within 100 budget-one drives".into())
}

#[path = "first_success_failure.rs"]
mod failure;

#[path = "first_success_choose.rs"]
mod choose;
