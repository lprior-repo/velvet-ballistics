//! Behavioral tests for RuntimeJournal::append_policy_decision_strict.
//!
//! Validates: full DecisionRecord preservation through storage and cold reopen,
//! queued prefix lifecycle drain before strict decision, and default trait
//! behavior on Volatile/Noop implementations.

use std::sync::Arc;

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx, WorkflowDigest};
use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, DecisionRecord, InfluenceState,
    PinnedAdmissionPolicy, PolicyDigest, PolicyLayer,
};
use vb_core::value::Taint;
use vb_storage::{
    DurabilityProfile, EventSeq, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits,
};

use super::{
    NoopRuntimeJournal, QueuedStorageRuntimeJournal, RuntimeJournal,
    RuntimeJournalEvent, StorageRuntimeJournal, VolatileRuntimeJournal,
};
use crate::{RuntimeError, RuntimeResult};

fn test_action_ticket(run: RunId, step: StepIdx, seq: SeqNo, action: ActionId) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq,
        action,
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
        capacity: 10,
    }
}

fn test_decision_record(run: RunId, policy_id: u8) -> DecisionRecord {
    let ticket = test_action_ticket(run, StepIdx::new(1), SeqNo::new(1), ActionId::new(0));
    let digest = PolicyDigest::from_bytes([
        0xAB, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, policy_id,
    ]);
    DecisionRecord {
        ticket,
        admission: PinnedAdmissionPolicy::Artifact(digest),
        current_live: CurrentPolicyIdentity::Unconfigured,
        input_digest: PolicyDigest::from_bytes([
            0xCD, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0x01,
        ]),
        taint: Taint::Clean,
        outcome: DecisionCode::Allowed,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

fn make_temp_journal() -> Result<(tempfile::TempDir, Arc<FjallJournal>), Box<dyn std::error::Error>> {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&base)?;
    let dir = tempfile::Builder::new().prefix("vb-pd-test-").tempdir_in(&base)?;
    let journal = Arc::new(FjallJournal::open(dir.path(), None)?);
    Ok((dir, journal))
}

fn make_journal_queue(capacity: usize) -> Result<Arc<JournalWriterQueue>, Box<dyn std::error::Error>> {
    JournalWriterQueue::new(capacity, 2, StorageLimits::DEFAULT)
        .map(Arc::new)
        .map_err(|e| e as Box<dyn std::error::Error>)
}

/// StorageRuntimeJournal::shared_journaled must write the full DecisionRecord
/// through the strict port and survive a cold reopen with exact equality.
#[test]
fn journaled_full_decision_event_cold_reopen() -> Result<(), Box<dyn std::error::Error>> {
    let (dir, journal) = make_temp_journal()?;
    let adapter = StorageRuntimeJournal::shared_journaled(journal.clone());

    let run = RunId::new(1001);
    let seq = EventSeq::new(0);
    let original = test_decision_record(run, 0x42);

    let result = adapter.append_policy_decision_strict(original.clone(), seq);
    result?;

    // Drop all original journal references before reopen.
    drop(adapter);
    drop(journal);

    let reopened = Arc::new(FjallJournal::open(dir.path(), None)?);
    let events = reopened.events_for_run(run)?;

    assert_eq!(events.len(), 1, "expected exactly one decision event after reopen");

    let event = events.first().expect("events vec is non-empty");
    match event {
        JournalEvent::PolicyDecisionEvent {
            run: ev_run,
            seq: ev_seq,
            decision: ev_decision,
        } => {
            assert_eq!(*ev_run, run, "run mismatch in decision event");
            assert_eq!(*ev_seq, seq, "seq mismatch in decision event");
            assert_eq!(*ev_decision, original, "decision did not round-trip exactly");
        }
        other => panic!("unexpected journal event type after reopen: {other:?}"),
    }

    drop(reopened);
    drop(dir);
    Ok(())
}

/// Queued journal must drain any prior lifecycle prefix before the strict
/// decision write, then the queue must remain accepting for a later
/// ordinary append.
#[test]
fn queued_prefix_strict_decision_later_drain_exact_events() -> Result<(), Box<dyn std::error::Error>> {
    let (dir, journal) = make_temp_journal()?;
    let queue = make_journal_queue(16)?;
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue);

    let run = RunId::new(1002);
    let prefix_seq = EventSeq::new(0);
    let decision_seq = EventSeq::new(1);
    let later_seq = EventSeq::new(2);
    let original = test_decision_record(run, 0xEE);

    // Enqueue a lifecycle prefix that is not yet durable.
    adapter.append_sequenced(
        RuntimeJournalEvent::RunSubmitted {
            run,
            workflow: WorkflowDigest::from_bytes([0x11; 32]),
        },
        prefix_seq,
    )?;

    // Strict decision must force a drain of the prefix, write the decision,
    // and sync. Queue remains accepting after.
    adapter.append_policy_decision_strict(original.clone(), decision_seq)?;

    // Later ordinary append is still accepted (queued, not yet durable).
    adapter.append_sequenced(
        RuntimeJournalEvent::RunFinished {
            run,
            result: vb_core::ids::SlotIdx::new(0),
            attempt: 1,
        },
        later_seq,
    )?;

    // Force all queued writes to drain so we can inspect the full journal.
    adapter.drain_for_shutdown()?;

    // Verify all three events are present in exact sequence order.
    let events = journal.events_for_run(run)?;
    assert_eq!(events.len(), 3, "expected 3 events after full drain");

    let first = events.first().expect("events vec is non-empty");
    match first {
        JournalEvent::RunAccepted {
            run: r, seq: s, ..
        } => {
            assert_eq!(*r, run);
            assert_eq!(*s, prefix_seq);
        }
        other => panic!("prefix event not RunAccepted: {other:?}"),
    }

    let second = events.get(1).expect("second event exists");
    match second {
        JournalEvent::PolicyDecisionEvent {
            run: r, seq: s, decision: ev
        } => {
            assert_eq!(*r, run);
            assert_eq!(*s, decision_seq);
            assert_eq!(*ev, original, "decision not preserved exactly");
        }
        other => panic!("second event not PolicyDecisionEvent: {other:?}"),
    }

    let third = events.get(2).expect("third event exists");
    match third {
        JournalEvent::RunFinished {
            run: r, seq: s, ..
        } => {
            assert_eq!(*r, run);
            assert_eq!(*s, later_seq);
        }
        other => panic!("third event not RunFinished: {other:?}"),
    }

    drop(adapter);
    drop(journal);
    drop(queue);
    drop(dir);
    Ok(())
}

