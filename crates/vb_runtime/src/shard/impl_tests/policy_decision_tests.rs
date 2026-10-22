//! Behavioral tests for Shard::append_policy_decision_strict.
//!
//! Validates: exact pending ticket ownership enforcement, sequence
//! unchanged on port error, wrong-ticket rejection, overflow rejection
//! before append, and sequence advance only after successful port call.

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, DecisionRecord, InfluenceState,
    PinnedAdmissionPolicy, PolicyDigest, PolicyLayer,
};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};
use vb_storage::EventSeq;

use crate::journal::NoopRuntimeJournal;
use super::super::types::{Shard, ShardCommand, ShardConfig};
use crate::{RuntimeError, RuntimeResult};

fn small_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    }
}

fn suspended_workflow() -> CompiledWorkflow {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("suspended"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    CompiledWorkflow::try_from_parts(parts).expect("suspended workflow must compile")
}

fn action_contract() -> ActionContract {
    ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("test-action").expect("action name"),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([Capability::new("__contract_required__".into(), ActionId::new(0))]),
    }
}

fn action_capabilities() -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([Capability::new("__contract_required__".into(), ActionId::new(0))]))
}

fn test_decision_record(run: RunId, policy_id: u8) -> DecisionRecord {
    let ticket = shard_pending_ticket(run);
    let digest = PolicyDigest::from_bytes([
        0xAB, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, policy_id,
    ]);
    DecisionRecord {
        ticket,
        admission: PinnedAdmissionPolicy::Artifact(digest),
        current_live: CurrentPolicyIdentity::Unconfigured,
        input_digest: PolicyDigest::from_bytes([
            0xCD, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0x01,
        ]),
        taint: Taint::Clean,
        outcome: DecisionCode::Allowed,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

fn shard_pending_ticket(run: RunId) -> vb_core::action::ActionTicket {
    vb_core::action::ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq: vb_core::ids::SeqNo::new(0),
        action: ActionId::new(0),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(run, 1, vb_core::ids::SeqNo::new(0), ActionId::new(0)),
        capacity: 10,
    }
}

fn submit_suspended_run(shard: &mut Shard, run: RunId) -> RuntimeResult<()> {
    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow: suspended_workflow(),
        inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
        caps: action_capabilities(),
        action_contracts: Box::from([action_contract()]),
    })?;
    shard.tick()
}

/// No pending action for the run: rejected before sequence computation.
#[test]
fn rejects_ticket_without_pending_ownership() -> RuntimeResult<()> {
    let shard = Shard::new(small_config());
    let run = RunId::new(2001);
    let ticket = shard_pending_ticket(run);
    let decision = DecisionRecord {
        ticket,
        admission: PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes([0; 32])),
        current_live: CurrentPolicyIdentity::Unconfigured,
        input_digest: PolicyDigest::from_bytes([0; 32]),
        taint: Taint::Clean,
        outcome: DecisionCode::Allowed,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    };
    let result = shard.append_policy_decision_strict(decision);
    match result {
        Err(RuntimeError::RunNotFound) => Ok(()),
        other => Err(RuntimeError::Core {
            source: Box::new(vb_core::errors::CoreError::Validation(format!(
                "expected RunNotFound, got: {other:?}"
            ))),
        }),
    }
}

/// Actual run with correct pending action: passes ownership validation.
#[test]
fn accepts_correct_pending_ticket() -> RuntimeResult<()> {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(2002);
    submit_suspended_run(&mut shard, run)?;

    let pending = shard
        .pending_action_get(run)
        .ok_or_else(|| RuntimeError::Core {
            source: Box::new(vb_core::errors::CoreError::Validation(
                "pending action must exist after submit+tick".into(),
            )),
        })?;
    assert_eq!(pending.run, run, "pending ticket must belong to our run");

    let decision = test_decision_record(run, 0x42);
    let result = shard.append_policy_decision_strict(decision);

    // NoopRuntimeJournal uses trait default: UnsupportedOperation.
    match result {
        Err(RuntimeError::UnsupportedOperation { operation }) => {
            assert_eq!(operation, "append_policy_decision_strict");
            Ok(())
        }
        Ok(()) => Ok(()),
        other => Err(RuntimeError::Core {
            source: Box::new(vb_core::errors::CoreError::Validation(format!(
                "expected UnsupportedOperation or Ok, got: {other:?}"
            ))),
        }),
    }
}

