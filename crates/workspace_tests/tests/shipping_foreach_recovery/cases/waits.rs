use super::*;

#[test]
fn owned_foreach_pending_ask_hydrates_without_reissue() -> TestResult {
    let source = ask_source();
    let ask = control_step(&source, |kind| matches!(kind, K::Ask { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(1), SlotValue::I64(2)],
        &[Taint::Clean; 2],
        (ask, StepState::Asking),
    )?;
    replay_parity(&cut)?;
    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let mut runtime = runtime(journal.clone(), 1);
    assert!(matches!(
        runtime.recover_product(RUN)?,
        RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime.recover_and_resume(RUN)?;
    let mut answers = [41_i64, 42_i64].into_iter();
    for _ in 0..256 {
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(RUN, ask) {
            let value = answers.next().ok_or("more ask tickets than answers")?;
            runtime.answer_ask(AskAnswer::new(
                ticket,
                ticket.answer_slot(),
                SlotValue::I64(value),
                Taint::Clean,
            ))?;
        }
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(answers.next(), None, "answer list not consumed");
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::AskScheduledEvent { .. }))
            .count(),
        2,
        "recovery must not reissue an already recorded ask"
    );
    let replay = replay_owned_for_each(&events, &cut.workflow, &cut.roots, Some(&cut.input_store))?
        .ok_or("owned ask replay not selected")?;
    let result = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .ok_or("missing RunFinished")?;
    let SlotValue::List(list) = *replay.frame.read_slot(result)? else {
        return Err("ask result is not a list".into());
    };
    let store = ValueStore::from_checkpoint_v1(replay.store.clone())?;
    assert_eq!(store.list(list)?, [SlotValue::I64(41), SlotValue::I64(42)]);
    Ok(())
}

#[test]
fn wait_resolution_prefix_recovers_and_resumes_after_reopen() -> TestResult {
    let source = wait_source();
    let wait = control_step(&source, |kind| matches!(kind, K::WaitUntil { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(0)],
        &[Taint::Clean],
        (wait, StepState::Succeeded),
    )?;
    assert_eq!(cut.events.iter().filter(|event| matches!(event, JournalEvent::WaitScheduledEvent { run, step, attempt: 1, .. } if *run == RUN && *step == wait)).count(), 1, "exact scheduled wait authority");
    assert_eq!(cut.events.iter().filter(|event| matches!(event, JournalEvent::WaitResolvedEvent { run, step, attempt: 1, .. } if *run == RUN && *step == wait)).count(), 1, "exact resolved wait authority");
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(
        events
            .iter()
            .filter_map(|event| match event {
                JournalEvent::WaitScheduledEvent {
                    run, step, attempt, ..
                } => Some((*run, *step, *attempt)),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [(RUN, wait, 1)],
        "cold resume must not reissue the resolved wait"
    );
    assert_eq!(
        events
            .iter()
            .filter_map(|event| match event {
                JournalEvent::WaitResolvedEvent {
                    run, step, attempt, ..
                } => Some((*run, *step, *attempt)),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [(RUN, wait, 1)],
        "cold resume must preserve exactly one resolved wait authority"
    );
    let (value, taint) = terminal_result(&events)?;
    let SlotValue::List(list) = value else {
        return Err("wait result is not a list".into());
    };
    assert_eq!(taint, Taint::Clean);
    let replay = replay_owned_for_each(&events, &cut.workflow, &cut.roots, Some(&cut.input_store))?
        .ok_or("wait replay not selected")?;
    let store = ValueStore::from_checkpoint_v1(replay.store.clone())?;
    assert_eq!(
        store.list_items_with_taint(list)?.collect::<Vec<_>>(),
        [(SlotValue::I64(0), Taint::Clean)]
    );
    Ok(())
}

#[test]
fn timed_out_owned_ask_recovers_as_terminal_after_reopen() -> TestResult {
    let source = ask_timeout_source();
    let ask = control_step(&source, |kind| matches!(kind, K::Ask { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(0)],
        &[Taint::Clean],
        (ask, StepState::Asking),
    )?;
    {
        let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
        let mut runtime = runtime(journal, 1);
        assert!(matches!(
            runtime.recover_product(RUN)?,
            RuntimeRecoveryProduct::Resumable(_)
        ));
        runtime.recover_and_resume(RUN)?;
        for _ in 0..256 {
            runtime.tick_all()?;
            if runtime.counters_snapshot().runs_failed == 1 {
                break;
            }
        }
        assert_eq!(runtime.counters_snapshot().runs_failed, 1);
    }
    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let runtime = runtime(journal, 1);
    let product = runtime.recover_product(RUN)?;
    assert_eq!(product.cannot_resume_reason(), Some("terminal_state"));
    assert_eq!(
        product.summary().terminal,
        Some(vb_storage::recovery::RecoveryTerminalState::Failed)
    );
    Ok(())
}

#[test]
fn owned_replay_without_step_start_uses_default_source_store() -> TestResult {
    let source = multi_source("count($step.loop)");
    let accepted = accepted_run(&source, &[SlotValue::I64(1)], &[Taint::Clean])?;
    let journal = Arc::new(FjallJournal::open(accepted.directory.path(), None)?);
    let prefix = journal.events_for_run_full(RUN)?;
    assert!(
        !prefix
            .iter()
            .any(|event| matches!(event, JournalEvent::StepStarted { .. })),
        "accepted-only journal must have no StepStarted event"
    );
    let mut runtime = runtime(journal.clone(), 1);
    let product = runtime.recover_product(RUN)?;
    assert!(matches!(product, RuntimeRecoveryProduct::Resumable(_)));
    assert_eq!(
        product.summary().steps_started,
        0,
        "public cold recovery starts with no prior step"
    );
    runtime.recover_and_resume(RUN)?;
    for _ in 0..256 {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(terminal_result(&events)?, (SlotValue::I64(1), Taint::Clean));
    Ok(())
}

#[test]
fn accepted_for_each_limit_recovers_and_resumes_after_reopen() -> TestResult {
    let source = multi_source("count($step.loop)");
    let values = [
        SlotValue::I64(1),
        SlotValue::I64(2),
        SlotValue::I64(3),
        SlotValue::I64(4),
    ];
    let cut = cut_at(
        &source,
        &values,
        &[Taint::Clean; 4],
        (body_step(&source)?, StepState::Succeeded),
    )?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(terminal_result(&events)?, (SlotValue::I64(4), Taint::Clean));
    Ok(())
}

#[test]
fn owned_foreach_pending_wait_restores_timer_authority() -> TestResult {
    let source = wait_source();
    let wait = control_step(&source, |kind| matches!(kind, K::WaitUntil { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(60_000)],
        &[Taint::Clean],
        (wait, StepState::Waiting),
    )?;
    replay_parity(&cut)?;
    assert_eq!(cut.events.iter().filter(|event| matches!(event, JournalEvent::WaitScheduledEvent { run, step, attempt: 1, .. } if *run == RUN && *step == wait)).count(), 1);
    assert_eq!(
        cut.events
            .iter()
            .filter(|event| matches!(event, JournalEvent::WaitResolvedEvent { .. }))
            .count(),
        0
    );
    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let live_deadline = cut
        .live_deadline
        .ok_or("pending wait cut lacks its live deadline")?;
    let mut runtime = runtime(journal.clone(), 1);
    let recovery_started = std::time::Instant::now();
    assert!(matches!(
        runtime
            .recover_product(RUN)
            .map_err(|error| format!("wait recover_product: {error:?}"))?,
        RuntimeRecoveryProduct::Resumable(_)
    ));
    runtime
        .recover_and_resume(RUN)
        .map_err(|error| format!("wait recover_and_resume: {error:?}"))?;
    runtime.tick_all()?;
    let timer = runtime
        .capture_timer_entry(RUN)
        .map_err(|error| format!("wait capture_timer_entry: {error:?}"))?;
    let recovery_observed = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(60_000);
    let earliest_deadline = recovery_started
        .checked_add(timeout)
        .ok_or("recovery timeout deadline overflow")?;
    let latest_deadline = recovery_observed
        .checked_add(timeout)
        .ok_or("recovery timeout deadline overflow")?;
    assert_eq!(timer.run, RUN);
    assert_eq!(timer.kind, PendingTimerKind::Wait);
    assert!(timer.generation > 0);
    assert!(
        timer.deadline > live_deadline,
        "WaitScheduled has no durable deadline; recovery restores the configured timeout from the cold-reopen point"
    );
    assert!(
        timer.deadline >= earliest_deadline && timer.deadline <= latest_deadline,
        "recovered deadline must match the 60-second wait input within the observed recovery interval"
    );
    let recovered_events = journal.events_for_run_full(RUN)?;
    assert_eq!(recovered_events.iter().filter(|event| matches!(event, JournalEvent::WaitScheduledEvent { run, step, attempt: 1, .. } if *run == RUN && *step == wait)).count(), 1, "recovery must not duplicate scheduled wait");
    assert_eq!(
        recovered_events
            .iter()
            .filter(|event| matches!(event, JournalEvent::WaitResolvedEvent { .. }))
            .count(),
        0,
        "pending wait must remain unresolved"
    );
    Ok(())
}
