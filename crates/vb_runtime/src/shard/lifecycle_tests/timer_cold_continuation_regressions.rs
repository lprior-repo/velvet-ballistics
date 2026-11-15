fn assert_cold_journal_prefix(before: &ColdTimerJournalSnapshot, after: &ColdTimerJournalSnapshot) {
    assert_eq!(
        after.events.get(..before.events.len()),
        Some(before.events.as_slice())
    );
    assert_eq!(
        after.records.get(..before.records.len()),
        Some(before.records.as_slice())
    );
}

fn cold_resolution_count(events: &[vb_storage::JournalEvent], run: RunId) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(event,
        vb_storage::JournalEvent::WaitResolvedEvent { run: found, .. }
        | vb_storage::JournalEvent::AskTimedOutEvent { run: found, .. }
            if *found == run)
        })
        .count()
}

fn assert_cold_timer_retired(
    fixture: &mut ColdTimerFixture,
    run: RunId,
    before: &ColdTimerJournalSnapshot,
) -> Result<ColdTimerJournalSnapshot, String> {
    let boundary = fixture.runtime.pending_boundary_snapshot(8);
    assert_eq!(boundary.pending_timer_count(), 0);
    assert!(
        boundary
            .shards()
            .iter()
            .all(|shard| shard.pending_timers().is_empty())
    );
    assert_eq!(
        fixture.runtime.capture_timer_entry(run),
        Err(RuntimeError::InvalidTimerFire)
    );
    assert!(matches!(
        fixture.runtime.snapshot_run(run, 5),
        Ok(InspectResponse::NotFound { .. })
    ));
    let after = cold_timer_events(&fixture.journal, run)?;
    assert_cold_journal_prefix(before, &after);
    Ok(after)
}

fn assert_cold_idle_unchanged(
    fixture: &mut ColdTimerFixture,
    run: RunId,
    expected: &ColdTimerJournalSnapshot,
) -> Result<(), String> {
    let counters = fixture.runtime.counters_snapshot();
    let boundary = fixture.runtime.pending_boundary_snapshot(8);
    assert_eq!(fixture.runtime.tick_all(), Ok(true));
    assert_eq!(fixture.runtime.counters_snapshot(), counters);
    assert_eq!(fixture.runtime.pending_boundary_snapshot(8), boundary);
    assert_eq!(cold_timer_events(&fixture.journal, run)?, *expected);
    Ok(())
}

fn strict_cold_automatic_timer_case(
    workflow: CompiledWorkflow,
    run: RunId,
    step: StepIdx,
    kind: PendingTimerKind,
) -> Result<(), String> {
    let mut fixture = hydrate_strict_cold_timer(
        workflow,
        run,
        step,
        kind,
        std::time::Duration::from_millis(10),
    )?;
    let before = cold_timer_events(&fixture.journal, run)?;
    let entry = fixture
        .runtime
        .capture_timer_entry(run)
        .map_err(|error| error.to_string())?;
    std::thread::sleep(
        entry
            .deadline
            .saturating_duration_since(std::time::Instant::now()),
    );
    assert_eq!(fixture.runtime.tick_all(), Ok(true));
    let after = assert_cold_timer_retired(&mut fixture, run, &before)?;
    assert_eq!(cold_resolution_count(&after.events, run), 1);
    let counters = fixture.runtime.counters_snapshot();
    match kind {
        PendingTimerKind::Wait => {
            assert_eq!(counters.runs_completed, 1);
            assert_eq!(counters.runs_failed, 0);
            assert_eq!(
                after
                    .events
                    .iter()
                    .filter(|event| matches!(event,
                vb_storage::JournalEvent::RunFinished { run: found, .. } if *found == run))
                    .count(),
                1
            );
        }
        PendingTimerKind::Ask => {
            assert_eq!(counters.runs_completed, 0);
            assert_eq!(counters.runs_failed, 1);
            assert_eq!(
                after
                    .events
                    .iter()
                    .filter(|event| matches!(event,
                vb_storage::JournalEvent::RunFailedEvent { run: found, .. } if *found == run))
                    .count(),
                1
            );
        }
    }
    assert_cold_idle_unchanged(&mut fixture, run, &after)
}

#[test]
fn strict_fjall_cold_positive_wait_resumes_on_empty_tick_once() -> Result<(), String> {
    strict_cold_automatic_timer_case(
        timer_wait_workflow().ok_or("wait workflow")?,
        RunId::new(84_050),
        StepIdx::new(1),
        PendingTimerKind::Wait,
    )
}

#[test]
fn strict_fjall_cold_positive_ask_times_out_on_empty_tick_once() -> Result<(), String> {
    strict_cold_automatic_timer_case(
        timer_ask_workflow().ok_or("ask workflow")?,
        RunId::new(84_051),
        StepIdx::new(2),
        PendingTimerKind::Ask,
    )
}

