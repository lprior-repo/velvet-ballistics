//! Tests for runtime journal adapters.

use super::{
    RuntimeJournal, RuntimeJournalConfig, RuntimeJournalEvent,
};
use crate::journal_storage::{QueuedStorageRuntimeJournal, StorageRuntimeJournal};
use crate::runtime::Runtime;
use crate::shard::ShardConfig;
use std::num::NonZeroUsize;
use std::sync::Arc;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_storage::{
    DurabilityProfile, EventSeq, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits,
};

fn single_finish_workflow(workflow: WorkflowDigest) -> Result<CompiledWorkflow, String> {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("single_finish"),
        digest: workflow,
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

fn temp_journal() -> Result<(tempfile::TempDir, Arc<FjallJournal>), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal = FjallJournal::open(dir.path(), None).map_err(|error| error.to_string())?;
    Ok((dir, Arc::new(journal)))
}

fn journal_queue(
    capacity: usize,
    batch_size: usize,
) -> Result<Arc<JournalWriterQueue>, String> {
    JournalWriterQueue::new(capacity, batch_size, StorageLimits::DEFAULT)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

fn require_ok<T>(result: Result<T, String>, context: &'static str) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            assert!(false, "{context}: {error}");
            None
        }
    }
}

#[test]
fn storage_runtime_journal_maps_lifecycle_events_in_sequence() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = RunId::new(41);
    let workflow = WorkflowDigest::from_bytes([7; 32]);

    assert_eq!(
        adapter.append(RuntimeJournalEvent::RunSubmitted { run, workflow }),
        Ok(())
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::RunFinished {
            run,
            result: SlotIdx::new(3),
        }),
        Ok(())
    );

    let Some(events) = require_ok(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string()),
        "events read",
    ) else {
        return;
    };
    assert_eq!(
        events,
        vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(1),
                result: SlotIdx::new(3),
            },
        ]
    );
}

#[test]
fn storage_runtime_journal_maps_cancelled_and_failed_events() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = RunId::new(42);
    let workflow = WorkflowDigest::from_bytes([8; 32]);

    assert_eq!(
        adapter.append(RuntimeJournalEvent::RunSubmitted { run, workflow }),
        Ok(())
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::RunCancelled { run }),
        Ok(())
    );

    let Some(events) = require_ok(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string()),
        "cancelled events read",
    ) else {
        return;
    };
    assert_eq!(
        events,
        vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            },
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(1),
            },
        ]
    );

    let failed_run = RunId::new(43);
    assert_eq!(
        adapter.append(RuntimeJournalEvent::RunFailed { run: failed_run }),
        Ok(())
    );
    let Some(failed_events) = require_ok(
        journal
            .events_for_run(failed_run)
            .map_err(|error| error.to_string()),
        "failed events read",
    ) else {
        return;
    };
    assert_eq!(
        failed_events,
        vec![JournalEvent::RunFailedEvent {
            run: failed_run,
            seq: EventSeq::new(0),
        }]
    );
}

#[test]
fn storage_runtime_journal_maps_action_wait_and_ask_events() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = RunId::new(44);

    assert_eq!(
        adapter.append(RuntimeJournalEvent::ActionScheduled {
            run,
            step: StepIdx::new(1),
            action: ActionId::new(2),
        }),
        Ok(())
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::ActionCompleted {
            run,
            step: StepIdx::new(1),
            action: ActionId::new(2),
        }),
        Ok(())
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::WaitScheduled {
            run,
            step: StepIdx::new(3),
        }),
        Ok(())
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::WaitResolved {
            run,
            step: StepIdx::new(3),
        }),
        Ok(())
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::AskScheduled {
            run,
            step: StepIdx::new(4),
        }),
        Ok(())
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::AskAnswered {
            run,
            step: StepIdx::new(4),
            slot: SlotIdx::new(5),
        }),
        Ok(())
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::SlotWritten {
            run,
            slot: SlotIdx::new(5),
        }),
        Ok(())
    );

    let Some(events) = require_ok(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string()),
        "action/wait/ask events read",
    ) else {
        return;
    };
    assert_eq!(
        events,
        vec![
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(1),
                action: ActionId::new(2),
            },
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(1),
                action: ActionId::new(2),
            },
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(3),
            },
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(3),
            },
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(4),
            },
            JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(4),
            },
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(6),
                slot: SlotIdx::new(5),
            },
        ]
    );
}

#[test]
fn queued_storage_runtime_journal_flushes_mapped_events_to_fjall() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(4, 2), "journal queue opens") else {
        return;
    };
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue);
    let run = RunId::new(45);
    let workflow = WorkflowDigest::from_bytes([9; 32]);

    assert_eq!(
        adapter.append(RuntimeJournalEvent::RunSubmitted { run, workflow }),
        Ok(())
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::ActionScheduled {
            run,
            step: StepIdx::new(1),
            action: ActionId::new(2),
        }),
        Ok(())
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::RunFinished {
            run,
            result: SlotIdx::new(3),
        }),
        Ok(())
    );

    assert!(matches!(journal.events_for_run(run), Ok(events) if events.is_empty()));
    assert!(
        matches!(adapter.flush_batch(), Ok(report) if report.drained == 2 && report.written == 2)
    );
    assert!(
        matches!(adapter.flush_batch(), Ok(report) if report.drained == 1 && report.written == 1)
    );

    let Some(events) = require_ok(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string()),
        "queued events read",
    ) else {
        return;
    };
    assert_eq!(
        events,
        vec![
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            },
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(1),
                action: ActionId::new(2),
            },
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(2),
                result: SlotIdx::new(3),
            },
        ]
    );
}

