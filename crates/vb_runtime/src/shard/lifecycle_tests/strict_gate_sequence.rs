use std::sync::{Arc, Mutex};

use vb_core::ids::{ActionId, RunId, SlotIdx};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, GateDecision, GateDecisionInput, GateDigest, GateId, GateOutcome,
    GateOutcomeRecord, ManualGate, RiskClass, SeparationOfDuties,
};
use vb_core::value::SlotValue;

use crate::RuntimeError;
use crate::error::RuntimeResult;
use crate::journal::{RuntimeJournal, RuntimeJournalEvent};
use crate::manual_gate::{evaluate_decision, evaluate_timeout};

use super::super::super::types::{Shard, ShardCommand};
use super::{require_workflow, small_config, suspended_workflow};

#[derive(Debug, Default)]
struct RecordingJournal {
    strict_calls: Mutex<Vec<(GateOutcomeRecord, vb_storage::EventSeq)>>,
    strict_error: Mutex<Option<RuntimeError>>,
}

impl RecordingJournal {
    fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn strict_calls(&self) -> RuntimeResult<Vec<(GateOutcomeRecord, vb_storage::EventSeq)>> {
        let calls = self
            .strict_calls
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        Ok(calls.clone())
    }

    fn set_strict_error(&self, err: RuntimeError) -> RuntimeResult<()> {
        let mut slot = self
            .strict_error
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        *slot = Some(err);
        Ok(())
    }
}

impl RuntimeJournal for RecordingJournal {
    fn append(&self, _event: RuntimeJournalEvent) -> RuntimeResult<()> {
        Ok(())
    }

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        _start_seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        for event in events {
            self.append(event.clone())?;
        }
        Ok(())
    }

    fn append_gate_decision_strict(
        &self,
        record: GateOutcomeRecord,
        seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        let mut err_slot = self
            .strict_error
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;

        let pending_err = err_slot.take();

        let mut calls = self
            .strict_calls
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        calls.push((record, seq));

        if let Some(err) = pending_err {
            return Err(err);
        }
        Ok(())
    }

    fn probe(&self) -> RuntimeResult<()> {
        Ok(())
    }
}

fn digest(byte: u8) -> GateDigest {
    GateDigest::from_bytes([byte; 32])
}

fn role() -> ApproverRole {
    ApproverRole::new("deploy-approver".to_string()).unwrap()
}

fn person(value: &str) -> ApproverId {
    ApproverId::new(value.to_string()).unwrap()
}

fn test_gate(run: RunId) -> ManualGate {
    ManualGate {
        id: GateId::new(7),
        run,
        action: ActionId::new(9),
        risk: RiskClass::High,
        compensatable: false,
        required_role: role(),
        separation_of_duties: SeparationOfDuties::DistinctFromRequester,
        evidence_bundle_digest: digest(1),
        prompt_digest: digest(2),
        timeout_ms: 1_000,
        escalation_policy_digest: digest(3),
        break_glass_allowed: true,
        approval_reason_required: true,
    }
}

fn test_input(decision: GateDecision) -> GateDecisionInput {
    GateDecisionInput {
        gate: GateId::new(7),
        decision,
        approver: person("alice"),
        approver_role: role(),
        requester: person("bob"),
        reason: Some("checked the rollout plan".to_string()),
        evidence_shown: true,
        evidence_bundle_digest: digest(1),
        delegated_to: None,
        break_glass_justification: None,
        decided_at_ms: 500,
    }
}

fn test_record(run: RunId, decision: GateDecision) -> GateOutcomeRecord {
    evaluate_decision(&test_gate(run), 0, &test_input(decision)).unwrap()
}

fn test_escalation(run: RunId) -> GateOutcomeRecord {
    evaluate_timeout(&test_gate(run), &person("bob"), 0, 2_000).unwrap()
}

fn make_shard_with_journal(
    run: RunId,
) -> Result<(Shard, Arc<RecordingJournal>), Box<dyn std::error::Error>> {
    let journal = RecordingJournal::shared();
    let mut shard = Shard::new_with_journal(small_config(), journal.clone());
    let workflow = require_workflow("suspended", suspended_workflow())?;

    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0)),
        inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
    })?;
    shard.tick()?;

    Ok((shard, journal))
}