fn strict_cold_cancel_timer_case(
    workflow: CompiledWorkflow,
    run: RunId,
    step: StepIdx,
    kind: PendingTimerKind,
) -> Result<(), String> {
    let mut fixture = hydrate_strict_cold_timer(
        workflow,
        run,
        step,
        kind,
        std::time::Duration::from_millis(10),
    )?;
    let before = cold_timer_events(&fixture.journal, run)?;
    let stale = fixture
        .runtime
        .capture_timer_entry(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(fixture.runtime.cancel_run(run), Ok(()));
    assert_eq!(fixture.runtime.tick_all(), Ok(true));
    let after = assert_cold_timer_retired(&mut fixture, run, &before)?;
    assert_eq!(cold_resolution_count(&after.events, run), 0);
    assert_eq!(
        after
            .events
            .iter()
            .filter(|event| matches!(event,
        vb_storage::JournalEvent::RunCancelled { run: found, .. } if *found == run))
            .count(),
        1
    );
    assert_eq!(fixture.runtime.counters_snapshot().runs_failed, 1);
    assert_eq!(fixture.runtime.counters_snapshot().runs_completed, 0);
    assert_eq!(fixture.runtime.lookup_pending_ask_ticket(run, step), None);
    assert_eq!(
        fixture.runtime.timer_entry_fired(stale),
        Err(RuntimeError::InvalidTimerFire)
    );
    std::thread::sleep(
        stale
            .deadline
            .saturating_duration_since(std::time::Instant::now()),
    );
    assert_cold_idle_unchanged(&mut fixture, run, &after)
}

#[test]
fn strict_fjall_cold_cancelled_wait_cannot_resolve_after_deadline() -> Result<(), String> {
    strict_cold_cancel_timer_case(
        timer_wait_workflow().ok_or("wait workflow")?,
        RunId::new(84_052),
        StepIdx::new(1),
        PendingTimerKind::Wait,
    )
}

#[test]
fn strict_fjall_cold_cancelled_ask_cannot_time_out_after_deadline() -> Result<(), String> {
    strict_cold_cancel_timer_case(
        timer_ask_workflow().ok_or("ask workflow")?,
        RunId::new(84_053),
        StepIdx::new(2),
        PendingTimerKind::Ask,
    )
}

#[test]
fn strict_fjall_cold_answered_ask_cannot_time_out_after_deadline() -> Result<(), String> {
    let run = RunId::new(84_054);
    let mut fixture = hydrate_strict_cold_timer(
        timer_ask_workflow().ok_or("ask workflow")?,
        run,
        StepIdx::new(2),
        PendingTimerKind::Ask,
        std::time::Duration::from_millis(10),
    )?;
    let before = cold_timer_events(&fixture.journal, run)?;
    let stale = fixture
        .runtime
        .capture_timer_entry(run)
        .map_err(|error| error.to_string())?;
    let ticket = fixture
        .runtime
        .lookup_pending_ask_ticket(run, StepIdx::new(2))
        .ok_or("recovered ask ticket")?;
    assert_eq!(
        fixture.runtime.answer_ask(AskAnswer {
            ticket,
            answer_slot: SlotIdx::new(2),
            value: SlotValue::I64(42),
            taint: Taint::Clean,
            encoded_len: 0,
        }),
        Ok(())
    );
    assert_eq!(fixture.runtime.tick_all(), Ok(true));
    let after = assert_cold_timer_retired(&mut fixture, run, &before)?;
    assert_eq!(cold_resolution_count(&after.events, run), 0);
    assert_eq!(
        after
            .events
            .iter()
            .filter(|event| matches!(event,
        vb_storage::JournalEvent::AskAnsweredEvent { run: found, step, .. }
            if *found == run && *step == StepIdx::new(2)))
            .count(),
        1
    );
    assert_eq!(
        after
            .events
            .iter()
            .filter(|event| matches!(event,
        vb_storage::JournalEvent::RunFinished { run: found, .. } if *found == run))
            .count(),
        1
    );
    assert_eq!(fixture.runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(fixture.runtime.counters_snapshot().runs_failed, 0);
    assert_eq!(
        fixture
            .runtime
            .lookup_pending_ask_ticket(run, StepIdx::new(2)),
        None
    );
    assert_eq!(
        fixture.runtime.timer_entry_fired(stale),
        Err(RuntimeError::InvalidTimerFire)
    );
    std::thread::sleep(
        stale
            .deadline
            .saturating_duration_since(std::time::Instant::now()),
    );
    assert_cold_idle_unchanged(&mut fixture, run, &after)
}