#[test]
fn runtime_journal_config_maps_profiles_to_volatile_journaled_and_strict_behavior() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(volatile_queue) = require_ok(journal_queue(4, 4), "volatile queue opens") else {
        return;
    };
    let run = RunId::new(47);
    let workflow = WorkflowDigest::from_bytes([10; 32]);

    let volatile = RuntimeJournalConfig::new(DurabilityProfile::Volatile)
        .shared_journal(journal.clone(), volatile_queue.clone());
    assert_eq!(
        volatile.append(RuntimeJournalEvent::RunSubmitted { run, workflow }),
        Ok(())
    );
    assert!(matches!(journal.events_for_run(run), Ok(events) if events.is_empty()));
    assert!(matches!(
        volatile_queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));

    let Some(journaled_queue) = require_ok(journal_queue(4, 4), "journaled queue opens") else {
        return;
    };
    let journaled = RuntimeJournalConfig::new(DurabilityProfile::Journaled)
        .shared_journal(journal.clone(), journaled_queue.clone());
    assert_eq!(
        journaled.append(RuntimeJournalEvent::RunCancelled { run }),
        Ok(())
    );
    assert!(matches!(
        journaled_queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 1 && counts.strict == 0
    ));

    let Some(strict_queue) = require_ok(journal_queue(4, 4), "strict queue opens") else {
        return;
    };
    let strict_run = RunId::new(48);
    let strict = RuntimeJournalConfig::new(DurabilityProfile::Strict)
        .shared_journal(journal.clone(), strict_queue.clone());
    assert_eq!(
        strict.append(RuntimeJournalEvent::RunFailed { run: strict_run }),
        Ok(())
    );
    assert!(matches!(
        strict_queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));
    assert!(matches!(
        journal.events_for_run(strict_run),
        Ok(events) if matches!(events.as_slice(), [JournalEvent::RunFailedEvent { seq, .. }] if *seq == EventSeq::new(0))
    ));
}

#[test]
fn queued_storage_runtime_journal_drain_all_flushes_past_batch_size() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(8, 2), "journal queue opens") else {
        return;
    };
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue.clone());
    let run = RunId::new(48);
    let workflow = WorkflowDigest::from_bytes([11; 32]);

    assert_eq!(
        adapter.append(RuntimeJournalEvent::RunSubmitted { run, workflow }),
        Ok(())
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::RunCancelled { run }),
        Ok(())
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::RunFailed { run }),
        Ok(())
    );
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 3 && counts.strict == 0
    ));

    assert!(matches!(
        adapter.drain_all(),
        Ok(report) if report.drained == 3 && report.written == 3
    ));
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));
    assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 3));
}

#[test]
fn runtime_shutdown_graceful_drains_owned_queued_journal() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(4, 1), "journal queue opens") else {
        return;
    };
    let runtime_journal = Arc::new(QueuedStorageRuntimeJournal::journaled(
        journal.clone(),
        queue.clone(),
    ));
    let run = RunId::new(49);
    let workflow = WorkflowDigest::from_bytes([12; 32]);
    let Some(shard_count) = NonZeroUsize::new(1) else {
        assert!(false, "invalid shard count");
        return;
    };
    let runtime =
        Runtime::new_with_journal(shard_count, ShardConfig::default(), runtime_journal);

    let Some(compiled) = require_ok(single_finish_workflow(workflow), "workflow compiles")
    else {
        return;
    };
    assert_eq!(runtime.submit_direct(run, compiled), Ok(()));
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));

    let mut runtime = runtime;
    assert_eq!(runtime.tick_all(), Ok(true));
    // Evidence chain adds StepStarted + StepSucceeded per step.
    // Single Finish step: RunSubmitted + StepStarted(0) + StepSucceeded(0) + RunFinished
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(ref c) if c.journaled >= 3 && c.strict == 0
    ));
    assert_eq!(runtime.shutdown_graceful(), Ok(()));
    assert!(matches!(
        queue.pending_profile_counts(),
        Ok(counts) if counts.journaled == 0 && counts.strict == 0
    ));
    // At minimum RunSubmitted + StepSucceeded + RunFinished stored after drain
    assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() >= 3));
}

#[test]
fn queued_storage_runtime_journal_maps_queue_full_to_runtime_error() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let Some(queue) = require_ok(journal_queue(1, 1), "journal queue opens") else {
        return;
    };
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue);
    let run = RunId::new(46);

    assert_eq!(
        adapter.append(RuntimeJournalEvent::RunCancelled { run }),
        Ok(())
    );
    assert!(matches!(
        adapter.append(RuntimeJournalEvent::RunFailed { run }),
        Err(crate::RuntimeError::StorageJournalAppendFailed)
    ));
    assert!(
        matches!(adapter.flush_batch(), Ok(report) if report.drained == 1 && report.written == 1)
    );
    assert_eq!(
        adapter.append(RuntimeJournalEvent::RunFailed { run }),
        Ok(())
    );
    assert!(
        matches!(adapter.flush_batch(), Ok(report) if report.drained == 1 && report.written == 1)
    );

    let Some(events) = require_ok(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string()),
        "queue-full events read",
    ) else {
        return;
    };
    assert_eq!(
        events,
        vec![
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(0),
            },
            JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(1),
            },
        ]
    );
}
