//! Bounded behavioral tests for RuntimeJournal::append_policy_decision_strict.
//!
//! Validates: full DecisionRecord preservation through storage and drop/reopen,
//! queued prefix lifecycle drain before strict decision, and default trait
//! behavior on Volatile/Noop implementations.

use std::sync::Arc;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy,
    PolicyDigest, PolicyLayer,
};
use vb_core::action::ActionTicket;
use vb_core::Taint;
use vb_storage::{
    DurabilityProfile, EventSeq, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits,
};

use super::{
    QueuedStorageRuntimeJournal, RuntimeError, RuntimeResult, StorageRuntimeJournal,
    VolatileRuntimeJournal, NoopRuntimeJournal, RuntimeJournal,
};

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
    let digest = PolicyDigest::from_bytes([0xAB, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, policy_id]);
    DecisionRecord {
        ticket,
        admission: PinnedAdmissionPolicy::Artifact(digest),
        current_live: CurrentPolicyIdentity::Unconfigured,
        input_digest: PolicyDigest::from_bytes([0xCD, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01]),
        taint: Taint::Clean,
        outcome: DecisionCode::Allowed,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

fn temp_journal_path() -> std::io::Result<(tempfile::TempDir, Arc<FjallJournal>)> {
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&base)?;
    let dir = tempfile::Builder::new()
        .prefix("vb-pd-test-")
        .tempdir_in(&base)?;
    let journal = Arc::new(FjallJournal::open(dir.path(), None)?);
    Ok((dir, journal))
}

fn journal_queue(capacity: usize) -> Result<Arc<JournalWriterQueue>, String> {
    JournalWriterQueue::new(capacity, 2, StorageLimits::DEFAULT)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

fn require_ok<T>(result: Result<T, RuntimeError>, context: &str) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            assert!(false, "{context}: {error}");
            None
        }
    }
}

/// StorageRuntimeJournal::shared_journaled must write the full DecisionRecord
/// through the strict port, not a reduced or synthetic shape.
#[test]
fn storage_journaled_writes_full_decision_record_strict_port() {
    let (dir, journal) = match temp_journal_path() {
        Ok(j) => j,
        Err(error) => {
            assert!(false, "temp journal: {error}");
            return;
        }
    };
    let adapter = StorageRuntimeJournal::shared_journaled(journal.clone());
    let run = RunId::new(1001);
    let seq = EventSeq::new(0);
    let decision = test_decision_record(run, 0x42);

    let result = adapter.append_policy_decision_strict(decision, seq);
    let _ = require_ok(result, "append_policy_decision_strict");

    let events = match journal.events_for_run(run) {
        Ok(events) => events,
        Err(error) => {
            assert!(false, "read events: {error}");
            return;
        }
    };

    assert_eq!(events.len(), 1, "expected exactly one decision event");
    match &events[0] {
        JournalEvent::PolicyDecisionEvent { run: ev_run, seq: ev_seq, decision: ev_decision } => {
            assert_eq!(*ev_run, run, "run mismatch in decision event");
            assert_eq!(*ev_seq, seq, "seq mismatch in decision event");
            assert_eq!(ev_decision.ticket.run, run, "ticket.run mismatch");
            assert_eq!(ev_decision.ticket.step, StepIdx::new(1), "ticket.step mismatch");
            assert_eq!(ev_decision.ticket.seq, SeqNo::new(1), "ticket.seq mismatch");
            assert_eq!(ev_decision.ticket.action, ActionId::new(0), "ticket.action mismatch");
            assert_eq!(ev_decision.admission, PinnedAdmissionPolicy::Artifact(
                PolicyDigest::from_bytes([0xAB, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x42])));
            assert_eq!(ev_decision.input_digest, PolicyDigest::from_bytes([0xCD, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01]));
            assert_eq!(ev_decision.taint, Taint::Clean);
            assert_eq!(ev_decision.outcome, DecisionCode::Allowed);
            assert_eq!(ev_decision.layer, PolicyLayer::Live);
            assert_eq!(ev_decision.emergency_influence, InfluenceState::Inactive);
            assert_eq!(ev_decision.compat_influence, InfluenceState::NotEvaluated);
        }
        other => {
            assert!(false, "unexpected journal event type: {other:?}");
        }
    }
    drop(dir);
}

/// Round-trip durability: drop the journaled adapter, reopen the journal,
/// and verify the full DecisionRecord round-trips with exact equality.
#[test]
fn storage_journaled_decision_roundtrips_after_reopen() {
    let (dir, journal) = match temp_journal_path() {
        Ok(j) => j,
        Err(error) => {
            assert!(false, "temp journal: {error}");
            return;
        }
    };
    let adapter = StorageRuntimeJournal::shared_journaled(journal.clone());
    let run = RunId::new(1002);
    let seq = EventSeq::new(0);
    let original = test_decision_record(run, 0x77);

    let result = adapter.append_policy_decision_strict(original.clone(), seq);
    let _ = require_ok(result, "append_policy_decision_strict");
    drop(adapter);

    let reopened = match FjallJournal::open(dir.path(), None) {
        Ok(j) => Arc::new(j),
        Err(error) => {
            assert!(false, "reopen journal: {error}");
            return;
        }
    };

    let events = match reopened.events_for_run(run) {
        Ok(events) => events,
        Err(error) => {
            assert!(false, "read events after reopen: {error}");
            return;
        }
    };

    assert_eq!(events.len(), 1, "expected exactly one decision event after reopen");
    match &events[0] {
        JournalEvent::PolicyDecisionEvent { decision: ev_decision, .. } => {
            assert_eq!(*ev_decision, original, "decision did not round-trip");
        }
        other => {
            assert!(false, "unexpected journal event type after reopen: {other:?}");
        }
    }
    drop(reopened);
    drop(dir);
}

