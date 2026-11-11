type TimerSnapshotOracle = (RunId, StepIdx, PendingTimerKind, u64, std::time::Instant);

struct CapacityState {
    boundary: crate::shard::ShardPendingBoundarySnapshot,
    timers: indexmap::IndexMap<RunId, crate::shard::types::PendingTimer>,
    counters: crate::counters::CounterSnapshot,
    runs: [Option<crate::shard::types::RunState>; 2],
}

fn capacity_two_config() -> ShardConfig {
    let mut config = small_config();
    config.max_active_runs = 2;
    config
}

fn future_deadline() -> Result<std::time::Instant, String> {
    std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(86_400))
        .ok_or_else(|| String::from("future Instant must be representable"))
}

fn submit_wait_run(shard: &mut Shard, run: RunId) -> Result<std::time::Instant, String> {
    timer_due_regression_submit_wait(shard, run)?;
    let deadline = future_deadline()?;
    timer_due_regression_set_deadline(shard, run, deadline)?;
    Ok(deadline)
}

fn expected_capacity_snapshot(
    low_run: RunId,
    low_generation: u64,
    low_deadline: std::time::Instant,
    high_run: RunId,
    high_deadline: std::time::Instant,
) -> [TimerSnapshotOracle; 2] {
    [
        (
            low_run,
            StepIdx::new(1),
            PendingTimerKind::Wait,
            low_generation,
            low_deadline,
        ),
        (
            high_run,
            StepIdx::new(1),
            PendingTimerKind::Wait,
            1,
            high_deadline,
        ),
    ]
}

fn expected_wait_timer(
    deadline: std::time::Instant,
    generation: u64,
) -> crate::shard::types::PendingTimer {
    crate::shard::types::PendingTimer {
        step: StepIdx::new(1),
        kind: PendingTimerKind::Wait,
        generation,
        deadline,
    }
}

fn assert_complete_timer_snapshot(shard: &Shard, expected: [TimerSnapshotOracle; 2]) {
    let boundary = shard.pending_boundary_snapshot(0, 2);
    assert_eq!(boundary.active_run_count(), 2);
    assert_eq!(boundary.pending_timer_count(), 2);
    assert!(!boundary.truncated());
    let actual = boundary
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
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

fn capacity_two_suspended(
    high_run: RunId,
    low_run: RunId,
) -> Result<(Shard, std::time::Instant, std::time::Instant), String> {
    let mut shard = Shard::new(capacity_two_config());
    let high_deadline = submit_wait_run(&mut shard, high_run)?;
    let low_deadline = submit_wait_run(&mut shard, low_run)?;
    assert_eq!(shard.command_queue_len(), 0);
    assert_complete_timer_snapshot(
        &shard,
        expected_capacity_snapshot(low_run, 1, low_deadline, high_run, high_deadline),
    );
    Ok((shard, low_deadline, high_deadline))
}

fn enqueue_third_wait_submission(shard: &mut Shard, run: RunId) -> Result<(), String> {
    let workflow = wait_workflow().ok_or_else(|| String::from("wait workflow must compile"))?;
    shard
        .enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            inputs: Box::from([]),
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0,
            )),
        })
        .map_err(|error| format!("{error:?}"))
}

fn next_replacement_timer(
    old_timer: crate::shard::types::PendingTimer,
) -> Result<crate::shard::types::PendingTimer, String> {
    let deadline = old_timer
        .deadline
        .checked_add(std::time::Duration::from_secs(86_400))
        .ok_or_else(|| String::from("replacement deadline must be representable"))?;
    let generation = old_timer
        .generation
        .checked_add(1)
        .ok_or_else(|| String::from("timer generation must advance"))?;
    assert_eq!(generation, 2);
    Ok(expected_wait_timer(deadline, generation))
}

fn insert_replacement_timer(
    shard: &mut Shard,
    run: RunId,
    old_timer: crate::shard::types::PendingTimer,
) -> Result<crate::shard::types::PendingTimer, String> {
    let replacement = next_replacement_timer(old_timer)?;
    let previous = shard
        .pending_timer_insert(run, replacement)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(previous, Some(old_timer));
    assert_eq!(shard.pending_timer_get(run), Some(replacement));
    Ok(replacement)
}

