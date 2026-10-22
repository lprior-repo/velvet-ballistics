use std::sync::Arc;
use vb_core::ids::{ActionId, EventSeq, RunId, SeqNo, StepIdx, WorkflowDigest};
use vb_core::policy::{
    DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy, PolicyDigest,
    PolicyLayer,
};
use vb_core::Taint;
use vb_storage::{
    DurabilityProfile, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits,
};

use crate::journal::{QueuedStorageRuntimeJournal, RuntimeJournalEvent, StorageRuntimeJournal};
use crate::{RuntimeError, VolatileRuntimeJournal};

fn make_fixture_decision(run: RunId) -> DecisionRecord {
    let ticket = vb_core::action::ActionTicket {
        run,
        step: StepIdx::new(3),
        seq: SeqNo::new(7),
        action: ActionId::new(12),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0x0a0b0c0d0e0f1011u128,
        capacity: 3,
    };
    let admission = PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes([
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
        27, 28, 29, 30, 31, 32,
    ])));
    let current_live = vb_core::policy::CurrentPolicyIdentity::Configured {
        digest: PolicyDigest::from_bytes([
            32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11,
            10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
        ]),
    };
    let input_digest = PolicyDigest::from_bytes([
        128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145,
        146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159,
    ]);
    DecisionRecord {
        ticket,
        admission,
        current_live,
        input_digest,
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
    let decision = make_fixture_decision(run);

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
    assert_eq!(actual, expected, "full durable PolicyDecisionEvent roundtrip");
    Ok(())
}

#[test]
fn queued_adapter_drains_prefix_before_strict_decision() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(temp.path(), None)?);
    let limits = StorageLimits {
        max_journal_event_payload_bytes: u32::MAX,
    };
    let queue = Arc::new(JournalWriterQueue::new(64, 16, limits)?);
    let adapter = QueuedStorageRuntimeJournal::shared_journaled(journal.clone(), queue.clone());

    let run_a = RunId::new(2001);
    let run_b = RunId::new(2002);
    let workflow = WorkflowDigest::from_bytes([
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]);

    adapter.append_sequenced(
        RuntimeJournalEvent::RunSubmitted { run: run_a, workflow },
        EventSeq::new(0),
    )?;
    adapter.append_policy_decision_strict(make_fixture_decision(run_a), EventSeq::new(1))?;
    adapter.append_sequenced(
        RuntimeJournalEvent::RunFinished {
            run: run_b,
            result: vb_core::ids::SlotIdx::new(1),
            attempt: 1,
        },
        EventSeq::new(2),
    )?;

    adapter.drain_all()?;

    let actual_a = journal.events_for_run(run_a)?;
    let expected_a = vec![
        JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::PolicyDecisionEvent {
            run: run_a,
            seq: EventSeq::new(1),
            decision: make_fixture_decision(run_a),
        },
    ];
    assert_eq!(actual_a, expected_a, "run_a lifecycle plus policy decision persisted");

    let actual_b = journal.events_for_run(run_b)?;
    let expected_b = vec![JournalEvent::RunFinished {
        run: run_b,
        seq: EventSeq::new(2),
        result: vb_core::ids::SlotIdx::new(1),
        attempt: 1,
    }];
    assert_eq!(actual_b, expected_b, "run_b event persisted after strict decision");

    adapter.append_sequenced(
        RuntimeJournalEvent::Resumed {
            run: run_b,
            timestamp: 100,
        },
        EventSeq::new(3),
    )?;
    Ok(())
}

#[test]
fn noop_and_volatile_adapters_fail_closed() {
    let noop = crate::journal::NoopRuntimeJournal::shared();
    let run = RunId::new(3001);
    let result = noop.append_policy_decision_strict(make_fixture_decision(run), EventSeq::new(0));
    match result {
        Err(RuntimeError::UnsupportedOperation { operation }) => {
            assert_eq!(operation, "append_policy_decision_strict");
        }
        other => panic!("expected UnsupportedOperation, got {:?}", other),
    }

    let volatile = VolatileRuntimeJournal::shared();
    let result = volatile.append_policy_decision_strict(make_fixture_decision(run), EventSeq::new(0));
    match result {
        Err(RuntimeError::UnsupportedOperation { operation }) => {
            assert_eq!(operation, "append_policy_decision_strict");
        }
        other => panic!("expected UnsupportedOperation, got {:?}", other),
    }
}