/// Nonzero and distinct policy/input artifact identities must survive
/// the storage round-trip, proving the full record is preserved.
#[test]
fn storage_journaled_decision_preserves_distinct_artifact_identities() {
    let (dir, journal) = match temp_journal_path() {
        Ok(j) => j,
        Err(error) => {
            assert!(false, "temp journal: {error}");
            return;
        }
    };
    let adapter = StorageRuntimeJournal::shared_journaled(journal.clone());
    let run = RunId::new(1003);
    let seq = EventSeq::new(0);
    let decision = test_decision_record(run, 0xFF);

    let result = adapter.append_policy_decision_strict(decision.clone(), seq);
    let _ = require_ok(result, "append_policy_decision_strict");

    let events = match journal.events_for_run(run) {
        Ok(events) => events,
        Err(error) => {
            assert!(false, "read events: {error}");
            return;
        }
    };

    match &events[0] {
        JournalEvent::PolicyDecisionEvent { decision: ev_decision, .. } => {
            assert_eq!(
                ev_decision.admission, decision.admission,
                "admission artifact identity altered"
            );
            assert_eq!(
                ev_decision.input_digest, decision.input_digest,
                "input digest artifact identity altered"
            );
            assert_ne!(
                ev_decision.admission, ev_decision.input_digest.into(),
                "admission and input digests must be distinct"
            );
        }
        other => {
            assert!(false, "unexpected journal event type: {other:?}");
        }
    }
    drop(dir);
}

/// Queued journal must drain any prior lifecycle prefix before the strict
/// decision write, then the queue must remain accepting for a later
/// ordinary append.
#[test]
fn queued_prefix_drains_before_strict_decision_and_queue_remains_accepting() -> RuntimeResult<()> {
    let (dir, journal) = temp_journal_path()
        .map_err(|error| RuntimeError::from(vb_storage::JournalError::Open {
            path: dir.path().to_path_buf(),
            source: Box::new(error),
        }))
        .map(|(d, j)| (d, j))?;
    let queue = journal_queue(16)
        .map_err(|error| RuntimeError::from(vb_storage::JournalError::QueueFull))?;
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue);
    let run = RunId::new(1004);
    let prefix_seq = EventSeq::new(0);
    let decision_seq = EventSeq::new(1);
    let later_seq = EventSeq::new(2);
    let decision = test_decision_record(run, 0xEE);

    adapter.append_sequenced(
        super::RuntimeJournalEvent::RunSubmitted {
            run,
            workflow: WorkflowDigest::from_bytes([0x11; 32]),
        },
        prefix_seq,
    )?;

    adapter.append_policy_decision_strict(decision.clone(), decision_seq)?;

    adapter.append_sequenced(
        super::RuntimeJournalEvent::RunFinished {
            run,
            result: SlotIdx::new(0),
            attempt: 1,
        },
        later_seq,
    )?;

    let events = journal
        .events_for_run(run)
        .map_err(|error| RuntimeError::from(error))?;

    assert_eq!(events.len(), 3, "expected 3 events after drain and later append");
    match &events[0] {
        JournalEvent::RunAccepted { run: r, seq: s, .. } => {
            assert_eq!(*r, run);
            assert_eq!(*s, prefix_seq);
        }
        other => {
            assert!(false, "prefix event not RunAccepted: {other:?}");
        }
    }
    match &events[1] {
        JournalEvent::PolicyDecisionEvent { run: r, seq: s, decision: ev } => {
            assert_eq!(*r, run);
            assert_eq!(*s, decision_seq);
            assert_eq!(*ev, decision, "drained decision not preserved");
        }
        other => {
            assert!(false, "decision event missing: {other:?}");
        }
    }
    match &events[2] {
        JournalEvent::RunFinished { run: r, seq: s, .. } => {
            assert_eq!(*r, run);
            assert_eq!(*s, later_seq);
        }
        other => {
            assert!(false, "later event not RunFinished: {other:?}");
        }
    }
    drop(dir);
    Ok(())
}

/// VolatileRuntimeJournal uses the trait default: exact
/// UnsupportedOperation("append_policy_decision_strict").
#[test]
fn volatile_journal_default_returns_unsupported_operation() {
    let journal = VolatileRuntimeJournal::shared();
    let run = RunId::new(1005);
    let decision = test_decision_record(run, 0x00);
    let result = journal.append_policy_decision_strict(decision, EventSeq::new(0));
    match result {
        Err(RuntimeError::UnsupportedOperation { operation }) => {
            assert_eq!(
                operation, "append_policy_decision_strict",
                "VolatileRuntimeJournal must use trait default error message"
            );
        }
        other => {
            assert!(false, "expected UnsupportedOperation, got: {other:?}");
        }
    }
}

/// NoopRuntimeJournal uses the trait default: exact
/// UnsupportedOperation("append_policy_decision_strict").
#[test]
fn noop_journal_default_returns_unsupported_operation() {
    let journal = NoopRuntimeJournal::shared();
    let run = RunId::new(1006);
    let decision = test_decision_record(run, 0x00);
    let result = journal.append_policy_decision_strict(decision, EventSeq::new(0));
    match result {
        Err(RuntimeError::UnsupportedOperation { operation }) => {
            assert_eq!(
                operation, "append_policy_decision_strict",
                "NoopRuntimeJournal must use trait default error message"
            );
        }
        other => {
            assert!(false, "expected UnsupportedOperation, got: {other:?}");
        }
    }
}