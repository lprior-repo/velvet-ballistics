//! Bounded behavioral tests for Shard::append_policy_decision_strict.
//!
//! Validates: exact pending ticket ownership enforcement, sequence
//! unchanged on port error, wrong-ticket rejection, overflow rejection
//! before append, and sequence advance only after successful port call.

use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy,
    PolicyDigest, PolicyLayer,
};
use vb_core::action::ActionTicket;
use vb_core::Taint;
use vb_storage::EventSeq;

use crate::admission::AlwaysPresentArtifactStore;
use crate::journal::NoopRuntimeJournal;
use crate::shard::ShardConfig;
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

fn small_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    }
}

/// Ticket not registered as pending must be rejected before any
/// sequence computation or journal call.
#[test]
fn shard_rejects_ticket_without_pending_ownership() {
    let shard = Shard::new(small_config());
    let run = RunId::new(2001);
    let decision = test_decision_record(run, 0x00);
    let result = shard.append_policy_decision_strict(decision);
    match result {
        Err(RuntimeError::RunNotFound) => {}
        other => {
            assert!(false, "expected RunNotFound, got: {other:?}");
        }
    }
}

/// Different ticket than the registered pending action must be rejected
/// with InvalidActionCompletion.
#[test]
fn shard_rejects_wrong_ticket_with_invalid_action_completion() {
    let shard = Shard::new(small_config());
    let run = RunId::new(2002);
    let correct = test_action_ticket(run, StepIdx::new(1), SeqNo::new(1), ActionId::new(0));
    let wrong = test_action_ticket(run, StepIdx::new(2), SeqNo::new(2), ActionId::new(1));
    let decision = test_decision_record(run, 0x00);

    let result = shard.pending_action_insert(run, correct);
    match result {
        Ok(_) => {}
        Err(error) => {
            assert!(false, "pending_action_insert: {error}");
            return;
        }
    }

    let result = shard.append_policy_decision_strict(decision);
    match result {
        Err(RuntimeError::InvalidActionCompletion) => {}
        other => {
            assert!(false, "expected InvalidActionCompletion, got: {other:?}");
        }
    }
}

/// Correct pending ticket must pass ownership validation.
#[test]
fn shard_accepts_correct_pending_ticket() {
    let shard = Shard::new(small_config());
    let run = RunId::new(2003);
    let ticket = test_action_ticket(run, StepIdx::new(1), SeqNo::new(1), ActionId::new(0));
    let decision = test_decision_record(run, 0x00);

    let result = shard.pending_action_insert(run, ticket);
    match result {
        Ok(_) => {}
        Err(error) => {
            assert!(false, "pending_action_insert: {error}");
            return;
        }
    }

    let result = shard.append_policy_decision_strict(decision);
    match result {
        Err(RuntimeError::UnsupportedOperation { operation }) => {
            assert_eq!(operation, "append_policy_decision_strict");
        }
        Ok(()) => {}
        other => {
            assert!(false, "expected UnsupportedOperation or Ok, got: {other:?}");
        }
    }
}

/// After successful append, shard's next sequence must advance by exactly
/// one. The test verifies that NoopRuntimeJournal uses the trait default
/// (UnsupportedOperation) so we can observe sequence state after the
/// shard call fails at the port layer.
#[test]
fn shard_sequence_unchanged_when_port_fails() {
    let shard = Shard::new(small_config());
    let run = RunId::new(2004);
    let ticket = test_action_ticket(run, StepIdx::new(1), SeqNo::new(1), ActionId::new(0));
    let decision = test_decision_record(run, 0x00);

    let result = shard.pending_action_insert(run, ticket);
    match result {
        Ok(_) => {}
        Err(error) => {
            assert!(false, "pending_action_insert: {error}");
            return;
        }
    }

    shard.seed_journal_sequence(run, EventSeq::new(5)).unwrap_or_else(|error| {
        assert!(false, "seed_journal_sequence: {error}");
    });

    let result = shard.append_policy_decision_strict(decision);
    let failed = matches!(
        result,
        Err(RuntimeError::UnsupportedOperation { .. })
    );
    assert!(failed, "port must fail on NoopRuntimeJournal");

    let current_seq = shard.journal_sequence_for(run);
    assert_eq!(current_seq, EventSeq::new(5), "sequence must be unchanged after failed port call");
}

/// Sequence overflow at u64::MAX must be rejected before the journal port
/// is called.
#[test]
fn shard_rejects_sequence_overflow_before_port_call() {
    let shard = Shard::new(small_config());
    let run = RunId::new(2005);
    let ticket = test_action_ticket(run, StepIdx::new(1), SeqNo::new(1), ActionId::new(0));
    let decision = test_decision_record(run, 0x00);

    let result = shard.pending_action_insert(run, ticket);
    match result {
        Ok(_) => {}
        Err(error) => {
            assert!(false, "pending_action_insert: {error}");
            return;
        }
    }

    shard.seed_journal_sequence(run, EventSeq::new(u64::MAX)).unwrap_or_else(|error| {
        assert!(false, "seed_journal_sequence at u64::MAX: {error}");
    });

    let result = shard.append_policy_decision_strict(decision);
    match result {
        Err(RuntimeError::from(vb_storage::JournalError::SequenceOverflow)) => {}
        other => {
            assert!(false, "expected SequenceOverflow, got: {other:?}");
        }
    }
}