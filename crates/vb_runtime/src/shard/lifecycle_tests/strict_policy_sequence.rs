// Policy decision lifecycle tests: shard-level strict port behavior.
// Tests call real Shard::append_policy_decision_strict with actual pending
// action tickets obtained by submitting workflows and ticking to suspension.

use std::sync::{Arc, Mutex};

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SlotIdx};
use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy,
    PolicyDigest, PolicyLayer,
};
use vb_core::value::{SlotValue, Taint};

use crate::RuntimeError;
use crate::error::RuntimeResult;
use crate::journal::{RuntimeJournal, RuntimeJournalEvent};

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

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        _start_seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        // RecordingJournal is a test fixture that only observes the strict
        // policy decision port; it does not claim real durability semantics
        // or record lifecycle event payloads. The shard's lifecycle paths
        // (submit, tick, policy decision) exercise this fake via
        // append_sequenced_batch; for test purposes every event is accepted.
        for event in events {
            self.append(event.clone())?;
        }
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

fn assert_ticket_and_seq_unchanged(
    shard: &Shard,
    expected_ticket: ActionTicket,
    expected_seq: vb_storage::EventSeq,
) -> Result<(), Box<dyn std::error::Error>> {
    let run = expected_ticket.run;
    let seq_after = shard.journal_sequence_for(run);
    assert_eq!(seq_after, expected_seq, "sequence must remain unchanged");
    let ticket_after = shard
        .pending_action_get(run)
        .ok_or("pending action ticket must still exist")?;
    assert_eq!(
        ticket_after, expected_ticket,
        "pending ticket must remain unchanged"
    );
    Ok(())
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
    assert_eq!(
        seq_after.get(),
        6,
        "internal sequence must advance from 5 to 6"
    );

    let expected = test_decision_record(ticket);
    let calls = journal.strict_calls()?;
    assert_eq!(
        calls,
        vec![(expected, vb_storage::EventSeq::new(5))],
        "journal must record exact decision at seq 5"
    );

    assert_ticket_and_seq_unchanged(&shard, ticket, seq_after)?;
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
    let err = result
        .err()
        .ok_or("expected shard to propagate journal error")?;
    match err {
        RuntimeError::UnsupportedOperation { operation } => {
            assert_eq!(
                operation, "test_policy_port_failure",
                "exact error message must propagate"
            );
        }
        _ => return Err("wrong typed error propagated".into()),
    }

    let calls = journal.strict_calls()?;
    assert_eq!(
        calls,
        vec![(test_decision_record(ticket), seq_before)],
        "journal must record exact decision at seq before"
    );

    assert_ticket_and_seq_unchanged(&shard, ticket, seq_before)?;
    Ok(())
}

fn journal_bytes_written(shard: &Shard, run: RunId) -> Result<u64, Box<dyn std::error::Error>> {
    shard
        .run_state_get(run)
        .map(|state| state.journal_bytes_written)
        .ok_or("run state must exist".into())
}

fn policy_decision_bytes(
    decision: &DecisionRecord,
    seq: vb_storage::EventSeq,
) -> Result<u64, Box<dyn std::error::Error>> {
    let event = vb_storage::JournalEvent::PolicyDecisionEvent {
        run: decision.ticket.run,
        seq,
        decision: decision.clone(),
    };
    let encoded = postcard::to_allocvec(&event).map_err(|_| "decision event must encode")?;
    u64::try_from(encoded.len()).map_err(|_| "encoded length must fit u64".into())
}

#[test]
fn policy_decision_charges_journal_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(90005);
    let (mut shard, journal, ticket) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let before = journal_bytes_written(&shard, run)?;
    let decision = test_decision_record(ticket);

    shard.append_policy_decision_strict(decision.clone())?;

    let after = journal_bytes_written(&shard, run)?;
    let expected = policy_decision_bytes(&decision, vb_storage::EventSeq::new(5))?;
    assert_eq!(
        after,
        before + expected,
        "counter must grow by exactly the stored decision event bytes"
    );

    let calls = journal.strict_calls()?;
    assert_eq!(
        calls,
        vec![(decision, vb_storage::EventSeq::new(5))],
        "journal must record exact decision at seq 5"
    );
    Ok(())
}

#[test]
fn policy_decision_denied_outcome_charges_journal_bytes() -> Result<(), Box<dyn std::error::Error>>
{
    let run = RunId::new(90006);
    let (mut shard, _journal, ticket) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let before = journal_bytes_written(&shard, run)?;
    let decision = DecisionRecord {
        outcome: DecisionCode::Denied,
        ..test_decision_record(ticket)
    };

    shard.append_policy_decision_strict(decision.clone())?;

    let after = journal_bytes_written(&shard, run)?;
    let expected = policy_decision_bytes(&decision, vb_storage::EventSeq::new(5))?;
    assert_eq!(
        after,
        before + expected,
        "denied decisions must charge journal bytes exactly like allowed ones"
    );
    Ok(())
}

#[test]
fn policy_decision_charges_accumulate() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(90007);
    let (mut shard, _journal, ticket) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let before = journal_bytes_written(&shard, run)?;
    let first = test_decision_record(ticket);
    let second = test_decision_record(ticket);

    shard.append_policy_decision_strict(first.clone())?;
    shard.append_policy_decision_strict(second.clone())?;

    let after = journal_bytes_written(&shard, run)?;
    let expected = policy_decision_bytes(&first, vb_storage::EventSeq::new(5))?
        + policy_decision_bytes(&second, vb_storage::EventSeq::new(6))?;
    assert_eq!(
        after,
        before + expected,
        "two decisions must charge the sum of their stored event bytes"
    );
    Ok(())
}

#[test]
fn policy_decision_counter_overflow_fails_before_append() -> Result<(), Box<dyn std::error::Error>>
{
    let run = RunId::new(90008);
    let (mut shard, journal, ticket) = make_shard_with_journal(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let seq_before = shard.journal_sequence_for(run);
    shard
        .run_state_get_mut(run)
        .ok_or("run state must exist")?
        .journal_bytes_written = u64::MAX;

    let result = shard.append_policy_decision_strict(test_decision_record(ticket));
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

    assert_ticket_and_seq_unchanged(&shard, ticket, seq_before)?;
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

    assert_ticket_and_seq_unchanged(&shard, ticket, seq_max)?;
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
    let err = result
        .err()
        .ok_or("expected shard to reject wrong ticket")?;
    match err {
        RuntimeError::InvalidActionCompletion => {}
        _ => return Err("wrong typed wrong-ticket error".into()),
    }

    let calls = journal.strict_calls()?;
    assert_eq!(
        calls.len(),
        0,
        "journal must not be called for wrong ticket"
    );

    assert_ticket_and_seq_unchanged(&shard, ticket, seq_before)?;
    Ok(())
}