fn assert_full_replacement(
    shard: &mut Shard,
    low_run: RunId,
    old_timer: crate::shard::types::PendingTimer,
    high_run: RunId,
    high_deadline: std::time::Instant,
) -> Result<(), String> {
    let replacement = insert_replacement_timer(shard, low_run, old_timer)?;
    let captured = shard
        .timer_entry(low_run)
        .ok_or_else(|| String::from("replacement timer entry must exist"))?;
    assert_eq!(captured.run, low_run);
    assert_eq!(captured.generation, replacement.generation);
    assert_eq!(captured.deadline, replacement.deadline);
    assert_eq!(captured.kind, replacement.kind);
    assert_complete_timer_snapshot(
        shard,
        expected_capacity_snapshot(low_run, 2, replacement.deadline, high_run, high_deadline),
    );
    assert_eq!(shard.pending_timer_count(), 2);
    assert_eq!(
        shard.run_aggregate.pending_timer_next_deadline(),
        Some(replacement.deadline.min(high_deadline))
    );
    Ok(())
}

fn capacity_state(shard: &Shard, high_run: RunId, low_run: RunId) -> CapacityState {
    CapacityState {
        boundary: shard.pending_boundary_snapshot(0, 2),
        timers: shard.pending_timer_clone(),
        counters: shard.counters().snapshot(),
        runs: [
            shard.run_state_get(high_run).cloned(),
            shard.run_state_get(low_run).cloned(),
        ],
    }
}

fn assert_third_submit_rejected(shard: &mut Shard, third_run: RunId) -> Result<(), String> {
    enqueue_third_wait_submission(shard, third_run)?;
    assert_eq!(
        shard.pending_boundary_snapshot(0, 2).command_queue_depth(),
        1
    );
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 2 })
    );
    Ok(())
}

fn assert_capacity_state_unchanged(
    shard: &Shard,
    third_run: RunId,
    high_run: RunId,
    low_run: RunId,
    before: CapacityState,
) {
    let after = capacity_state(shard, high_run, low_run);
    assert_eq!(after.boundary, before.boundary);
    assert_eq!(after.timers, before.timers);
    assert_eq!(after.counters, before.counters);
    assert_eq!(after.runs, before.runs);
    assert_eq!(shard.active_run_count(), 2);
    assert_eq!(shard.pending_timer_count(), 2);
    assert!(shard.run_state_get(third_run).is_none());
}

fn assert_capacity_rejection_unchanged(
    shard: &mut Shard,
    third_run: RunId,
    high_run: RunId,
    low_run: RunId,
) -> Result<(), String> {
    let before = capacity_state(shard, high_run, low_run);
    assert_third_submit_rejected(shard, third_run)?;
    assert_capacity_state_unchanged(shard, third_run, high_run, low_run, before);
    Ok(())
}

#[test]
fn timer_wheel_capacity_two_admits_and_sorts_two_pending_timers() -> Result<(), String> {
    let high_run = RunId::new(85_002);
    let low_run = RunId::new(85_001);
    let (shard, _, _) = capacity_two_suspended(high_run, low_run)?;
    assert_eq!(shard.active_run_count(), 2);
    assert_eq!(shard.pending_timer_count(), 2);
    assert_eq!(shard.command_queue_len(), 0);
    Ok(())
}

#[test]
fn timer_wheel_capacity_two_replaces_one_timer_and_preserves_survivor() -> Result<(), String> {
    let high_run = RunId::new(85_012);
    let low_run = RunId::new(85_011);
    let (mut shard, low_deadline, high_deadline) = capacity_two_suspended(high_run, low_run)?;
    let old_timer = expected_wait_timer(low_deadline, 1);
    assert_eq!(shard.pending_timer_get(low_run), Some(old_timer));
    assert_full_replacement(&mut shard, low_run, old_timer, high_run, high_deadline)?;
    Ok(())
}

#[test]
fn timer_wheel_capacity_two_rejects_third_submit_atomically() -> Result<(), String> {
    let high_run = RunId::new(85_022);
    let low_run = RunId::new(85_021);
    let third_run = RunId::new(85_023);
    let (mut shard, _, _) = capacity_two_suspended(high_run, low_run)?;
    assert_capacity_rejection_unchanged(&mut shard, third_run, high_run, low_run)?;
    Ok(())
}
