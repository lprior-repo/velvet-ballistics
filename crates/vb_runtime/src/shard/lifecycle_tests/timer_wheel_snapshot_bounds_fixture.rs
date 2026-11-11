type BoundedSnapshotTuple = (
    RunId,
    vb_core::ids::StepIdx,
    crate::shard::PendingTimerKind,
    u64,
    std::time::Instant,
);

fn bounded_snapshot_future_deadline(offset_seconds: u64) -> Result<std::time::Instant, String> {
    std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(86_400))
        .and_then(|deadline| deadline.checked_add(std::time::Duration::from_secs(offset_seconds)))
        .ok_or_else(|| String::from("snapshot fixture deadline must be representable"))
}

fn snapshot_ask_prompt_node() -> vb_core::workflow::CompiledNode {
    vb_core::workflow::node(
        vb_core::ids::StepIdx::ZERO,
        vb_core::workflow::CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(0),
        },
        Some(vb_core::ids::SlotIdx::ZERO),
        Some(vb_core::ids::StepIdx::new(1)),
    )
}

fn snapshot_ask_timeout_node() -> vb_core::workflow::CompiledNode {
    vb_core::workflow::node(
        vb_core::ids::StepIdx::new(1),
        vb_core::workflow::CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(1),
        },
        Some(vb_core::ids::SlotIdx::new(1)),
        Some(vb_core::ids::StepIdx::new(2)),
    )
}

fn snapshot_ask_node() -> vb_core::workflow::CompiledNode {
    vb_core::workflow::node(
        vb_core::ids::StepIdx::new(2),
        vb_core::workflow::CompiledNodeKind::Ask {
            prompt: vb_core::ids::SlotIdx::ZERO,
            timeout_slot: Some(vb_core::ids::SlotIdx::new(1)),
        },
        None,
        Some(vb_core::ids::StepIdx::new(3)),
    )
}

fn snapshot_ask_resume_node() -> vb_core::workflow::CompiledNode {
    vb_core::workflow::node(
        vb_core::ids::StepIdx::new(3),
        vb_core::workflow::CompiledNodeKind::AskResume {
            answer: vb_core::ids::SlotIdx::new(2),
        },
        None,
        Some(vb_core::ids::StepIdx::new(4)),
    )
}

fn snapshot_ask_finish_node() -> vb_core::workflow::CompiledNode {
    vb_core::workflow::node(
        vb_core::ids::StepIdx::new(4),
        vb_core::workflow::CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::new(2),
        },
        None,
        None,
    )
}

fn bounded_snapshot_ask_nodes() -> Box<[vb_core::workflow::CompiledNode]> {
    Box::from([
        snapshot_ask_prompt_node(),
        snapshot_ask_timeout_node(),
        snapshot_ask_node(),
        snapshot_ask_resume_node(),
        snapshot_ask_finish_node(),
    ])
}

fn bounded_snapshot_ask_parts() -> vb_core::workflow::WorkflowParts {
    vb_core::workflow::WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("bounded_snapshot_timed_ask"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([5; 32]),
        nodes: bounded_snapshot_ask_nodes(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([
            vb_core::value::ConstValue::Symbol(vb_core::ids::SymbolId::new(1)),
            vb_core::value::ConstValue::I64(86_400),
        ]),
        slot_count: 3,
        symbols_count: 2,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    }
}

fn bounded_snapshot_ask_workflow() -> Result<vb_core::workflow::CompiledWorkflow, String> {
    vb_core::workflow::CompiledWorkflow::try_from_parts(bounded_snapshot_ask_parts())
        .map_err(|error| format!("{error:?}"))
}

fn bounded_snapshot_submit_ask(shard: &mut Shard, run: RunId) -> Result<(), String> {
    let workflow = bounded_snapshot_ask_workflow()?;
    shard
        .enqueue(ShardCommand::SubmitWithContracts {
            run,
            workflow,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0,
            )),
        })
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(shard.tick(), Ok(true));
    Ok(())
}

fn bounded_snapshot_next_timer(
    previous: crate::shard::types::PendingTimer,
    deadline: std::time::Instant,
) -> Result<crate::shard::types::PendingTimer, String> {
    Ok(crate::shard::types::PendingTimer {
        generation: previous
            .generation
            .checked_add(1)
            .ok_or_else(|| String::from("timer generation must advance"))?,
        deadline,
        ..previous
    })
}

fn bounded_snapshot_freeze_timer(
    shard: &mut Shard,
    run: RunId,
    deadline: std::time::Instant,
    generation_advances: u8,
) -> Result<(), String> {
    let mut previous = shard
        .pending_timer_get(run)
        .ok_or_else(|| String::from("submitted run must own a pending timer"))?;
    for _ in 0..generation_advances {
        let replacement = bounded_snapshot_next_timer(previous, deadline)?;
        assert_eq!(
            shard.pending_timer_insert(run, replacement),
            Ok(Some(previous))
        );
        previous = replacement;
    }
    assert_eq!(shard.pending_timer_get(run), Some(previous));
    Ok(())
}

fn bounded_snapshot_tuples(
    boundary: &crate::shard::ShardPendingBoundarySnapshot,
) -> Vec<BoundedSnapshotTuple> {
    boundary
        .pending_timers()
        .iter()
        .map(|timer| {
            (
                timer.run_id(),
                timer.step(),
                timer.kind(),
                timer.generation(),
                timer.deadline(),
            )
        })
        .collect()
}

struct BoundedSnapshotFixture {
    shard: Shard,
    first_deadline: std::time::Instant,
    second_deadline: std::time::Instant,
    third_deadline: std::time::Instant,
}

fn bounded_snapshot_fixture() -> Result<BoundedSnapshotFixture, String> {
    let first_inserted = RunId::new(93_003);
    let second_inserted = RunId::new(93_001);
    let third_inserted = RunId::new(93_002);
    let mut shard = Shard::new(small_config());
    let first_deadline = bounded_snapshot_future_deadline(0)?;
    timer_due_regression_submit_wait(&mut shard, first_inserted)?;
    bounded_snapshot_freeze_timer(&mut shard, first_inserted, first_deadline, 1)?;
    let second_deadline = bounded_snapshot_future_deadline(60)?;
    bounded_snapshot_submit_ask(&mut shard, second_inserted)?;
    bounded_snapshot_freeze_timer(&mut shard, second_inserted, second_deadline, 2)?;
    let third_deadline = bounded_snapshot_future_deadline(120)?;
    timer_due_regression_submit_wait(&mut shard, third_inserted)?;
    bounded_snapshot_freeze_timer(&mut shard, third_inserted, third_deadline, 3)?;
    assert_eq!(shard.active_run_count(), 3);
    assert_eq!(shard.pending_timer_count(), 3);
    assert_eq!(shard.pending_boundary_snapshot(0, 3).pending_ask_count(), 1);
    Ok(BoundedSnapshotFixture {
        shard,
        first_deadline,
        second_deadline,
        third_deadline,
    })
}