fn journal_bytes_written(shard: &Shard, run: RunId) -> Result<u64, Box<dyn std::error::Error>> {
    shard
        .run_state_get(run)
        .map(|state| state.journal_bytes_written)
        .ok_or("run state must exist".into())
}

fn gate_decision_bytes(
    record: &GateOutcomeRecord,
    seq: vb_storage::EventSeq,
) -> Result<u64, Box<dyn std::error::Error>> {
    let event = vb_storage::JournalEvent::GateDecisionEvent {
        run: record.run,
        seq,
        record: Box::new(record.clone()),
    };
    let encoded = postcard::to_allocvec(&event).map_err(|_| "decision event must encode")?;
    u64::try_from(encoded.len()).map_err(|_| "encoded length must fit u64".into())
}

fn assert_seq_unchanged(
    shard: &Shard,
    run: RunId,
    expected_seq: vb_storage::EventSeq,
) -> Result<(), Box<dyn std::error::Error>> {
    let seq_after = shard.journal_sequence_for(run);
    assert_eq!(seq_after, expected_seq, "sequence must remain unchanged");
    Ok(())
}

#[test]
fn successful_strict_gate_port() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(91001);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let seq_before = shard.journal_sequence_for(run);
    assert_eq!(seq_before.get(), 5, "sequence must be seeded at 5");

    let record = test_record(run, GateDecision::Approve);
    shard.append_gate_decision_strict(record.clone())?;

    let seq_after = shard.journal_sequence_for(run);
    assert_eq!(
        seq_after.get(),
        6,
        "internal sequence must advance from 5 to 6"
    );

    let calls = journal.strict_calls()?;
    assert_eq!(
        calls,
        vec![(record, vb_storage::EventSeq::new(5))],
        "journal must record exact decision at seq 5"
    );

    assert_seq_unchanged(&shard, run, seq_after)?;
    Ok(())
}

#[test]
fn typed_strict_gate_port_failure() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(91002);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    let seq_before = shard.journal_sequence_for(run);
    let bytes_before = journal_bytes_written(&shard, run)?;

    journal.set_strict_error(RuntimeError::UnsupportedOperation {
        operation: "test_gate_port_failure",
    })?;

    let result = shard.append_gate_decision_strict(test_record(run, GateDecision::Approve));
    let err = result
        .err()
        .ok_or("expected shard to propagate journal error")?;
    match err {
        RuntimeError::UnsupportedOperation { operation } => {
            assert_eq!(
                operation, "test_gate_port_failure",
                "exact error message must propagate"
            );
        }
        _ => return Err("wrong typed error propagated".into()),
    }

    let calls = journal.strict_calls()?;
    assert_eq!(
        calls,
        vec![(
            test_record(run, GateDecision::Approve),
            seq_before
        )],
        "journal must record exact decision at seq before"
    );

    assert_eq!(
        journal_bytes_written(&shard, run)?,
        bytes_before,
        "counter must not advance when the journal fails"
    );
    assert_seq_unchanged(&shard, run, seq_before)?;
    Ok(())
}

#[test]
fn gate_decision_charges_journal_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(91003);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let before = journal_bytes_written(&shard, run)?;
    let record = test_record(run, GateDecision::Approve);

    shard.append_gate_decision_strict(record.clone())?;

    let after = journal_bytes_written(&shard, run)?;
    let expected = gate_decision_bytes(&record, vb_storage::EventSeq::new(5))?;
    assert_eq!(
        after,
        before + expected,
        "counter must grow by exactly the stored decision event bytes"
    );

    let calls = journal.strict_calls()?;
    assert_eq!(
        calls,
        vec![(record, vb_storage::EventSeq::new(5))],
        "journal must record exact decision at seq 5"
    );
    Ok(())
}

#[test]
fn gate_decision_denied_outcome_charges_journal_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(91004);
    let (mut shard, _journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let before = journal_bytes_written(&shard, run)?;
    let record = test_record(run, GateDecision::Deny);

    shard.append_gate_decision_strict(record.clone())?;

    let after = journal_bytes_written(&shard, run)?;
    let expected = gate_decision_bytes(&record, vb_storage::EventSeq::new(5))?;
    assert_eq!(
        after,
        before + expected,
        "denied decisions must charge journal bytes exactly like approvals"
    );
    Ok(())
}

