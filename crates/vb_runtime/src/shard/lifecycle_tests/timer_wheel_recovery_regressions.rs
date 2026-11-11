type ColdTimerResult<T = ()> = Result<T, String>;

#[derive(Clone, Copy)]
struct ColdTimerExpectation {
    run: vb_core::RunId,
    step: vb_core::StepIdx,
    kind: crate::shard::types::PendingTimerKind,
    timeout: std::time::Duration,
}

fn short_recovery_timeout(
    workflow: vb_core::workflow::CompiledWorkflow,
    timeout: std::time::Duration,
) -> ColdTimerResult<vb_core::workflow::CompiledWorkflow> {
    let timeout_millis = i64::try_from(timeout.as_millis())
        .map_err(|error| format!("timeout milliseconds out of range: {error}"))?;
    let mut parts = workflow.to_parts();
    let mut changed = 0usize;
    for value in &mut parts.constants {
        if matches!(value, vb_core::ConstValue::I64(10)) {
            *value = vb_core::ConstValue::I64(timeout_millis);
            changed = changed.saturating_add(1);
        }
    }
    if changed != 1 {
        return Err(format!("expected one timer constant, changed {changed}"));
    }
    parts.digest = vb_core::WorkflowDigest::from_bytes([0; 32]);
    let bytes = postcard::to_allocvec(&parts).map_err(|error| format!("{error:?}"))?;
    parts.digest = vb_core::WorkflowDigest::from_bytes(*blake3::hash(&bytes).as_bytes());
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).map_err(|error| format!("{error:?}"))
}

fn cold_timer_runtime(
    journal: std::sync::Arc<vb_storage::FjallJournal>,
) -> crate::runtime::Runtime {
    crate::runtime::Runtime::new(
        std::num::NonZeroUsize::MIN,
        crate::shard::ShardConfig {
            policy: vb_core::RuntimePolicy::Strict,
            ..crate::shard::ShardConfig::default()
        },
        crate::journal::StorageRuntimeJournal::shared_strict(journal),
    )
}

fn create_suspended_run(
    directory: &tempfile::TempDir,
    run: vb_core::RunId,
    workflow: vb_core::workflow::CompiledWorkflow,
    scheduled_step: vb_core::StepIdx,
    kind: crate::shard::types::PendingTimerKind,
) -> ColdTimerResult {
    let journal = open_cold_journal(directory)?;
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[],
    )
    .map_err(|error| format!("{error:?}"))?;
    let mut runtime = cold_timer_runtime(std::sync::Arc::clone(&journal));
    runtime
        .submit_direct(run, workflow)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_cold_schedule_once(&journal, run, scheduled_step, kind)
}

fn open_cold_journal(
    directory: &tempfile::TempDir,
) -> ColdTimerResult<std::sync::Arc<vb_storage::FjallJournal>> {
    vb_storage::FjallJournal::open(directory.path(), None)
        .map(std::sync::Arc::new)
        .map_err(|error| format!("{error:?}"))
}

fn assert_cold_schedule_once(
    journal: &vb_storage::FjallJournal,
    run: vb_core::RunId,
    scheduled_step: vb_core::StepIdx,
    kind: crate::shard::types::PendingTimerKind,
) -> ColdTimerResult {
    let events = journal
        .events_for_run_full(run)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!((kind, event),
                (crate::shard::types::PendingTimerKind::Wait, vb_storage::JournalEvent::WaitScheduledEvent { step, .. })
                | (crate::shard::types::PendingTimerKind::Ask, vb_storage::JournalEvent::AskScheduledEvent { step, .. })
                if *step == scheduled_step))
            .count(),
        1
    );
    Ok(())
}

fn recovered_timer_snapshot(
    runtime: &crate::runtime::Runtime,
    run: vb_core::RunId,
    step: vb_core::StepIdx,
    kind: crate::shard::types::PendingTimerKind,
) -> ColdTimerResult<crate::shard::PendingTimerBoundarySnapshot> {
    let snapshot = runtime.pending_boundary_snapshot(8);
    let shard = snapshot.shards().first().ok_or("shard snapshot missing")?;
    shard
        .pending_timers()
        .iter()
        .copied()
        .find(|timer| timer.run_id() == run && timer.step() == step && timer.kind() == kind)
        .ok_or_else(|| "recovered timer snapshot missing".to_owned())
}

fn assert_timer_recovered(
    runtime: &mut crate::runtime::Runtime,
    journal: &vb_storage::FjallJournal,
    expected: ColdTimerExpectation,
) -> ColdTimerResult<std::time::Instant> {
    let (started, finished) = recover_without_journal_change(runtime, journal, expected.run)?;
    let timer = recovered_timer_snapshot(runtime, expected.run, expected.step, expected.kind)?;
    assert_timer_snapshot_policy(runtime, timer, expected, started, finished)?;
    assert_recovered_run_pc(runtime, expected.run, expected.step)?;
    Ok(finished)
}

fn recover_without_journal_change(
    runtime: &mut crate::runtime::Runtime,
    journal: &vb_storage::FjallJournal,
    run: vb_core::RunId,
) -> ColdTimerResult<(std::time::Instant, std::time::Instant)> {
    let before = journal
        .events_for_run_full(run)
        .map_err(|e| format!("{e:?}"))?;
    let started = std::time::Instant::now();
    runtime
        .recover_and_resume(run)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(runtime.tick_all(), Ok(true));
    let finished = std::time::Instant::now();
    let after = journal
        .events_for_run_full(run)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(after, before);
    Ok((started, finished))
}

