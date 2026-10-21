//! Actual Runtime + Fjall shutdown ownership regressions.
#[path = "shutdown_fixtures.rs"]
mod fixtures;
use crate::RuntimeError;
use fixtures::*;
use vb_core::RunId;
use vb_storage::{EventSeq, JournalEvent};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn failed_shutdown_preserves_actual_pending_timer_authority() -> TestResult {
    let (_dir, journal, queue) = storage()?;
    let mut runtime = runtime(&journal, &queue);
    let run = RunId::new(700);
    runtime.submit_direct(run, waiting_workflow()?)?;
    assert!(runtime.tick_all()?);
    let before = runtime.pending_boundary_snapshot(64);
    assert_eq!(before.active_run_count(), 1);
    assert_eq!(before.pending_timer_count(), 1);
    let durable = journal.events_for_run(run)?;
    let first_queued_seq = match durable.last() {
        Some(event) => event
            .seq()
            .get()
            .checked_add(1)
            .ok_or("sequence overflow")?,
        None => 0,
    };
    journal.append_journaled(&JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(first_queued_seq),
        workflow: vb_core::WorkflowDigest::from_bytes([99; 32]),
    })?;
    let failure = match runtime.shutdown_graceful() {
        Err(failure) => failure,
        Ok(_) => return Err("conflicting durable event must fail".into()),
    };
    assert_eq!(failure.phase(), super::ShutdownPhase::JournalDrain);
    assert_eq!(failure.completed_shards(), 1);
    assert_eq!(failure.remaining(), &before);
    let source = failure.cause();
    let journal_error = match source {
        RuntimeError::StorageJournalAppend { source } => source.as_ref(),
        RuntimeError::AdmissionHeaderPersistenceFailed { source } => source.as_ref(),
        other => return Err(format!("expected storage journal error, got {other:?}").into()),
    };
    match journal_error {
        vb_storage::JournalError::DuplicateEvent { run: r, seq } => {
            assert_eq!(
                r.get(),
                run.get(),
                "DuplicateEvent must carry the colliding run"
            );
            assert_eq!(
                seq.get(),
                first_queued_seq,
                "DuplicateEvent must carry the colliding seq"
            );
        }
        other => return Err(format!("expected DuplicateEvent, got {other:?}").into()),
    }
    let after = runtime.pending_boundary_snapshot(64);
    assert_eq!(after.active_run_count(), 1);
    assert_eq!(
        after.pending_timer_count(),
        1,
        "a failed durable shutdown must not erase its pending timer authority"
    );
    assert_eq!(
        after
            .shards()
            .first()
            .ok_or("missing shard")?
            .pending_timers(),
        before
            .shards()
            .first()
            .ok_or("missing shard")?
            .pending_timers()
    );
    assert!(queue.pending_profile_counts()?.journaled > 0);
    Ok(())
}

#[test]
fn successful_shutdown_reports_suspended_authority_without_cancelling_run() -> TestResult {
    let (_dir, journal, queue) = storage()?;
    let mut runtime = runtime(&journal, &queue);
    let run = RunId::new(701);
    runtime.submit_direct(run, waiting_workflow()?)?;
    assert!(runtime.tick_all()?);
    let before = runtime.pending_boundary_snapshot(64);
    let queued = queue.pending_profile_counts()?.journaled;
    let report = runtime.shutdown_graceful()?;
    assert_eq!(report.commands_drained(), 0);
    assert_eq!(report.remaining(), &before);
    assert_eq!(report.timers_released(), 1);
    assert_eq!(report.journal().drained, queued);
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(runtime.pending_boundary_snapshot(64).active_run_count(), 1);
    assert_eq!(
        runtime.pending_boundary_snapshot(64).pending_timer_count(),
        0
    );
    let events = journal.events_for_run(run)?;
    assert!(!events.is_empty());
    assert!(events
        .iter()
        .any(|event| matches!(event, JournalEvent::WaitScheduledEvent { .. })));
    assert!(!events.iter().any(|event| matches!(
        event,
        JournalEvent::RunCancelled { .. } | JournalEvent::RunFinished { .. }
    )));
    assert_eq!(runtime.tick_all(), Ok(false));
    let repeated = runtime.shutdown_graceful()?;
    assert_eq!(repeated.commands_drained(), 0);
    assert_eq!(repeated.journal().drained, 0);
    assert_eq!(repeated.timers_released(), 0);
    assert_eq!(repeated.remaining().active_run_count(), 1);
    assert_eq!(journal.events_for_run(run)?, events);
    Ok(())
}