/// VolatileRuntimeJournal uses the trait default: exact
/// UnsupportedOperation("append_policy_decision_strict").
#[test]
fn volatile_journal_default_unsupported_operation() -> Result<(), Box<dyn std::error::Error>> {
    let journal = VolatileRuntimeJournal::shared();
    let run = RunId::new(1003);
    let decision = test_decision_record(run, 0x00);

    let result = journal.append_policy_decision_strict(decision, EventSeq::new(0));
    match result {
        Err(RuntimeError::UnsupportedOperation { operation }) => {
            assert_eq!(operation, "append_policy_decision_strict");
        }
        other => panic!(
            "expected UnsupportedOperation, got: {other:?}"
        ),
    }
    Ok(())
}

/// NoopRuntimeJournal uses the trait default: exact
/// UnsupportedOperation("append_policy_decision_strict").
#[test]
fn noop_journal_default_unsupported_operation() -> Result<(), Box<dyn std::error::Error>> {
    let journal = NoopRuntimeJournal::shared();
    let run = RunId::new(1004);
    let decision = test_decision_record(run, 0x00);

    let result = journal.append_policy_decision_strict(decision, EventSeq::new(0));
    match result {
        Err(RuntimeError::UnsupportedOperation { operation }) => {
            assert_eq!(operation, "append_policy_decision_strict");
        }
        other => panic!(
            "expected UnsupportedOperation, got: {other:?}"
        ),
    }
    Ok(())
}