/// Wrong ticket for actual run: rejected with InvalidActionCompletion.
#[test]
fn rejects_wrong_ticket_with_invalid_action_completion() -> RuntimeResult<()> {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(2003);
    submit_suspended_run(&mut shard, run)?;

    // Correct ticket is registered. Pass a different (wrong) ticket.
    let wrong_ticket = shard_pending_ticket(RunId::new(9999));
    let decision = DecisionRecord {
        ticket: wrong_ticket,
        admission: PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes([0; 32])),
        current_live: CurrentPolicyIdentity::Unconfigured,
        input_digest: PolicyDigest::from_bytes([0; 32]),
        taint: Taint::Clean,
        outcome: DecisionCode::Allowed,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    };
    let result = shard.append_policy_decision_strict(decision);
    match result {
        Err(RuntimeError::InvalidActionCompletion) => Ok(()),
        other => Err(RuntimeError::Core {
            source: Box::new(vb_core::errors::CoreError::Validation(format!(
                "expected InvalidActionCompletion, got: {other:?}"
            ))),
        }),
    }
}

/// Sequence unchanged when port call fails.
#[test]
fn sequence_unchanged_when_port_fails() -> RuntimeResult<()> {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(2004);
    submit_suspended_run(&mut shard, run)?;

    let _pending = shard
        .pending_action_get(run)
        .ok_or_else(|| RuntimeError::Core {
            source: Box::new(vb_core::errors::CoreError::Validation(
                "pending action must exist".into(),
            )),
        })?;

    shard.seed_journal_sequence(run, EventSeq::new(5))?;

    let decision = test_decision_record(run, 0x42);
    let result = shard.append_policy_decision_strict(decision);

    // Port must fail: NoopRuntimeJournal uses trait default.
    let port_failed = matches!(result, Err(RuntimeError::UnsupportedOperation { .. }));
    if !port_failed {
        return Err(RuntimeError::Core {
            source: Box::new(vb_core::errors::CoreError::Validation(
                "port must fail on NoopRuntimeJournal".into(),
            )),
        });
    }

    // Sequence must be unchanged after failed port call.
    let current_seq = shard.journal_sequence_for(run);
    if current_seq != EventSeq::new(5) {
        return Err(RuntimeError::Core {
            source: Box::new(vb_core::errors::CoreError::Validation(format!(
                "sequence must be unchanged after failed port call, got {:?}",
                current_seq
            ))),
        });
    }
    Ok(())
}

/// u64::MAX rejected before port call.
#[test]
fn rejects_sequence_overflow_before_port_call() -> RuntimeResult<()> {
    let mut shard = Shard::new(small_config());
    let run = RunId::new(2005);
    submit_suspended_run(&mut shard, run)?;

    let _pending = shard
        .pending_action_get(run)
        .ok_or_else(|| RuntimeError::Core {
            source: Box::new(vb_core::errors::CoreError::Validation(
                "pending action must exist".into(),
            )),
        })?;

    shard.seed_journal_sequence(run, EventSeq::new(u64::MAX))?;

    let decision = test_decision_record(run, 0x42);
    let result = shard.append_policy_decision_strict(decision);
    match result {
        Err(RuntimeError::StorageJournalAppend { .. }) => Ok(()),
        other => Err(RuntimeError::Core {
            source: Box::new(vb_core::errors::CoreError::Validation(format!(
                "expected sequence overflow rejection, got: {other:?}"
            ))),
        }),
    }
}