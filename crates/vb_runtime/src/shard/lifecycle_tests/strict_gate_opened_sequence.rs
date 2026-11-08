use std::sync::{Arc, Mutex};

use vb_core::ids::{ActionId, RunId, SlotIdx};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, GateDigest, GateId, ManualGate, RiskClass, SeparationOfDuties,
};
use vb_core::value::SlotValue;

use crate::RuntimeError;
use crate::error::RuntimeResult;
use crate::journal::{RuntimeJournal, RuntimeJournalEvent};

use super::super::super::types::{Shard, ShardCommand};
use super::{require_workflow, small_config, suspended_workflow};

#[derive(Debug, Default)]
struct RecordingJournal {
    opened_calls: Mutex<Vec<(ManualGate, u64, ApproverId, vb_storage::EventSeq)>>,
    strict_error: Mutex<Option<RuntimeError>>,
}

impl RecordingJournal {
    fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn opened_calls(
        &self,
    ) -> RuntimeResult<Vec<(ManualGate, u64, ApproverId, vb_storage::EventSeq)>> {
        let calls = self
            .opened_calls
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

    fn append_gate_opened_strict(
        &self,
        gate: ManualGate,
        opened_at_ms: u64,
        requester: ApproverId,
        seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        let mut err_slot = self
            .strict_error
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;

        let pending_err = err_slot.take();

        let mut calls = self
            .opened_calls
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?;
        calls.push((gate, opened_at_ms, requester, seq));

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

fn test_gate(run: RunId, id: u64) -> ManualGate {
    ManualGate {
        id: GateId::new(id),
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

fn gate_opened_bytes(
    gate: &ManualGate,
    opened_at_ms: u64,
    requester: &ApproverId,
    seq: vb_storage::EventSeq,
) -> Result<u64, Box<dyn std::error::Error>> {
    let event = vb_storage::JournalEvent::GateOpenedEvent {
        run: gate.run,
        seq,
        gate: gate.clone(),
        opened_at_ms,
        requester: requester.clone(),
    };
    let encoded = postcard::to_allocvec(&event).map_err(|_| "opened event must encode")?;
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
fn successful_strict_gate_opened_port() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92001);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let seq_before = shard.journal_sequence_for(run);
    assert_eq!(seq_before.get(), 5, "sequence must be seeded at 5");

    let gate = test_gate(run, 7);
    shard.append_gate_opened_strict(gate.clone(), 100, person("bob"))?;

    let seq_after = shard.journal_sequence_for(run);
    assert_eq!(
        seq_after.get(),
        6,
        "internal sequence must advance from 5 to 6"
    );

    let calls = journal.opened_calls()?;
    assert_eq!(
        calls,
        vec![(gate, 100, person("bob"), vb_storage::EventSeq::new(5))],
        "journal must record exact opening at seq 5"
    );

    assert_seq_unchanged(&shard, run, seq_after)?;
    Ok(())
}

#[test]
fn typed_strict_gate_opened_port_failure() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92002);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    let seq_before = shard.journal_sequence_for(run);
    let bytes_before = journal_bytes_written(&shard, run)?;

    journal.set_strict_error(RuntimeError::UnsupportedOperation {
        operation: "test_gate_opened_port_failure",
    })?;

    let gate = test_gate(run, 7);
    let result = shard.append_gate_opened_strict(gate.clone(), 100, person("bob"));
    let err = result
        .err()
        .ok_or("expected shard to propagate journal error")?;
    match err {
        RuntimeError::UnsupportedOperation { operation } => {
            assert_eq!(
                operation, "test_gate_opened_port_failure",
                "exact error message must propagate"
            );
        }
        _ => return Err("wrong typed error propagated".into()),
    }

    let calls = journal.opened_calls()?;
    assert_eq!(
        calls,
        vec![(gate, 100, person("bob"), seq_before)],
        "journal must record exact opening at seq before"
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
fn gate_opened_charges_journal_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92003);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let before = journal_bytes_written(&shard, run)?;
    let gate = test_gate(run, 7);

    shard.append_gate_opened_strict(gate.clone(), 100, person("bob"))?;

    let after = journal_bytes_written(&shard, run)?;
    let expected = gate_opened_bytes(&gate, 100, &person("bob"), vb_storage::EventSeq::new(5))?;
    assert_eq!(
        after,
        before + expected,
        "counter must grow by exactly the stored opened event bytes"
    );

    let calls = journal.opened_calls()?;
    assert_eq!(
        calls,
        vec![(gate, 100, person("bob"), vb_storage::EventSeq::new(5))],
        "journal must record exact opening at seq 5"
    );
    Ok(())
}

#[test]
fn gate_opened_charges_accumulate() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92004);
    let (mut shard, _journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let before = journal_bytes_written(&shard, run)?;
    let first = test_gate(run, 7);
    let second = test_gate(run, 8);

    shard.append_gate_opened_strict(first.clone(), 100, person("bob"))?;
    shard.append_gate_opened_strict(second.clone(), 200, person("bob"))?;

    let after = journal_bytes_written(&shard, run)?;
    let expected = gate_opened_bytes(&first, 100, &person("bob"), vb_storage::EventSeq::new(5))?
        + gate_opened_bytes(&second, 200, &person("bob"), vb_storage::EventSeq::new(6))?;
    assert_eq!(
        after,
        before + expected,
        "two openings must charge the sum of their stored event bytes"
    );
    Ok(())
}

#[test]
fn gate_opened_counter_overflow_fails_before_append() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92005);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let seq_before = shard.journal_sequence_for(run);
    shard
        .run_state_get_mut(run)
        .ok_or("run state must exist")?
        .journal_bytes_written = u64::MAX;

    let result = shard.append_gate_opened_strict(test_gate(run, 7), 100, person("bob"));
    let err = result
        .err()
        .ok_or("expected shard to reject counter overflow")?;
    match err {
        RuntimeError::EncodeFailed => {}
        _ => return Err("wrong typed counter overflow error".into()),
    }

    let calls = journal.opened_calls()?;
    assert_eq!(
        calls.len(),
        0,
        "journal must not be called when the counter overflows"
    );

    assert_seq_unchanged(&shard, run, seq_before)?;
    Ok(())
}

