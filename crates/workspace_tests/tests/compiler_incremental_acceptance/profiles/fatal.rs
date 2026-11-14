use super::*;
#[test]
fn checked_first_success_treats_iteration_limit_as_fatal() -> Result<(), String> {
    let n = |id: u16, kind: K, output: Option<u16>, next: Option<u16>| {
        node(
            StepIdx::new(id),
            kind,
            output.map(SlotIdx::new),
            next.map(StepIdx::new),
        )
    };
    let workflow = plan(
        vec![
            n(
                0,
                K::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(0),
                Some(1),
            ),
            n(
                1,
                K::SetConst {
                    value: ConstIdx::new(1),
                },
                Some(1),
                Some(2),
            ),
            n(
                2,
                K::BuildList {
                    items: Box::from([SlotIdx::new(0), SlotIdx::new(1)]),
                },
                Some(2),
                Some(3),
            ),
            n(
                3,
                K::FirstSuccessStart {
                    branches: Box::from([StepIdx::new(4), StepIdx::new(9)]),
                    join: StepIdx::new(11),
                },
                None,
                None,
            ),
            n(
                4,
                K::ForEachConcurrentStart {
                    input: SlotIdx::new(2),
                    item_slot: SlotIdx::new(3),
                    limit: vb_core::FanoutLimit::new(1),
                    at_once: vb_core::BranchCount::new(1),
                    body: StepIdx::new(5),
                    join: StepIdx::new(7),
                },
                None,
                None,
            ),
            n(
                5,
                K::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(4),
                Some(6),
            ),
            n(
                6,
                K::ConcurrentLaneEnd {
                    owner: StepIdx::new(4),
                    result: Some(SlotIdx::new(4)),
                },
                None,
                Some(7),
            ),
            n(
                7,
                K::ConcurrentGroupJoin {
                    owner: StepIdx::new(4),
                },
                Some(5),
                Some(8),
            ),
            n(
                8,
                K::ConcurrentLaneEnd {
                    owner: StepIdx::new(3),
                    result: Some(SlotIdx::new(5)),
                },
                None,
                Some(11),
            ),
            n(
                9,
                K::SetConst {
                    value: ConstIdx::new(1),
                },
                Some(6),
                Some(10),
            ),
            n(
                10,
                K::ConcurrentLaneEnd {
                    owner: StepIdx::new(3),
                    result: Some(SlotIdx::new(6)),
                },
                None,
                Some(11),
            ),
            n(
                11,
                K::ConcurrentGroupJoin {
                    owner: StepIdx::new(3),
                },
                Some(7),
                Some(12),
            ),
            n(
                12,
                K::Finish {
                    result: SlotIdx::new(7),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(10), ConstValue::I64(20)],
        Vec::new(),
        8,
        true,
    )?;
    let mut run = RunFrame::new(
        RunId::new(41_707),
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
    let result = drive_deterministic_full(
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
    );
    assert_eq!(
        result,
        Err(RuntimeEngineError::Core(
            CoreError::IterationLimitExceeded {
                resource: "for_each_limit",
            }
        ))
    );
    let mut replay_run = RunFrame::new(
        RunId::new(41_708),
        StepIdx::new(0),
        workflow.node_count(),
        workflow.slot_count(),
    )
    .map_err(|error| format!("replay frame: {error}"))?;
    let mut replay_store = ValueStore::new();
    let mut replay_states = ReplayCollectStates::new();
    let mut replay_pc = StepIdx::new(0);
    let mut replay_failure = None;
    for _ in 0..4 {
        let replay_node = workflow
            .node(replay_pc)
            .ok_or("missing checked replay node")?;
        let result = replay_step_with_collect(
            replay_node,
            &mut replay_run,
            &mut replay_store,
            &workflow,
            &mut replay_states,
        );
        if replay_pc == StepIdx::new(3) {
            replay_failure = Some(result);
            break;
        }
        match result.map_err(|_| "replay failed before the nested limit step".to_owned())? {
            ReplayAction::Continue(next) => replay_pc = next,
            _ => return Err("replay did not advance to FirstSuccess".to_owned()),
        }
    }
    match replay_failure {
        Some(Err(vb_core::replay::ReplayError::IterationLimitExceeded {
            resource: "for_each_limit",
        })) => Ok(()),
        Some(Err(error)) => Err(format!("wrong typed replay failure: {error:?}")),
        Some(Ok(ReplayAction::Continue(_))) => Err(
            "replay swallowed fatal iteration-limit as an ordinary FirstSuccess lane failure"
                .to_owned(),
        ),
        Some(Ok(ReplayAction::Finished)) => {
            Err("replay finished after swallowing fatal iteration-limit".to_owned())
        }
        Some(Ok(_)) => Err("replay suspended instead of propagating iteration-limit".to_owned()),
        None => Err("replay did not reach the nested iteration-limit step".to_owned()),
    }
}
