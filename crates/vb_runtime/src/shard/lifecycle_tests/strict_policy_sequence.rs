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
use crate::journal::{RuntimeJournal, RuntimeJournalEvent, SharedRuntimeJournal};
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
    fn shared() -> SharedRuntimeJournal {
        Arc::new(Self::default())
    }

    fn strict_calls(&self) -> Vec<(DecisionRecord, vb_storage::EventSeq)> {
        let calls = self
            .strict_calls
            .lock()
            .map_err(|e| RuntimeError::from(vb_storage::JournalError::JournalPoisoned { source: Box::new(e) }))?;
        Ok(calls.clone())
    }

    fn set_strict_error(&self, err: RuntimeError) {
        let mut slot = self
            .strict_error
            .lock()
            .map_err(|e| RuntimeError::from(vb_storage::JournalError::JournalPoisoned { source: Box::new(e) }))?;
        *slot = Some(err);
    }
}

impl RuntimeJournal for RecordingJournal {
    fn append(&self, _event: RuntimeJournalEvent) -> RuntimeResult<()> {
        Ok(())
    }

    fn append_sequenced(&self, event: RuntimeJournalEvent, _seq: vb_storage::EventSeq) -> RuntimeResult<()> {
        self.append(event)
    }

    fn append_policy_decision_strict(
        &self,
        decision: vb_core::policy::DecisionRecord,
        seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        let mut err_slot = self
            .strict_error
            .lock()
            .map_err(|e| RuntimeError::from(vb_storage::JournalError::JournalPoisoned { source: Box::new(e) }))?;
        let pending_err = err_slot.take();

        let mut calls = self
            .strict_calls
            .lock()
            .map_err(|e| RuntimeError::from(vb_storage::JournalError::JournalPoisoned { source: Box::new(e) }))?;
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
        admission: PinnedAdmissionPolicy::Unverified,
        current_live: CurrentPolicyIdentity::Unconfigured,
        input_digest: PolicyDigest::from_bytes([0; 32]),
        taint: Taint::Clean,
        outcome: DecisionCode::Allowed,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::NotEvaluated,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

// Successful strict port: real shard appends decision at current seq,
// advances internal sequence, leaves pending ticket unchanged.
#[test]
fn successful_strict_policy_port() -> Result<(), Box<dyn std::error::Error>> {
    let journal = RecordingJournal::shared();
    let shard = Shard::new_with_journal(small_config(), journal.clone());
    let run = RunId::new(90001);
    let workflow = require_workflow("suspended", suspended_workflow())?;

    // Submit and tick to reach real pending action state.
    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0)),
        inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
    })?;
    shard.tick()?;

    // Get actual pending ticket from shard.
    let ticket = shard
        .pending_action_get(run)
        .ok_or("pending action ticket must exist")?;

    // Seed sequence at 5 for exact seq verification.
    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let seq_before = shard.journal_sequence_for(run);
    assert_eq!(seq_before.get(), 5, "sequence must be seeded at 5");

    let result = shard.append_policy_decision_strict(test_decision_record(ticket));
    result?;

    // Verify shard advanced internal sequence from 5 to 6.
    let seq_after = shard.journal_sequence_for(run);
    assert_eq!(
        seq_after.get(),
        6,
        "internal sequence must advance from 5 to 6"
    );

    // Verify journal received exactly one call with seq 5.
    let calls = journal.strict_calls()?;
    assert_eq!(calls.len(), 1, "journal must record exactly one strict call");
    assert_eq!(
        calls[0].1, seq_before,
        "journal must receive current shard sequence (5)"
    );
    assert_eq!(
        calls[0].0.ticket, ticket,
        "journal must receive exact ticket"
    );

    // Verify pending ticket unchanged.
    let ticket_after = shard
        .pending_action_get(run)
        .ok_or("pending action ticket must still exist")?;
    assert_eq!(ticket_after, ticket, "pending ticket must remain unchanged");

    Ok(())
}