#[test]
fn gate_decision_escalation_charges_journal_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(91005);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let before = journal_bytes_written(&shard, run)?;
    let record = test_escalation(run);
    assert_eq!(record.outcome, GateOutcome::Escalated);
    assert_eq!(record.decision, None);

    shard.append_gate_decision_strict(record.clone())?;

    let after = journal_bytes_written(&shard, run)?;
    let expected = gate_decision_bytes(&record, vb_storage::EventSeq::new(5))?;
    assert_eq!(
        after,
        before + expected,
        "escalations must charge journal bytes exactly like operator decisions"
    );

    let calls = journal.strict_calls()?;
    assert_eq!(
        calls,
        vec![(record, vb_storage::EventSeq::new(5))],
        "journal must record exact escalation at seq 5"
    );
    Ok(())
}

#[test]
fn gate_decision_charges_accumulate() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(91006);
    let (mut shard, _journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let before = journal_bytes_written(&shard, run)?;
    let first = test_record(run, GateDecision::Approve);
    let second = test_record(run, GateDecision::Deny);

    shard.append_gate_decision_strict(first.clone())?;
    shard.append_gate_decision_strict(second.clone())?;

    let after = journal_bytes_written(&shard, run)?;
    let expected = gate_decision_bytes(&first, vb_storage::EventSeq::new(5))?
        + gate_decision_bytes(&second, vb_storage::EventSeq::new(6))?;
    assert_eq!(
        after,
        before + expected,
        "two decisions must charge the sum of their stored event bytes"
    );
    Ok(())
}

#[test]
fn gate_decision_counter_overflow_fails_before_append() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(91007);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let seq_before = shard.journal_sequence_for(run);
    shard
        .run_state_get_mut(run)
        .ok_or("run state must exist")?
        .journal_bytes_written = u64::MAX;

    let result = shard.append_gate_decision_strict(test_record(run, GateDecision::Approve));
    let err = result
        .err()
        .ok_or("expected shard to reject counter overflow")?;
    match err {
        RuntimeError::EncodeFailed => {}
        _ => return Err("wrong typed counter overflow error".into()),
    }

    let calls = journal.strict_calls()?;
    assert_eq!(
        calls.len(),
        0,
        "journal must not be called when the counter overflows"
    );

    assert_seq_unchanged(&shard, run, seq_before)?;
    Ok(())
}

#[test]
fn gate_decision_sequence_overflow_fails_before_append() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(91008);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(u64::MAX))?;
    let seq_max = shard.journal_sequence_for(run);
    assert_eq!(seq_max.get(), u64::MAX, "sequence must be at u64::MAX");

    let result = shard.append_gate_decision_strict(test_record(run, GateDecision::Approve));
    let err = result
        .err()
        .ok_or("expected shard to detect sequence overflow")?;
    match err {
        RuntimeError::StorageJournalAppend { source } => {
            assert!(
                matches!(source.as_ref(), vb_storage::JournalError::SequenceOverflow),
                "source must be JournalError::SequenceOverflow"
            );
        }
        _ => return Err("wrong typed overflow error".into()),
    }

    let calls = journal.strict_calls()?;
    assert_eq!(calls.len(), 0, "journal must not be called on overflow");

    assert_seq_unchanged(&shard, run, seq_max)?;
    Ok(())
}

#[test]
fn structurally_invalid_record_rejected_before_append() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(91009);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let seq_before = shard.journal_sequence_for(run);
    let bytes_before = journal_bytes_written(&shard, run)?;

    let mut invalid = test_record(run, GateDecision::Approve);
    invalid.receipt = None;
    assert!(!invalid.is_structurally_valid());

    let result = shard.append_gate_decision_strict(invalid);
    let err = result
        .err()
        .ok_or("expected shard to reject an invalid record")?;
    match err {
        RuntimeError::InvalidGateDecisionRecord => {}
        _ => return Err("wrong typed invalid record error".into()),
    }

    let calls = journal.strict_calls()?;
    assert_eq!(
        calls.len(),
        0,
        "journal must not be called for an invalid record"
    );

    assert_eq!(
        journal_bytes_written(&shard, run)?,
        bytes_before,
        "counter must not advance for an invalid record"
    );
    assert_seq_unchanged(&shard, run, seq_before)?;
    Ok(())
}
