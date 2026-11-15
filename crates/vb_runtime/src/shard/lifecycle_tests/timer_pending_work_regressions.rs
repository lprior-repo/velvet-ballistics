#[test]
fn future_timer_empty_tick_continues_pending_work_without_changing_timer() -> Result<(), String> {
    let waiting = RunId::new(84_060);
    let working = RunId::new(84_061);
    let (mut shard, journal) = timer_due_regression_fixture(waiting)?;
    let deadline = std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(86_400))
        .ok_or("future deadline")?;
    let timer = timer_due_regression_set_deadline(&mut shard, waiting, deadline)?;
    let waiting_state = shard.run_state_get(waiting).ok_or("waiting state")?.clone();
    let waiting_sequence = shard.journal_seq_get(waiting);
    shard.step_budget_per_tick = 1;
    shard
        .enqueue(ShardCommand::SubmitWithContracts {
            run: working,
            workflow: two_slot_workflow().ok_or("pending workflow")?,
            caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
            action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(
                0,
            )),
        })
        .map_err(|error| error.to_string())?;
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard
            .run_state_get(working)
            .ok_or("first continuation")?
            .frame
            .pc(),
        StepIdx::new(1)
    );
    let before = journal.snapshot().map_err(|error| error.to_string())?;
    assert_eq!(shard.command_queue_len(), 0);
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(
        shard
            .run_state_get(working)
            .ok_or("second continuation")?
            .frame
            .pc(),
        StepIdx::new(2)
    );
    assert_eq!(shard.run_state_get(waiting), Some(&waiting_state));
    assert_eq!(shard.journal_seq_get(waiting), waiting_sequence);
    assert_eq!(shard.pending_timer_get(waiting), Some(timer));
    assert_eq!(shard.pending_timer_count(), 1);
    assert_eq!(shard.tick(), Ok(true));
    assert!(shard.terminal_runs_contains(working));
    assert_eq!(shard.run_state_get(working), None);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    let after = journal.snapshot().map_err(|error| error.to_string())?;
    assert_eq!(after.get(..before.len()), Some(before.as_slice()));
    assert_eq!(
        after
            .iter()
            .filter(|event| matches!(event,
        RuntimeJournalEvent::RunFinished { run, .. } if *run == working))
            .count(),
        1
    );
    assert_eq!(
        after
            .iter()
            .filter(|event| matches!(event,
        RuntimeJournalEvent::WaitResolved { run, .. } if *run == waiting))
            .count(),
        0
    );
    let counters = shard.counters().snapshot();
    let boundary = shard.pending_boundary_snapshot(0, 4);
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.pending_boundary_snapshot(0, 4), boundary);
    assert_eq!(shard.counters().snapshot(), counters);
    assert_eq!(
        journal.snapshot().map_err(|error| error.to_string())?,
        after
    );
    Ok(())
}

#[test]
fn timer_wheel_snapshot_selects_insertion_prefix_before_sorting() -> Result<(), String> {
    let first_run = RunId::new(85_033);
    let second_run = RunId::new(85_032);
    let third_run = RunId::new(85_031);
    let (mut shard, journal) = timer_due_regression_fixture(first_run)?;
    let first = timer_due_regression_set_deadline(&mut shard, first_run, future_deadline()?)?;
    timer_due_regression_submit_wait(&mut shard, second_run)?;
    let second = timer_due_regression_set_deadline(&mut shard, second_run, future_deadline()?)?;
    timer_due_regression_submit_wait(&mut shard, third_run)?;
    let third = timer_due_regression_set_deadline(&mut shard, third_run, future_deadline()?)?;
    let original_first = first;
    let first = crate::shard::PendingTimer {
        generation: first
            .generation
            .checked_add(1)
            .ok_or("replacement generation")?,
        deadline: future_deadline()?,
        ..first
    };
    assert_eq!(
        shard.pending_timer_insert(first_run, first),
        Ok(Some(original_first))
    );
    let events = journal.snapshot().map_err(|error| error.to_string())?;
    let snapshot = shard.pending_boundary_snapshot(0, 2);
    assert_eq!(snapshot.pending_timer_count(), 3);
    assert!(snapshot.truncated());
    let actual = snapshot
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
    assert_eq!(
        actual,
        vec![
            (
                second_run,
                second.step,
                second.kind,
                second.generation,
                second.deadline
            ),
            (
                first_run,
                first.step,
                first.kind,
                first.generation,
                first.deadline
            ),
        ]
    );
    assert_eq!(
        journal.snapshot().map_err(|error| error.to_string())?,
        events
    );
    shard
        .enqueue(ShardCommand::Cancel {
            run: first_run,
            reason: None,
        })
        .map_err(|error| error.to_string())?;
    assert_eq!(shard.tick(), Ok(true));
    let after_removal = shard.pending_boundary_snapshot(0, 1);
    assert_eq!(after_removal.pending_timer_count(), 2);
    assert!(after_removal.truncated());
    let selected = after_removal
        .pending_timers()
        .first()
        .ok_or("swap-removed prefix")?;
    assert_eq!(
        (
            selected.run_id(),
            selected.step(),
            selected.kind(),
            selected.generation(),
            selected.deadline()
        ),
        (
            third_run,
            third.step,
            third.kind,
            third.generation,
            third.deadline
        )
    );
    assert_eq!(shard.pending_timer_get(second_run), Some(second));
    assert_eq!(shard.pending_timer_get(first_run), None);
    Ok(())
}
