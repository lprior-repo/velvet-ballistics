use std::sync::Arc;

use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::{
    DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy, PolicyDigest, PolicyLayer,
};
use vb_core::value::Taint;
use vb_storage::{
    DurabilityProfile, EventSeq, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits,
};

use crate::journal::{
    QueuedStorageRuntimeJournal, RuntimeJournal, RuntimeJournalEvent, StorageRuntimeJournal,
};
use crate::{RuntimeError, VolatileRuntimeJournal};

fn fixture_decision(run: RunId) -> DecisionRecord {
    DecisionRecord {
        ticket: vb_core::action::ActionTicket {
            run,
            step: StepIdx::new(3),
            seq: SeqNo::new(7),
            action: ActionId::new(12),
            run_attempt: 1,
            attempt: 1,
            idempotency_key: 0x0a0b0c0d0e0f1011u128,
            capacity: 3,
        },
        admission: PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes([17; 32])),
        current_live: vb_core::policy::CurrentPolicyIdentity::Configured {
            digest: PolicyDigest::from_bytes([29; 32]),
        },
        input_digest: PolicyDigest::from_bytes([43; 32]),
        taint: Taint::Clean,
        outcome: DecisionCode::Allowed,
        layer: PolicyLayer::Admission,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

#[test]
fn storage_adapter_persists_full_policy_decision_event() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(temp.path(), None)?);
    let adapter = StorageRuntimeJournal::shared_journaled(journal.clone());
    let run = RunId::new(1001);
    let decision = fixture_decision(run);

    adapter.append_policy_decision_strict(decision.clone(), EventSeq::new(1))?;

    drop(adapter);
    drop(journal);

    let reopened = FjallJournal::open(temp.path(), None)?;
    let actual = reopened.events_for_run(run)?;
    let expected = vec![JournalEvent::PolicyDecisionEvent {
        run,
        seq: EventSeq::new(1),
        decision: decision.clone(),
    }];
    assert_eq!(
        actual, expected,
        "full durable PolicyDecisionEvent roundtrip"
    );
    Ok(())
}

#[test]
fn queued_adapter_drains_prefix_before_strict_decision() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(temp.path(), None)?);
    let queue = Arc::new(JournalWriterQueue::new(64, 16, StorageLimits::DEFAULT)?);
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue.clone());

    let run_a = RunId::new(2001);
    let run_b = RunId::new(2002);
    let workflow = WorkflowDigest::from_bytes([17; 32]);

    adapter.append_sequenced(
        RuntimeJournalEvent::RunSubmitted {
            run: run_a,
            workflow,
        },
        EventSeq::new(0),
    )?;

    let decision = fixture_decision(run_a);
    adapter.append_policy_decision_strict(decision.clone(), EventSeq::new(1))?;

    let initial_a = journal.events_for_run(run_a)?;
    assert_eq!(
        initial_a,
        vec![
            JournalEvent::RunAccepted {
                run: run_a,
                seq: EventSeq::new(0),
                workflow
            },
            JournalEvent::PolicyDecisionEvent {
                run: run_a,
                seq: EventSeq::new(1),
                decision: decision.clone(),
            }
        ],
        "run_a prefix flushed before strict policy decision"
    );

    adapter.append_sequenced(
        RuntimeJournalEvent::RunFinished {
            run: run_b,
            result: SlotIdx::new(1),
            attempt: 1,
        },
        EventSeq::new(2),
    )?;

    adapter.drain_all()?;

    let actual_b = journal.events_for_run(run_b)?;
    let expected_b = vec![JournalEvent::RunFinished {
        run: run_b,
        seq: EventSeq::new(2),
        result: SlotIdx::new(1),
        attempt: 1,
    }];
    assert_eq!(
        actual_b, expected_b,
        "run_b event persisted after strict decision"
    );
    Ok(())
}

#[test]
fn noop_and_volatile_adapters_fail_closed() {
    let noop = crate::journal::NoopRuntimeJournal::shared();
    let run = RunId::new(3001);
    let result = noop.append_policy_decision_strict(fixture_decision(run), EventSeq::new(0));
    assert!(
        matches!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "append_policy_decision_strict"
            })
        ),
        "noop adapter must return exact UnsupportedOperation"
    );

    let volatile = VolatileRuntimeJournal::shared();
    let result = volatile.append_policy_decision_strict(fixture_decision(run), EventSeq::new(0));
    assert!(
        matches!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "append_policy_decision_strict"
            })
        ),
        "volatile adapter must return exact UnsupportedOperation"
    );
}