// Typed strict port failure: journal returns UnsupportedOperation,
// shard propagates error and leaves sequence/pending unchanged.
#[test]
fn typed_strict_policy_port_failure() -> Result<(), Box<dyn std::error::Error>> {
    let journal = RecordingJournal::shared();
    let shard = Shard::new_with_journal(small_config(), journal.clone());
    let run = RunId::new(90002);
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
        .ok_or("pending action ticket must exist")?;

    let seq_before = shard.journal_sequence_for(run);

    journal.set_strict_error(
        RuntimeError::UnsupportedOperation {
            operation: "test_policy_port_failure",
        }
    );

    let result = shard.append_policy_decision_strict(test_decision_record(ticket));
    let err = result.expect_err("shard must propagate journal error");
    match err {
        RuntimeError::UnsupportedOperation { operation } => {
            assert_eq!(
                operation, "test_policy_port_failure",
                "exact error message must propagate"
            );
        }
        _ => panic!("expected UnsupportedOperation error"),
    }

    // Verify shard did NOT advance sequence.
    let seq_after = shard.journal_sequence_for(run);
    assert_eq!(
        seq_after, seq_before,
        "internal sequence must remain unchanged on error"
    );

    // Verify pending ticket unchanged.
    let ticket_after = shard
        .pending_action_get(run)
        .ok_or("pending action ticket must still exist")?;
    assert_eq!(ticket_after, ticket, "pending ticket must remain unchanged");

    Ok(())
}

// Overflow at u64::MAX: shard's checked arithmetic rejects before journal
// call. No journal call, sequence unchanged.
#[test]
fn overflow_at_u64_max() -> Result<(), Box<dyn std::error::Error>> {
    let journal = RecordingJournal::shared();
    let shard = Shard::new_with_journal(small_config(), journal.clone());
    let run = RunId::new(90003);
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
        .ok_or("pending action ticket must exist")?;

    // Seed sequence at u64::MAX so checked_next_seq overflows.
    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(u64::MAX))?;

    let seq_max = shard.journal_sequence_for(run);
    assert_eq!(
        seq_max.get(),
        u64::MAX,
        "sequence must be at u64::MAX before overflow test"
    );

    let result = shard.append_policy_decision_strict(test_decision_record(ticket));
    let err = result.expect_err("shard must detect sequence overflow");
    match err {
        RuntimeError::StorageJournalAppend { source } => {
            // Check source is JournalError::SequenceOverflow
            let source_str = source.to_string();
            assert!(
                source_str.contains("overflow") || source_str.contains("Overflow"),
                "error source must reference sequence overflow"
            );
        }
        _ => panic!("expected StorageJournalAppend error"),
    }

    // Verify no journal call occurred (overflow rejected before port).
    let calls = journal.strict_calls()?;
    assert_eq!(
        calls.len(), 0,
        "journal must not be called when sequence overflows"
    );

    // Verify sequence unchanged.
    let seq_after = shard.journal_sequence_for(run);
    assert_eq!(
        seq_after, seq_max,
        "internal sequence must remain at u64::MAX"
    );

    Ok(())
}

// Wrong ticket for existing run: attempt to record decision for an
// action that is not the current pending ticket.
#[test]
fn wrong_ticket_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let journal = RecordingJournal::shared();
    let shard = Shard::new_with_journal(small_config(), journal.clone());
    let run = RunId::new(90004);
    let workflow = require_workflow("suspended", suspended_workflow())?;

    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        caps: crate::shard::tests::test_fixtures::contract_capability_for(ActionId::new(0)),
        action_contracts: crate::shard::tests::test_fixtures::contracts_through(ActionId::new(0)),
        inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
    })?;
    shard.tick()?;

    let real_ticket = shard
        .pending_action_get(run)
        .ok_or("pending action ticket must exist")?;

    let seq_before = shard.journal_sequence_for(run);

    // Build a wrong ticket: change the action ID (different action entirely).
    let wrong_ticket = ActionTicket {
        action: ActionId::new(999),
        ..real_ticket
    };

    let result = shard.append_policy_decision_strict(test_decision_record(wrong_ticket));
    let err = result.expect_err("shard must reject wrong ticket");
    match err {
        RuntimeError::InvalidActionCompletion => {
            // Expected: ticket does not match pending action
        }
        _ => panic!("expected InvalidActionCompletion error"),
    }

    // Verify no journal call occurred.
    let calls = journal.strict_calls()?;
    assert_eq!(
        calls.len(), 0,
        "journal must not be called for wrong ticket"
    );

    // Verify sequence and pending ticket unchanged.
    let seq_after = shard.journal_sequence_for(run);
    assert_eq!(
        seq_after, seq_before,
        "internal sequence must remain unchanged"
    );

    let ticket_after = shard
        .pending_action_get(run)
        .ok_or("pending action ticket must still exist")?;
    assert_eq!(ticket_after, real_ticket, "pending ticket must remain unchanged");

    Ok(())
}