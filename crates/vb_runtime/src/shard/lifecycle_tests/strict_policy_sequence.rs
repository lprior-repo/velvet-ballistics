// Policy decision lifecycle tests: shard-level strict port behavior.
// Tests call real Shard::append_policy_decision_strict with actual pending
// action tickets obtained by submitting workflows and ticking to suspension.

use std::sync::{Arc, Mutex};

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SlotIdx};
use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, DecisionRecord, InfluenceState, PolicyDigest,
    PolicyLayer, PinnedAdmissionPolicy,
};
use vb_core::value::{SlotValue, Taint};

use crate::error::RuntimeResult;
use crate::journal::{RuntimeJournal, RuntimeJournalEvent};
use crate::RuntimeError;

use super::super::super::types::{Shard, ShardCommand};
use super::{require_workflow, small_config, suspended_workflow};

// Test-local recording journal that captures strict policy decision calls.
#[derive(Debug, Default)]
struct RecordingJournal {
    strict_calls: Mutex<Vec<(DecisionRecord, vb_storage::EventSeq)>>,
    strict_error: Mutex<Option<RuntimeError>>,
}

impl RecordingJournal {
    fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn strict_calls(&self) -> RuntimeResult<Vec<(DecisionRecord, vb_storage::EventSeq)>> {
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

    fn append_policy_decision_strict(
        &self,
        decision: vb_core::policy::DecisionRecord,
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
        calls.push((decision, seq));

        if let Some(err) = pending_err {
            return Err(err);
        }
        Ok(())
    }

    fn probe(&self) -> RuntimeResult<()> {
        Ok(())
    }
}

fn test_decision_record(ticket: ActionTicket) -> DecisionRecord {
    DecisionRecord {
        ticket,
        admission: PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes([3; 32])),
        current_live: CurrentPolicyIdentity::Configured {
            digest: PolicyDigest::from_bytes([2; 32]),
        },
        input_digest: PolicyDigest::from_bytes([1; 32]),
        taint: Taint::Clean,
        outcome: DecisionCode::Allowed,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::NotEvaluated,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

fn make_shard_with_journal(
    run: RunId,
) -> Result<(Shard, Arc<RecordingJournal>, ActionTicket), Box<dyn std::error::Error>> {
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

    let ticket = shard
        .pending_action_get(run)
        .ok_or(RuntimeError::InvalidActionCompletion)?;
    Ok((shard, journal, ticket))
}

#[test]
fn successful_strict_policy_port() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(90001);
    let (mut shard, journal, ticket) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let seq_before = shard.journal_sequence_for(run);
    assert_eq!(seq_before.get(), 5, "sequence must be seeded at 5");

    shard.append_policy_decision_strict(test_decision_record(ticket))?;

    let seq_after = shard.journal_sequence_for(run);
    assert_eq!(seq_after.get(), 6, "internal sequence must advance from 5 to 6");

    let expected = test_decision_record(ticket);
    let calls = journal.strict_calls()?;
    assert_eq!(
        calls,
        vec![(expected, vb_storage::EventSeq::new(5))],
        "journal must record exact decision at seq 5"
    );

    let ticket_after = shard
        .pending_action_get(run)
        .ok_or("pending action ticket must still exist")?;
    assert_eq!(ticket_after, ticket, "pending ticket must remain unchanged");

    Ok(())
}

#[test]
fn typed_strict_policy_port_failure() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(90002);
    let (mut shard, journal, ticket) = make_shard_with_journal(run)?;

    let seq_before = shard.journal_sequence_for(run);

    journal.set_strict_error(RuntimeError::UnsupportedOperation {
        operation: "test_policy_port_failure",
    })?;

    let result = shard.append_policy_decision_strict(test_decision_record(ticket));
    let err = result.err().ok_or("expected shard to propagate journal error")?;
    match err {
        RuntimeError::UnsupportedOperation { operation } => {
            assert_eq!(
                operation,
                "test_policy_port_failure",
                "exact error message must propagate"
            );
        }
        _ => return Err("wrong typed error propagated".into()),
    }

    let seq_after = shard.journal_sequence_for(run);
    assert_eq!(seq_after, seq_before, "sequence must remain unchanged on error");

    let calls = journal.strict_calls()?;
    assert_eq!(calls.len(), 1, "exactly one journal call must be attempted");

    let ticket_after = shard
        .pending_action_get(run)
        .ok_or("pending action ticket must still exist")?;
    assert_eq!(ticket_after, ticket, "pending ticket must remain unchanged");

    Ok(())
}

#[test]
fn overflow_at_u64_max() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(90003);
    let (mut shard, journal, ticket) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(u64::MAX))?;
    let seq_max = shard.journal_sequence_for(run);
    assert_eq!(seq_max.get(), u64::MAX, "sequence must be at u64::MAX");

    let result = shard.append_policy_decision_strict(test_decision_record(ticket));
    let err = result.err().ok_or("expected shard to detect sequence overflow")?;
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

    let seq_after = shard.journal_sequence_for(run);
    assert_eq!(seq_after, seq_max, "sequence must remain at u64::MAX");

    let ticket_after = shard
        .pending_action_get(run)
        .ok_or("pending action ticket must still exist")?;
    assert_eq!(ticket_after, ticket, "pending ticket must remain unchanged");

    Ok(())
}

#[test]
fn wrong_ticket_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(90004);
    let (mut shard, journal, ticket) = make_shard_with_journal(run)?;

    let seq_before = shard.journal_sequence_for(run);

    let wrong_ticket = ActionTicket {
        action: ActionId::new(999),
        ..ticket
    };

    let result = shard.append_policy_decision_strict(test_decision_record(wrong_ticket));
    let err = result.err().ok_or("expected shard to reject wrong ticket")?;
    match err {
        RuntimeError::InvalidActionCompletion => {}
        _ => return Err("wrong typed wrong-ticket error".into()),
    }

    let calls = journal.strict_calls()?;
    assert_eq!(calls.len(), 0, "journal must not be called for wrong ticket");

    let seq_after = shard.journal_sequence_for(run);
    assert_eq!(seq_after, seq_before, "sequence must remain unchanged");

    let ticket_after = shard
        .pending_action_get(run)
        .ok_or("pending action ticket must still exist")?;
    assert_eq!(ticket_after, ticket, "pending ticket must remain unchanged");

    Ok(())
}