#[test]
fn gate_opened_sequence_overflow_fails_before_append() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92006);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(u64::MAX))?;
    let seq_max = shard.journal_sequence_for(run);
    assert_eq!(seq_max.get(), u64::MAX, "sequence must be at u64::MAX");

    let result = shard.append_gate_opened_strict(test_gate(run, 7), 100, person("bob"));
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

    let calls = journal.opened_calls()?;
    assert_eq!(calls.len(), 0, "journal must not be called on overflow");

    assert_seq_unchanged(&shard, run, seq_max)?;
    Ok(())
}

#[test]
fn structurally_invalid_gate_rejected_before_append() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92007);
    let (mut shard, journal) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let seq_before = shard.journal_sequence_for(run);
    let bytes_before = journal_bytes_written(&shard, run)?;

    let mut invalid = test_gate(run, 7);
    invalid.id = GateId::new(0);
    assert!(!invalid.is_structurally_valid());

    let result = shard.append_gate_opened_strict(invalid, 100, person("bob"));
    let err = result
        .err()
        .ok_or("expected shard to reject an invalid gate")?;
    match err {
        RuntimeError::InvalidGateOpenedRecord => {}
        _ => return Err("wrong typed invalid gate error".into()),
    }

    let calls = journal.opened_calls()?;
    assert_eq!(
        calls.len(),
        0,
        "journal must not be called for an invalid gate"
    );

    assert_eq!(
        journal_bytes_written(&shard, run)?,
        bytes_before,
        "counter must not advance for an invalid gate"
    );
    assert_seq_unchanged(&shard, run, seq_before)?;

    let bad_requester = shard.append_gate_opened_strict(
        test_gate(run, 7),
        100,
        ApproverId {
            value: String::new(),
        },
    );
    let err = bad_requester
        .err()
        .ok_or("expected shard to reject an invalid requester")?;
    match err {
        RuntimeError::InvalidGateOpenedRecord => {}
        _ => return Err("wrong typed invalid requester error".into()),
    }
    assert_eq!(
        journal.opened_calls()?.len(),
        0,
        "journal must not be called for an invalid requester"
    );
    Ok(())
}