#[test]
fn full_command_queue_drains_exact_count_and_completes_real_runs() -> TestResult {
    let (_dir, journal, queue) = storage()?;
    let config = crate::shard::ShardConfig {
        command_queue_capacity: 2,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        ..Default::default()
    };
    let mut runtime = super::Runtime::new(
        std::num::NonZeroUsize::MIN,
        config,
        std::sync::Arc::new(crate::journal::QueuedStorageRuntimeJournal::journaled(
            journal.clone(),
            queue.clone(),
        )),
    );
    for run in [RunId::new(710), RunId::new(711)] {
        runtime.submit_direct(run, finished_workflow()?)?;
    }
    assert!(runtime
        .submit_direct(RunId::new(712), finished_workflow()?)
        .is_err());
    assert_eq!(
        runtime.pending_boundary_snapshot(64).command_queue_depth(),
        2
    );
    let report = runtime.shutdown_graceful()?;
    assert_eq!(report.commands_drained(), 2);
    assert_eq!(report.remaining().command_queue_depth(), 0);
    assert_eq!(report.remaining().active_run_count(), 0);
    for run in [RunId::new(710), RunId::new(711)] {
        assert!(journal
            .events_for_run(run)?
            .iter()
            .any(|event| matches!(event, JournalEvent::RunFinished { .. })));
    }
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    Ok(())
}

#[test]
fn explicit_shutdown_sentinel_reports_commands_it_did_not_process() -> TestResult {
    let (_dir, journal, queue) = storage()?;
    let mut runtime = runtime(&journal, &queue);
    let shard = runtime.shards.first().ok_or("missing shard")?;
    shard.enqueue(crate::shard::ShardCommand::Shutdown)?;
    runtime.inspect_run(RunId::new(720), 9)?;
    let report = runtime.shutdown_graceful()?;
    assert_eq!(report.commands_drained(), 1);
    assert_eq!(report.remaining().command_queue_depth(), 1);
    assert_eq!(
        runtime.pending_boundary_snapshot(64).command_queue_depth(),
        1
    );
    assert_eq!(
        runtime
            .shutdown_graceful()?
            .remaining()
            .command_queue_depth(),
        1
    );
    Ok(())
}

#[test]
fn suspended_action_report_keeps_actual_ticket_and_frame() -> TestResult {
    let (_dir, journal, queue) = storage()?;
    let mut runtime = runtime(&journal, &queue);
    let run = RunId::new(730);
    submit_action(&runtime, run)?;
    assert!(runtime.tick_all()?);
    let before = runtime.pending_boundary_snapshot(64);
    assert_eq!(before.pending_action_count(), 1);
    let report = runtime.shutdown_graceful()?;
    assert_eq!(report.remaining(), &before);
    let shard = report.remaining().shards().first().ok_or("missing shard")?;
    assert_eq!(shard.active_runs(), &[run]);
    let pending = shard.pending_actions().first().ok_or("missing action")?;
    assert_eq!(pending.run_id(), run);
    assert_eq!(pending.ticket().run, run);
    assert_eq!(
        runtime.pending_boundary_snapshot(64).pending_action_count(),
        1
    );
    assert!(!journal.events_for_run(run)?.iter().any(|event| matches!(
        event,
        JournalEvent::RunCancelled { .. } | JournalEvent::ActionAbandoned { .. }
    )));
    Ok(())
}

#[test]
fn report_bounds_details_without_losing_actual_totals() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = std::sync::Arc::new(vb_storage::FjallJournal::open(dir.path(), None)?);
    let queue = std::sync::Arc::new(vb_storage::JournalWriterQueue::new(
        2048,
        64,
        vb_storage::StorageLimits::DEFAULT,
    )?);
    let mut runtime = runtime(&journal, &queue);
    for id in 1..=65 {
        runtime.submit_direct(RunId::new(id), waiting_workflow()?)?;
        assert!(runtime.tick_all()?);
    }
    let report = runtime.shutdown_graceful()?;
    assert_eq!(report.remaining().active_run_count(), 65);
    assert_eq!(report.remaining().pending_timer_count(), 65);
    assert_eq!(report.timers_released(), 65);
    assert!(report.remaining().truncated());
    let shard = report.remaining().shards().first().ok_or("missing shard")?;
    assert_eq!(shard.active_runs().len(), 64);
    assert_eq!(shard.pending_timers().len(), 64);
    assert_eq!(
        runtime.pending_boundary_snapshot(128).active_run_count(),
        65
    );
    Ok(())
}

#[path = "shutdown_failure_tests.rs"]
mod failure_tests;
