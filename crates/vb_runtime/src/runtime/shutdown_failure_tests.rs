//! Actual runtime/storage bridge; the one-shot failure is injected at its port.
use super::fixtures::*;
use crate::journal::{QueuedStorageRuntimeJournal, RuntimeJournal, RuntimeJournalEvent};
use crate::runtime::{Runtime, ShutdownPhase};
use crate::{RuntimeError, RuntimeResult};
use std::num::NonZeroUsize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use vb_core::RunId;
use vb_storage::{EventSeq, JournalEvent, JournalWriterFlushReport};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct FailShutdownPortOnce {
    inner: QueuedStorageRuntimeJournal,
    fail: AtomicBool,
}

impl RuntimeJournal for FailShutdownPortOnce {
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        self.inner.append(event)
    }
    fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()> {
        self.inner.append_sequenced(event, seq)
    }
    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        seq: EventSeq,
    ) -> RuntimeResult<()> {
        self.inner.append_sequenced_batch(events, seq)
    }
    fn probe(&self) -> RuntimeResult<()> {
        self.inner.probe()
    }
    fn storage_journal(&self) -> Option<Arc<vb_storage::FjallJournal>> {
        self.inner.storage_journal()
    }
    fn convert_to_storage_event(
        &self,
        event: RuntimeJournalEvent,
        seq: EventSeq,
    ) -> RuntimeResult<JournalEvent> {
        self.inner.convert_to_storage_event(event, seq)
    }
    fn drain_for_shutdown(&self) -> RuntimeResult<JournalWriterFlushReport> {
        if self.fail.swap(false, Ordering::SeqCst) {
            return Err(RuntimeError::from(
                vb_storage::JournalError::StrictDurabilityFailed,
            ));
        }
        self.inner.drain_for_shutdown()
    }
}

#[test]
fn journal_port_failure_preserves_timer_then_retry_drains_actual_fjall_queue() -> TestResult {
    let (_dir, journal, queue) = storage()?;
    let port = Arc::new(FailShutdownPortOnce {
        inner: QueuedStorageRuntimeJournal::journaled(journal.clone(), queue.clone()),
        fail: AtomicBool::new(true),
    });
    let config = crate::shard::ShardConfig {
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        ..Default::default()
    };
    let mut runtime = Runtime::new(NonZeroUsize::MIN, config, port);
    let run = RunId::new(740);
    runtime.submit_direct(run, waiting_workflow()?)?;
    assert!(runtime.tick_all()?);
    let before = runtime.pending_boundary_snapshot(64);
    let queued = queue.pending_profile_counts()?.journaled;
    assert!(queued > 0);
    let failure = match runtime.shutdown_graceful() {
        Err(failure) => failure,
        Ok(_) => return Err("expected injected journal port failure".into()),
    };
    assert_eq!(failure.phase(), ShutdownPhase::JournalDrain);
    assert_eq!(
        failure.cause(),
        &RuntimeError::from(vb_storage::JournalError::StrictDurabilityFailed)
    );
    assert_eq!(failure.remaining(), &before);
    assert_eq!(runtime.pending_boundary_snapshot(64), before);
    assert_eq!(queue.pending_profile_counts()?.journaled, queued);
    assert_eq!(
        runtime.inspect_run(run, 1),
        Err(RuntimeError::ShutdownInProgress)
    );
    let report = runtime.shutdown_graceful()?;
    assert_eq!(report.remaining(), &before);
    assert_eq!(report.timers_released(), 1);
    assert_eq!(report.journal().drained, queued);
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    let events = journal.events_for_run(run)?;
    assert_eq!(events.len(), queued);
    assert!(!events.iter().any(|event| matches!(
        event,
        JournalEvent::RunCancelled { .. } | JournalEvent::RunFinished { .. }
    )));
    assert_eq!(runtime.shutdown_graceful()?.journal().drained, 0);
    assert_eq!(journal.events_for_run(run)?, events);
    Ok(())
}

#[test]
fn full_journal_queue_is_durably_drained_after_nonwriting_command() -> TestResult {
    let (_dir, journal, queue) = storage()?;
    let mut runtime = runtime(&journal, &queue);
    let run = RunId::new(750);
    runtime.submit_direct(run, waiting_workflow()?)?;
    assert!(runtime.tick_all()?);
    for seq in 0..128 {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(751),
            seq: EventSeq::new(seq),
            workflow: vb_core::WorkflowDigest::from_bytes([75; 32]),
        };
        match queue.enqueue_journaled(event) {
            Ok(()) => {}
            Err(vb_storage::JournalError::QueueFull) => break,
            Err(error) => return Err(error.into()),
        }
    }
    assert_eq!(queue.pending_profile_counts()?.journaled, 128);
    runtime.inspect_run(run, 75)?;
    let report = runtime.shutdown_graceful()?;
    assert_eq!(report.commands_drained(), 1);
    assert_eq!(report.journal().drained, 128);
    assert_eq!(report.remaining().active_run_count(), 1);
    assert_eq!(report.remaining().pending_timer_count(), 1);
    assert_eq!(queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(
        journal
            .events_for_run(run)?
            .len()
            .saturating_add(journal.events_for_run(RunId::new(751))?.len()),
        128
    );
    Ok(())
}

#[test]
fn shard_failure_reports_completed_prefix_and_keeps_other_timer_authorities() -> TestResult {
    let (_dir, journal, queue) = storage()?;
    let config = crate::shard::ShardConfig {
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        ..Default::default()
    };
    let mut runtime = Runtime::new(
        NonZeroUsize::new(2).ok_or("invalid shard count")?,
        config,
        Arc::new(QueuedStorageRuntimeJournal::journaled(
            journal.clone(),
            queue.clone(),
        )),
    );
    runtime.submit_direct(RunId::new(760), waiting_workflow()?)?;
    assert!(runtime.tick_all()?);
    runtime.resume_run(RunId::new(761))?;
    let failure = match runtime.shutdown_graceful() {
        Err(failure) => failure,
        Ok(_) => return Err("missing run must fail shard drain".into()),
    };
    assert_eq!(failure.phase(), ShutdownPhase::ShardDrain { shard: 1 });
    assert_eq!(failure.completed_shards(), 1);
    assert_eq!(failure.cause(), &RuntimeError::RunNotFound);
    assert_eq!(failure.remaining().active_run_count(), 1);
    assert_eq!(failure.remaining().pending_timer_count(), 1);
    assert_eq!(failure.remaining().command_queue_depth(), 0);
    assert_eq!(runtime.pending_boundary_snapshot(64), *failure.remaining());
    assert!(queue.pending_profile_counts()?.journaled > 0);
    Ok(())
}