fn assert_timer_snapshot_policy(
    runtime: &crate::runtime::Runtime,
    timer: crate::shard::PendingTimerBoundarySnapshot,
    expected: ColdTimerExpectation,
    started: std::time::Instant,
    finished: std::time::Instant,
) -> ColdTimerResult {
    let captured = runtime
        .capture_timer_entry(expected.run)
        .map_err(|e| format!("{e:?}"))?;
    assert_recovered_timer_identity(timer, captured, expected);
    let earliest = started
        .checked_add(expected.timeout)
        .ok_or("minimum deadline overflow")?;
    let latest = finished
        .checked_add(expected.timeout)
        .ok_or("maximum deadline overflow")?;
    assert!(timer.deadline() >= earliest && timer.deadline() <= latest);
    Ok(())
}

fn assert_recovered_timer_identity(
    timer: crate::shard::PendingTimerBoundarySnapshot,
    captured: crate::shard::timer_wheel::TimerEntry,
    expected: ColdTimerExpectation,
) {
    assert_eq!(timer.run_id(), expected.run);
    assert_eq!(timer.step(), expected.step);
    assert_eq!(timer.run_id(), captured.run);
    assert_eq!(timer.kind(), captured.kind);
    assert_eq!(timer.kind(), expected.kind);
    assert_eq!(timer.generation(), 1);
    assert_eq!(timer.generation(), captured.generation);
    assert_eq!(timer.deadline(), captured.deadline);
}

fn assert_recovered_run_pc(
    runtime: &crate::runtime::Runtime,
    run: vb_core::RunId,
    step: vb_core::StepIdx,
) -> ColdTimerResult {
    let snapshot = runtime
        .snapshot_run(run, 41)
        .map_err(|e| format!("{e:?}"))?;
    assert!(
        matches!(snapshot, crate::shard::types::InspectResponse::Found(found)
        if found.run == run && found.pc == step)
    );
    Ok(())
}

fn assert_timer_removed(runtime: &crate::runtime::Runtime, run: vb_core::RunId) {
    assert_eq!(
        runtime.pending_boundary_snapshot(8).pending_timer_count(),
        0
    );
    assert!(matches!(
        runtime.snapshot_run(run, 42),
        Ok(crate::shard::types::InspectResponse::NotFound { .. })
    ));
}

fn cold_recovery_case(
    workflow: vb_core::workflow::CompiledWorkflow,
    expected: ColdTimerExpectation,
) -> ColdTimerResult<Vec<vb_storage::JournalEvent>> {
    let directory = tempfile::tempdir().map_err(|e| format!("{e:?}"))?;
    let workflow = short_recovery_timeout(workflow, expected.timeout)?;
    create_suspended_run(
        &directory,
        expected.run,
        workflow,
        expected.step,
        expected.kind,
    )?;
    let journal = open_cold_journal(&directory)?;
    let mut runtime = cold_timer_runtime(std::sync::Arc::clone(&journal));
    let finished = assert_timer_recovered(&mut runtime, &journal, expected)?;
    assert_timer_due_by(&runtime, expected.run, finished)?;
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_timer_removed(&runtime, expected.run);
    assert_idle_tick_preserves_events(&mut runtime, &journal, expected.run)?;
    journal
        .events_for_run_full(expected.run)
        .map_err(|e| format!("{e:?}"))
}

fn cold_recovery_policy_case(
    workflow: vb_core::workflow::CompiledWorkflow,
    expected: ColdTimerExpectation,
) -> ColdTimerResult {
    let directory = tempfile::tempdir().map_err(|error| format!("{error:?}"))?;
    let workflow = short_recovery_timeout(workflow, expected.timeout)?;
    create_suspended_run(
        &directory,
        expected.run,
        workflow,
        expected.step,
        expected.kind,
    )?;
    let journal = open_cold_journal(&directory)?;
    let mut runtime = cold_timer_runtime(std::sync::Arc::clone(&journal));
    assert_timer_recovered(&mut runtime, &journal, expected)?;
    assert_eq!(
        runtime.pending_boundary_snapshot(8).pending_timer_count(),
        1
    );
    Ok(())
}

fn assert_timer_due_by(
    runtime: &crate::runtime::Runtime,
    run: vb_core::RunId,
    finished: std::time::Instant,
) -> ColdTimerResult {
    let timer = runtime
        .capture_timer_entry(run)
        .map_err(|e| format!("{e:?}"))?;
    if timer.deadline <= finished {
        Ok(())
    } else {
        Err(format!("zero-timeout timer remained future: {timer:?}"))
    }
}

fn assert_idle_tick_preserves_events(
    runtime: &mut crate::runtime::Runtime,
    journal: &vb_storage::FjallJournal,
    run: vb_core::RunId,
) -> ColdTimerResult {
    let before = journal
        .events_for_run_full(run)
        .map_err(|e| format!("{e:?}"))?;
    let before_bytes = postcard::to_allocvec(&before).map_err(|e| format!("{e:?}"))?;
    assert_eq!(runtime.tick_all(), Ok(true));
    let after = journal
        .events_for_run_full(run)
        .map_err(|e| format!("{e:?}"))?;
    let after_bytes = postcard::to_allocvec(&after).map_err(|e| format!("{e:?}"))?;
    assert_eq!(after_bytes, before_bytes);
    Ok(())
}
