fn strict_cold_timer_runtime(
    journal: std::sync::Arc<vb_storage::FjallJournal>,
) -> crate::runtime::Runtime {
    crate::runtime::Runtime::new(std::num::NonZeroUsize::MIN, ShardConfig {
        policy: vb_core::RuntimePolicy::Strict,
        ..ShardConfig::default()
    }, crate::journal::StorageRuntimeJournal::shared_strict(journal)).expect("valid test configuration")
}

#[derive(Debug, PartialEq, Eq)]
struct ColdTimerJournalSnapshot {
    events: Vec<vb_storage::JournalEvent>,
    records: Vec<(vb_storage::EventSeq, Vec<u8>)>,
}

fn cold_timer_events(
    journal: &vb_storage::FjallJournal,
    run: RunId,
) -> Result<ColdTimerJournalSnapshot, String> {
    let events = journal
        .events_for_run_full(run)
        .map_err(|error| error.to_string())?;
    let records = events
        .iter()
        .map(|event| {
            assert_eq!(event.run_id(), run);
            let seq = event.seq();
            let bytes = journal
                .get_event_bytes(run, seq)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("missing stored event for {run:?} at {seq:?}"))?;
            Ok((seq, bytes))
        })
        .collect::<Result<Vec<_>, String>>()?;
    assert_eq!(records.len(), events.len());
    assert!(records.is_sorted_by(|first, second| first.0.get() < second.0.get()));
    Ok(ColdTimerJournalSnapshot { events, records })
}

fn cold_timer_schedule_generation(
    events: &[vb_storage::JournalEvent],
    run: RunId,
    step: StepIdx,
    kind: PendingTimerKind,
) -> Result<u64, String> {
    let schedules = events
        .iter()
        .filter(|event| matches!((kind, event),
            (PendingTimerKind::Wait, vb_storage::JournalEvent::WaitScheduledEvent { run: found, step: found_step, .. })
            | (PendingTimerKind::Ask, vb_storage::JournalEvent::AskScheduledEvent { run: found, step: found_step, .. })
                if *found == run && *found_step == step))
        .map(vb_storage::JournalEvent::seq)
        .collect::<Vec<_>>();
    assert_eq!(schedules.len(), 1);
    schedules
        .first()
        .and_then(|seq| seq.get().checked_add(1))
        .ok_or_else(|| "timer generation".into())
}

fn assert_current_cold_admission(
    events: &[vb_storage::JournalEvent],
    run: RunId,
    artifact: &vb_storage::AcceptedArtifact,
    expected_budget: vb_core::AggregateResourceBudget,
) -> Result<(), String> {
    let input = vb_storage::inputs::compute_input_digest(&[]).map_err(|error| error.to_string())?;
    assert!(expected_budget.max_timer_entries > 0);
    assert_eq!(events.iter().filter(|event| matches!(event,
        vb_storage::JournalEvent::RunAdmissionBudget { run: found, artifact_digest, input_digest, policy_pin, policy, budget, .. }
            if *found == run && *artifact_digest == artifact.digest
                && *input_digest == vb_storage::RunInputBinding::Digest(input)
                && *policy_pin == vb_storage::RunPolicyPin::Pinned(artifact.policy_digest)
                && *policy == vb_core::RuntimePolicy::Strict && *budget == expected_budget)).count(), 1);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event,
        vb_storage::JournalEvent::RunBudgetOriginEvent { run: found, started_millis, .. }
            if *found == run && *started_millis > 0))
            .count(),
        1
    );
    Ok(())
}

fn timed_cold_workflow(
    workflow: CompiledWorkflow,
    duration: std::time::Duration,
) -> Result<CompiledWorkflow, String> {
    let mut parts = workflow.to_parts();
    assert_eq!(
        parts
            .constants
            .iter()
            .filter(|value| matches!(value, ConstValue::I64(10)))
            .count(),
        1
    );
    let timeout = parts
        .constants
        .iter_mut()
        .find(|value| matches!(value, ConstValue::I64(10)))
        .ok_or("fixture timeout constant")?;
    *timeout =
        ConstValue::I64(i64::try_from(duration.as_millis()).map_err(|error| error.to_string())?);
    honest_digest_workflow(
        CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?,
    )
}

struct ColdTimerFixture {
    runtime: crate::runtime::Runtime,
    journal: std::sync::Arc<vb_storage::FjallJournal>,
    _directory: tempfile::TempDir,
}

