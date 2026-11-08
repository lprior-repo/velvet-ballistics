use std::sync::Arc;

use vb_core::ids::{ActionId, RunId, SlotIdx, WorkflowDigest};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, GateDecision, GateDecisionInput, GateDigest, GateId,
    GateOutcomeRecord, ManualGate, RiskClass, SeparationOfDuties,
};
use vb_storage::{EventSeq, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits};

use crate::RuntimeError;
use crate::journal::VolatileRuntimeJournal;
use crate::journal::{
    QueuedStorageRuntimeJournal, RuntimeJournal, RuntimeJournalEvent, StorageRuntimeJournal,
};
use crate::manual_gate::evaluate_decision;

fn digest(byte: u8) -> GateDigest {
    GateDigest::from_bytes([byte; 32])
}

fn fixture_gate(run: RunId) -> ManualGate {
    ManualGate {
        id: GateId::new(7),
        run,
        action: ActionId::new(9),
        risk: RiskClass::High,
        compensatable: false,
        required_role: ApproverRole::new("deploy-approver".to_string()).unwrap(),
        separation_of_duties: SeparationOfDuties::DistinctFromRequester,
        evidence_bundle_digest: digest(1),
        prompt_digest: digest(2),
        timeout_ms: 1_000,
        escalation_policy_digest: digest(3),
        break_glass_allowed: true,
        approval_reason_required: true,
    }
}

fn fixture_record(run: RunId) -> GateOutcomeRecord {
    let input = GateDecisionInput {
        gate: GateId::new(7),
        decision: GateDecision::Approve,
        approver: ApproverId::new("alice".to_string()).unwrap(),
        approver_role: ApproverRole::new("deploy-approver".to_string()).unwrap(),
        requester: ApproverId::new("bob".to_string()).unwrap(),
        reason: Some("checked the rollout plan".to_string()),
        evidence_shown: true,
        evidence_bundle_digest: digest(1),
        delegated_to: None,
        break_glass_justification: None,
        decided_at_ms: 500,
    };
    evaluate_decision(&fixture_gate(run), 0, &input).unwrap()
}

fn setup_queued_test() -> Result<
    (
        tempfile::TempDir,
        Arc<FjallJournal>,
        Arc<JournalWriterQueue>,
        QueuedStorageRuntimeJournal,
    ),
    Box<dyn std::error::Error>,
> {
    let temp = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(temp.path(), None)?);
    let queue = Arc::new(JournalWriterQueue::new(64, 16, StorageLimits::DEFAULT)?);
    let adapter = QueuedStorageRuntimeJournal::journaled(journal.clone(), queue.clone());
    Ok((temp, journal, queue, adapter))
}

fn assert_run_a_prefix_flushed(
    journal: &FjallJournal,
    run_a: RunId,
    workflow: WorkflowDigest,
    record: &GateOutcomeRecord,
) -> Result<(), Box<dyn std::error::Error>> {
    let initial_a = journal.events_for_run(run_a)?;
    let expected = vec![
        JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::GateDecisionEvent {
            run: run_a,
            seq: EventSeq::new(1),
            record: Box::new(record.clone()),
        },
    ];
    assert_eq!(
        initial_a, expected,
        "run_a prefix flushed before strict gate decision"
    );
    Ok(())
}

fn append_drain_assert_run_b(
    adapter: &QueuedStorageRuntimeJournal,
    journal: &FjallJournal,
    run_b: RunId,
) -> Result<(), Box<dyn std::error::Error>> {
    adapter.append_sequenced(
        RuntimeJournalEvent::RunFinished {
            run: run_b,
            result: SlotIdx::new(1),
            attempt: 1,
        },
        EventSeq::new(0),
    )?;
    adapter.drain_all()?;
    let actual_b = journal.events_for_run(run_b)?;
    let expected_b = vec![JournalEvent::RunFinished {
        run: run_b,
        seq: EventSeq::new(0),
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
fn storage_adapter_persists_full_gate_decision_event() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(temp.path(), None)?);
    let adapter = StorageRuntimeJournal::shared_journaled(journal.clone());
    let run = RunId::new(1001);
    let record = fixture_record(run);

    adapter.append_gate_decision_strict(record.clone(), EventSeq::new(0))?;

    drop(adapter);
    drop(journal);

    let reopened = FjallJournal::open(temp.path(), None)?;
    let actual = reopened.events_for_run(run)?;
    let expected = vec![JournalEvent::GateDecisionEvent {
        run,
        seq: EventSeq::new(0),
        record: Box::new(record.clone()),
    }];
    assert_eq!(
        actual, expected,
        "full durable GateDecisionEvent roundtrip"
    );
    Ok(())
}

#[test]
fn queued_adapter_drains_prefix_before_strict_decision() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, journal, _queue, adapter) = setup_queued_test()?;
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

    let record = fixture_record(run_a);
    adapter.append_gate_decision_strict(record.clone(), EventSeq::new(1))?;

    assert_run_a_prefix_flushed(&journal, run_a, workflow, &record)?;

    append_drain_assert_run_b(&adapter, &journal, run_b)?;

    Ok(())
}

#[test]
fn noop_and_volatile_adapters_fail_closed() {
    let noop = crate::journal::NoopRuntimeJournal::shared();
    let run = RunId::new(3001);
    let result = noop.append_gate_decision_strict(fixture_record(run), EventSeq::new(0));
    assert!(
        matches!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "append_gate_decision_strict"
            })
        ),
        "noop adapter must return exact UnsupportedOperation"
    );

    let volatile = VolatileRuntimeJournal::shared();
    let result = volatile.append_gate_decision_strict(fixture_record(run), EventSeq::new(0));
    assert!(
        matches!(
            result,
            Err(RuntimeError::UnsupportedOperation {
                operation: "append_gate_decision_strict"
            })
        ),
        "volatile adapter must return exact UnsupportedOperation"
    );
}
