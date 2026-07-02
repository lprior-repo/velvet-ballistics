use super::{
    QueuedStorageRuntimeJournal, RuntimeJournal, RuntimeJournalConfig, RuntimeJournalEvent,
    StorageRuntimeJournal, VolatileRuntimeJournal,
};
use crate::runtime::Runtime;
use crate::shard::ShardConfig;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};
use vb_core::value::Taint;
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
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

fn temp_journal() -> Result<(tempfile::TempDir, Arc<FjallJournal>), String> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&base).map_err(|error| error.to_string())?;
    let dir = tempfile::Builder::new()
        .prefix("vb-runtime-journal-")
        .tempdir_in(base)
        .map_err(|error| error.to_string())?;
    let journal = FjallJournal::open(dir.path(), None).map_err(|error| error.to_string())?;
    Ok((dir, Arc::new(journal)))
}

fn journal_queue(capacity: usize, batch_size: usize) -> Result<Arc<JournalWriterQueue>, String> {
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

fn fabricated_workflow_id_from_digest_for_test(digest: WorkflowDigest) -> WorkflowId {
    let [b0, b1, b2, b3, ..] = digest.as_bytes();
    WorkflowId::new(u32::from_be_bytes([b0, b1, b2, b3]))
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
        adapter.append_sequenced(
            RuntimeJournalEvent::RunSubmitted { run, workflow },
            EventSeq::new(0),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunFinished {
                run,
                result: SlotIdx::new(3),
            },
            EventSeq::new(1),
        ),
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
                attempt: 1,
            },
        ]
    );
}

#[test]
fn queued_runtime_admission_flush_does_not_fabricate_recovery_indexes() -> Result<(), String> {
    let (_dir, journal) = temp_journal()?;
    let queue = journal_queue(4, 2)?;
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue);
    let run = RunId::new(41_010);
    let workflow = WorkflowDigest::from_bytes([0x12; 32]);
    let seq = EventSeq::new(7);

    assert_eq!(
        adapter.append_sequenced(RuntimeJournalEvent::RunSubmitted { run, workflow }, seq),
        Ok(())
    );
    assert!(matches!(adapter.flush_batch(), Ok(report) if report.drained == 1 && report.written == 1));

    assert_eq!(
        journal.run_header(run).map_err(|error| error.to_string())?,
        None,
        "RunAccepted flush must not fabricate a run header from digest/seq"
    );

    let workflow_id = fabricated_workflow_id_from_digest_for_test(workflow);
    let status_key = vb_storage::keys::index_status_key(
        vb_storage::IndexStatusState::Submitted,
        seq.get(),
        run,
    )
    .map_err(|error| error.to_string())?;
    let workflow_key = vb_storage::keys::index_workflow_key(workflow_id, run)
        .map_err(|error| error.to_string())?;
    assert!(
        !journal
            .has_status_index_entry(status_key)
            .map_err(|error| error.to_string())?,
        "RunAccepted flush must not fabricate a status index timestamp from seq"
    );
    assert!(
        !journal
            .has_workflow_index_entry(workflow_key)
            .map_err(|error| error.to_string())?,
        "RunAccepted flush must not fabricate a workflow id from digest bytes"
    );
    Ok(())
}

#[test]
fn storage_runtime_journal_rejects_unsequenced_append() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = RunId::new(46);
    let workflow = WorkflowDigest::from_bytes([6; 32]);

    assert!(matches!(
        adapter.append(RuntimeJournalEvent::RunSubmitted { run, workflow }),
        Err(crate::RuntimeError::UnsupportedOperation {
            operation: "unsequenced_storage_journal_append"
        })
    ));
    assert!(matches!(journal.events_for_run(run), Ok(events) if events.is_empty()));
}

#[test]
fn storage_runtime_journal_maps_run_admission_event() {
    let Some((_dir, journal)) = require_ok(temp_journal(), "temp journal opens") else {
        return;
    };
    let adapter = StorageRuntimeJournal::journaled(journal.clone());
    let run = RunId::new(45);
    let workflow = WorkflowDigest::from_bytes([9; 32]);
    let admission = crate::admission::RunAdmission::new(
        workflow,
        run,
        vb_core::capability::CapabilitySet::empty(),
        vb_core::policy::RuntimePolicy::Relaxed,
    );

    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunAdmission { admission },
            EventSeq::new(0),
        ),
        Ok(())
    );

    let Some(events) = require_ok(
        journal
            .events_for_run(run)
            .map_err(|error| error.to_string()),
        "admission events read",
    ) else {
        return;
    };
    assert_eq!(
        events,
        vec![vb_storage::JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(0),
            artifact_digest: workflow,
            granted_capabilities: vb_core::capability::CapabilitySet::empty(),
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        }]
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
        adapter.append_sequenced(
            RuntimeJournalEvent::RunSubmitted { run, workflow },
            EventSeq::new(0),
        ),
        Ok(())
    );
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunCancelled { run, reason: None },
            EventSeq::new(1),
        ),
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
                attempt: 1,
                reason: None,
            },
        ]
    );

    let failed_run = RunId::new(43);
    assert_eq!(
        adapter.append_sequenced(
            RuntimeJournalEvent::RunFailed { run: failed_run },
            EventSeq::new(0),
        ),
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
            attempt: 1,
        }]
    );
}