fn hydrate_strict_cold_timer(
    workflow: CompiledWorkflow,
    run: RunId,
    step: StepIdx,
    kind: PendingTimerKind,
    timeout: std::time::Duration,
) -> Result<ColdTimerFixture, String> {
    let workflow = timed_cold_workflow(workflow, timeout)?;
    let expected_budget = vb_core::AggregateResourceBudget::from_workflow(&workflow)
        .map_err(|error| error.to_string())?;
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let (before, generation) = {
        let journal = std::sync::Arc::new(
            vb_storage::FjallJournal::open(directory.path(), None)
                .map_err(|error| error.to_string())?,
        );
        let artifact = vb_storage::submit_artifact_with_contracts(
            &journal,
            &workflow,
            vb_core::RuntimePolicy::Strict,
            &[],
        )
        .map_err(|error| error.to_string())?;
        let mut runtime = strict_cold_timer_runtime(journal.clone());
        runtime
            .submit_direct(run, workflow)
            .map_err(|error| error.to_string())?;
        assert_eq!(runtime.tick_all(), Ok(true));
        let events = cold_timer_events(&journal, run)?;
        assert_current_cold_admission(&events.events, run, &artifact, expected_budget)?;
        let generation = cold_timer_schedule_generation(&events.events, run, step, kind)?;
        let entry = runtime
            .capture_timer_entry(run)
            .map_err(|error| error.to_string())?;
        assert_eq!(
            (entry.run, entry.kind, entry.generation),
            (run, kind, generation)
        );
        (events, generation)
    };
    let journal = std::sync::Arc::new(
        vb_storage::FjallJournal::open(directory.path(), None)
            .map_err(|error| error.to_string())?,
    );
    let mut runtime = strict_cold_timer_runtime(journal.clone());
    let started = std::time::Instant::now();
    runtime
        .recover_and_resume(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    let finished = std::time::Instant::now();
    assert_eq!(cold_timer_events(&journal, run)?, before);
    let boundary = runtime.pending_boundary_snapshot(8);
    assert_eq!(boundary.pending_timer_count(), 1);
    let shard = boundary.shards().first().ok_or("shard snapshot")?;
    let timer = shard.pending_timers().first().ok_or("timer snapshot")?;
    let entry = runtime
        .capture_timer_entry(run)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        (
            timer.run_id(),
            timer.step(),
            timer.kind(),
            timer.generation()
        ),
        (run, step, kind, generation)
    );
    assert_eq!(
        (entry.run, entry.kind, entry.generation, entry.deadline),
        (run, kind, generation, timer.deadline())
    );
    assert!(timer.deadline() >= started.checked_add(timeout).ok_or("earliest deadline")?);
    assert!(timer.deadline() <= finished.checked_add(timeout).ok_or("latest deadline")?);
    assert!(
        matches!(runtime.snapshot_run(run, 1), Ok(InspectResponse::Found(found)) if found.run == run && found.pc == step)
    );
    Ok(ColdTimerFixture {
        runtime,
        journal,
        _directory: directory,
    })
}

fn strict_cold_timer_case(
    workflow: CompiledWorkflow,
    run: RunId,
    step: StepIdx,
    kind: PendingTimerKind,
) -> Result<(), String> {
    let ColdTimerFixture {
        mut runtime,
        journal,
        _directory,
    } = hydrate_strict_cold_timer(workflow, run, step, kind, std::time::Duration::ZERO)?;
    let suspended = cold_timer_events(&journal, run)?;
    let entry = runtime
        .capture_timer_entry(run)
        .map_err(|error| error.to_string())?;
    assert!(entry.deadline <= std::time::Instant::now());
    runtime
        .timer_entry_fired(entry)
        .map_err(|error| error.to_string())?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(
        runtime.pending_boundary_snapshot(8).pending_timer_count(),
        0
    );
    assert!(matches!(
        runtime.snapshot_run(run, 2),
        Ok(InspectResponse::NotFound { .. })
    ));
    let events = cold_timer_events(&journal, run)?;
    assert_eq!(
        events.events.get(..suspended.events.len()),
        Some(suspended.events.as_slice())
    );
    assert_eq!(
        events.records.get(..suspended.records.len()),
        Some(suspended.records.as_slice())
    );
    assert_eq!(events.events.iter().filter(|event| matches!((kind, event),
        (PendingTimerKind::Wait, vb_storage::JournalEvent::WaitResolvedEvent { run: found, step: found_step, .. })
        | (PendingTimerKind::Ask, vb_storage::JournalEvent::AskTimedOutEvent { run: found, step: found_step, .. })
            if *found == run && *found_step == step)).count(), 1);
    let counters = runtime.counters_snapshot();
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(cold_timer_events(&journal, run)?, events);
    assert_eq!(runtime.counters_snapshot(), counters);
    Ok(())
}

#[test]
fn strict_fjall_cold_wait_hydrates_authority_without_journal_change_then_resolves_once()
-> Result<(), String> {
    strict_cold_timer_case(
        timer_wait_workflow().ok_or("wait workflow")?,
        RunId::new(84_040),
        StepIdx::new(1),
        PendingTimerKind::Wait,
    )
}

#[test]
fn strict_fjall_cold_ask_hydrates_authority_without_journal_change_then_times_out_once()
-> Result<(), String> {
    strict_cold_timer_case(
        timer_ask_workflow().ok_or("ask workflow")?,
        RunId::new(84_041),
        StepIdx::new(2),
        PendingTimerKind::Ask,
    )
}

#[test]
fn strict_fjall_cold_wait_preserves_full_positive_timeout() -> Result<(), String> {
    hydrate_strict_cold_timer(
        timer_wait_workflow().ok_or("wait workflow")?,
        RunId::new(84_042),
        StepIdx::new(1),
        PendingTimerKind::Wait,
        std::time::Duration::from_millis(10),
    )
    .map(|_| ())
}

#[test]
fn strict_fjall_cold_ask_preserves_full_positive_timeout() -> Result<(), String> {
    hydrate_strict_cold_timer(
        timer_ask_workflow().ok_or("ask workflow")?,
        RunId::new(84_043),
        StepIdx::new(2),
        PendingTimerKind::Ask,
        std::time::Duration::from_millis(10),
    )
    .map(|_| ())
}
