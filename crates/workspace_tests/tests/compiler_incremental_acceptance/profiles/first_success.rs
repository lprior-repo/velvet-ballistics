use super::*;
fn first_success_plan() -> Result<CompiledWorkflow, String> {
    let n = |id: u16, kind: K, output: Option<u16>, next: Option<u16>| {
        node(
            StepIdx::new(id),
            kind,
            output.map(SlotIdx::new),
            next.map(StepIdx::new),
        )
    };
    plan(
        vec![
            n(
                0,
                K::FirstSuccessStart {
                    branches: Box::from([StepIdx::new(1), StepIdx::new(3)]),
                    join: StepIdx::new(5),
                },
                None,
                None,
            ),
            n(
                1,
                K::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(1),
                Some(2),
            ),
            n(
                2,
                K::ConcurrentLaneEnd {
                    owner: StepIdx::new(0),
                    result: Some(SlotIdx::new(1)),
                },
                None,
                Some(5),
            ),
            n(
                3,
                K::SetConst {
                    value: ConstIdx::new(2),
                },
                Some(1),
                Some(4),
            ),
            n(
                4,
                K::ConcurrentLaneEnd {
                    owner: StepIdx::new(0),
                    result: Some(SlotIdx::new(1)),
                },
                None,
                Some(5),
            ),
            n(
                5,
                K::ConcurrentGroupJoin {
                    owner: StepIdx::new(0),
                },
                Some(2),
                Some(6),
            ),
            n(
                6,
                K::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(10), ConstValue::I64(0), ConstValue::I64(42)],
        vec![division()?],
        3,
        true,
    )
}

#[test]
fn checked_first_success_skips_division_failure_in_runtime_and_replay() -> Result<(), String> {
    let workflow = first_success_plan()?;
    let mut run = RunFrame::new(
        RunId::new(41_705),
        StepIdx::new(0),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|error| format!("runtime frame: {error}"))?;
    let mut store = ValueStore::new();
    let mut budget = StepBudget::new(64);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    let granted = vb_core::capability::CapabilitySet::empty();
    let runtime_result = drive_deterministic_full(
        &workflow,
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
    .map_err(|error| format!("checked FirstSuccess runtime: {error:?}"))?;
    assert_eq!(runtime_result, RuntimeSignal::Finished(SlotValue::I64(42)));
    assert_eq!(
        run.read_taint(SlotIdx::new(2))
            .map_err(|error| format!("runtime result taint: {error}"))?,
        Taint::Clean
    );

    let mut replay_run = RunFrame::new(
        RunId::new(41_705),
        StepIdx::new(0),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|error| format!("replay frame: {error}"))?;
    let mut replay_store = ValueStore::new();
    let mut replay_states = ReplayCollectStates::new();
    let mut pc = StepIdx::new(0);
    let mut replay_finished = false;
    for _ in 0..32 {
        let node = workflow
            .node(pc)
            .ok_or_else(|| format!("missing replay node {pc:?}"))?;
        match replay_step_with_collect(
            node,
            &mut replay_run,
            &mut replay_store,
            &workflow,
            &mut replay_states,
        )
        .map_err(|error| format!("checked FirstSuccess replay: {error:?}"))?
        {
            ReplayAction::Continue(next) => pc = next,
            ReplayAction::Finished => {
                replay_finished = true;
                break;
            }
            _ => return Err("unexpected FirstSuccess replay action".to_owned()),
        }
    }
    assert!(
        replay_finished,
        "checked FirstSuccess replay did not finish"
    );
    assert_eq!(
        replay_run
            .read_slot(SlotIdx::new(2))
            .map_err(|error| format!("replay result: {error}"))?,
        &SlotValue::I64(42)
    );
    assert_eq!(
        replay_run
            .read_taint(SlotIdx::new(2))
            .map_err(|error| format!("replay result taint: {error}"))?,
        Taint::Clean
    );
    Ok(())
}
