use std::sync::{Arc, Mutex};

use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, GateDecision, GateDecisionInput, GateDigest, GateId, GateOutcome,
    GateOutcomeRecord, ManualGate, RiskClass, SeparationOfDuties,
};
use vb_core::value::{ConstValue, SlotValue};
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};

use crate::RuntimeError;
use crate::error::RuntimeResult;
use crate::journal::{RuntimeJournal, RuntimeJournalEvent};
use crate::shard::types::{RuntimeState, Shard, ShardCommand};

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

fn test_input(id: u64, decision: GateDecision) -> GateDecisionInput {
    GateDecisionInput {
        gate: GateId::new(id),
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

fn chain_workflow() -> Option<CompiledWorkflow> {
    let mut nodes = Vec::new();
    for index in 0..9_u16 {
        nodes.push(vb_core::workflow::node(
            StepIdx::new(index),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(index + 1)),
        ));
    }
    nodes.push(vb_core::workflow::node(
        StepIdx::new(9),
        CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
        None,
        None,
    ));
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("chain_then_finish"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(0)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

fn make_running_shard(
    run: RunId,
) -> Result<(Shard, Arc<RecordingJournal>), Box<dyn std::error::Error>> {
    let journal = RecordingJournal::shared();
    let mut shard = Shard::new_with_journal(small_config(), journal.clone());
    let workflow = require_workflow("chain", chain_workflow())?;
    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        caps: CapabilitySet::empty(),
        action_contracts: Box::from([]),
        inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
    })?;
    shard.tick()?;
    let state = shard
        .runtime_state_get(run)
        .ok_or("run must have runtime state")?;
    assert_eq!(
        state,
        RuntimeState::Running,
        "chain run must stay Running after the first drive"
    );
    Ok((shard, journal))
}

fn make_suspended_shard(
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
    let state = shard
        .runtime_state_get(run)
        .ok_or("run must have runtime state")?;
    assert_eq!(
        state,
        RuntimeState::Resumable,
        "suspended run must be Resumable after the first drive"
    );
    Ok((shard, journal))
}

fn open_test_gate(
    shard: &mut Shard,
    run: RunId,
    id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    shard.open_gate(test_gate(run, id), 0, person("bob"))?;
    Ok(())
}

fn journal_bytes_written(shard: &Shard, run: RunId) -> Result<u64, Box<dyn std::error::Error>> {
    shard
        .run_state_get(run)
        .map(|state| state.journal_bytes_written)
        .ok_or("run state must exist".into())
}

#[test]
fn open_gate_suspends_running_run() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92001);
    let (mut shard, _journal) = make_running_shard(run)?;

    open_test_gate(&mut shard, run, 7)?;

    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Resumable),
        "run must be suspended after the gate opens"
    );
    assert_eq!(
        shard.open_gate_for(run),
        Some(GateId::new(7)),
        "open gate must be registered for the run"
    );
    Ok(())
}

#[test]
fn open_gate_rejects_duplicate_gate_id() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92002);
    let (mut shard, _journal) = make_running_shard(run)?;

    open_test_gate(&mut shard, run, 7)?;
    let err = shard
        .open_gate(test_gate(run, 7), 0, person("bob"))
        .err()
        .ok_or("expected shard to reject a duplicate gate id")?;
    match err {
        RuntimeError::GateAlreadyOpen { gate } => {
            assert_eq!(gate, GateId::new(7));
        }
        _ => return Err("wrong typed duplicate gate error".into()),
    }
    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Resumable),
        "run must stay suspended after the rejected duplicate"
    );
    Ok(())
}

#[test]
fn open_gate_rejects_missing_run() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92003);
    let (mut shard, _journal) = make_running_shard(run)?;

    let err = shard
        .open_gate(test_gate(RunId::new(99999), 7), 0, person("bob"))
        .err()
        .ok_or("expected shard to reject a gate for a missing run")?;
    match err {
        RuntimeError::RunNotFound => {}
        _ => return Err("wrong typed missing run error".into()),
    }
    assert_eq!(
        shard.open_gate_for(RunId::new(99999)),
        None,
        "no gate must be registered for the missing run"
    );
    Ok(())
}

#[test]
fn open_gate_rejects_suspended_run() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92004);
    let (mut shard, _journal) = make_suspended_shard(run)?;

    let err = shard
        .open_gate(test_gate(run, 7), 0, person("bob"))
        .err()
        .ok_or("expected shard to reject a gate on a suspended run")?;
    match err {
        RuntimeError::GateRunNotSuspendable { run: err_run, .. } => {
            assert_eq!(err_run, run);
        }
        _ => return Err("wrong typed suspended run error".into()),
    }
    assert_eq!(
        shard.open_gate_for(run),
        None,
        "no gate must be registered for the suspended run"
    );
    Ok(())
}

#[test]
fn resume_blocked_while_gate_open() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92005);
    let (mut shard, _journal) = make_running_shard(run)?;

    open_test_gate(&mut shard, run, 7)?;
    shard.enqueue(ShardCommand::Resume { run })?;
    let err = shard
        .tick()
        .err()
        .ok_or("expected the resume tick to fail while the gate is open")?;
    match err {
        RuntimeError::GateOpenBlocksResume { run: err_run, gate } => {
            assert_eq!(err_run, run);
            assert_eq!(gate, GateId::new(7));
        }
        _ => return Err("wrong typed blocked resume error".into()),
    }
    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Resumable),
        "run must stay suspended after the blocked resume"
    );
    assert_eq!(
        shard.open_gate_for(run),
        Some(GateId::new(7)),
        "gate must stay open after the blocked resume"
    );
    Ok(())
}

#[test]
fn submit_approve_resumes_run() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92006);
    let (mut shard, journal) = make_running_shard(run)?;

    open_test_gate(&mut shard, run, 7)?;
    let record = shard.submit_gate_decision(test_input(7, GateDecision::Approve))?;

    assert_eq!(record.outcome, GateOutcome::Approved);
    assert!(record.receipt.is_some(), "approval must carry a receipt");
    assert!(record.is_structurally_valid());
    assert_eq!(
        shard.open_gate_for(run),
        None,
        "gate must close on approval"
    );
    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Running),
        "run must resume after approval"
    );
    let calls = journal.strict_calls()?;
    assert_eq!(
        calls.len(),
        1,
        "approval must journal exactly one decision event"
    );
    assert_eq!(calls[0].0, record, "journaled record must match the returned record");
    Ok(())
}

#[test]
fn submit_deny_fails_run() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92007);
    let (mut shard, journal) = make_running_shard(run)?;

    open_test_gate(&mut shard, run, 7)?;
    let record = shard.submit_gate_decision(test_input(7, GateDecision::Deny))?;

    assert_eq!(record.outcome, GateOutcome::Denied);
    assert!(record.receipt.is_none(), "denial must carry no receipt");
    assert_eq!(
        shard.open_gate_for(run),
        None,
        "gate must close on denial"
    );
    assert!(
        !shard.run_state_contains(run),
        "run state must be gone after denial"
    );
    assert!(
        shard.terminal_runs_contains(run),
        "run must be terminal after denial"
    );
    let calls = journal.strict_calls()?;
    assert_eq!(
        calls.len(),
        1,
        "denial must journal exactly one decision event"
    );
    assert_eq!(calls[0].0, record, "journaled record must match the returned record");
    Ok(())
}

#[test]
fn submit_more_evidence_keeps_gate_open() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92008);
    let (mut shard, journal) = make_running_shard(run)?;

    open_test_gate(&mut shard, run, 7)?;
    let record = shard.submit_gate_decision(test_input(7, GateDecision::RequestMoreEvidence))?;

    assert_eq!(record.outcome, GateOutcome::MoreEvidenceRequested);
    assert!(record.receipt.is_none());
    assert_eq!(
        shard.open_gate_for(run),
        Some(GateId::new(7)),
        "gate must stay open after a more-evidence request"
    );
    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Resumable),
        "run must stay suspended after a more-evidence request"
    );
    assert_eq!(
        journal.strict_calls()?.len(),
        1,
        "more-evidence request must still journal its decision"
    );
    Ok(())
}

#[test]
fn submit_delegate_restricts_next_approver() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92009);
    let (mut shard, _journal) = make_running_shard(run)?;

    open_test_gate(&mut shard, run, 7)?;
    let mut delegate = test_input(7, GateDecision::Delegate);
    delegate.delegated_to = Some(person("carol"));
    let record = shard.submit_gate_decision(delegate)?;
    assert_eq!(record.outcome, GateOutcome::Delegated);
    assert_eq!(record.delegated_to, Some(person("carol")));
    assert_eq!(
        shard.open_gate_for(run),
        Some(GateId::new(7)),
        "gate must stay open after delegation"
    );

    let err = shard
        .submit_gate_decision(test_input(7, GateDecision::Approve))
        .err()
        .ok_or("expected the original approver to be rejected after delegation")?;
    match err {
        RuntimeError::GateWrongDelegate {
            gate,
            expected,
            actual,
        } => {
            assert_eq!(gate, GateId::new(7));
            assert_eq!(expected, person("carol"));
            assert_eq!(actual, person("alice"));
        }
        _ => return Err("wrong typed delegate error".into()),
    }

    let mut carol = test_input(7, GateDecision::Approve);
    carol.approver = person("carol");
    let record = shard.submit_gate_decision(carol)?;
    assert_eq!(record.outcome, GateOutcome::Approved);
    assert_eq!(
        shard.open_gate_for(run),
        None,
        "gate must close after the delegate approves"
    );
    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Running),
        "run must resume after the delegate approves"
    );
    Ok(())
}

#[test]
fn submit_unknown_gate_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92010);
    let (mut shard, _journal) = make_running_shard(run)?;

    let err = shard
        .submit_gate_decision(test_input(7, GateDecision::Approve))
        .err()
        .ok_or("expected a decision for an unknown gate to be rejected")?;
    match err {
        RuntimeError::GateNotOpen { gate } => {
            assert_eq!(gate, GateId::new(7));
        }
        _ => return Err("wrong typed unknown gate error".into()),
    }
    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Running),
        "run must keep running after a rejected unknown-gate decision"
    );
    Ok(())
}

#[test]
fn submit_wrong_role_rejected_gate_stays_open() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92011);
    let (mut shard, journal) = make_running_shard(run)?;

    open_test_gate(&mut shard, run, 7)?;
    let bytes_before = journal_bytes_written(&shard, run)?;
    let mut bad = test_input(7, GateDecision::Approve);
    bad.approver_role = ApproverRole::new("viewer".to_string()).unwrap();
    let err = shard
        .submit_gate_decision(bad)
        .err()
        .ok_or("expected a wrong-role decision to be rejected")?;
    match err {
        RuntimeError::GateEvaluationFailed { source } => {
            assert!(
                matches!(source, crate::manual_gate::GateError::WrongRole { .. }),
                "source must be the wrong-role evaluation error"
            );
        }
        _ => return Err("wrong typed evaluation error".into()),
    }
    assert_eq!(
        shard.open_gate_for(run),
        Some(GateId::new(7)),
        "gate must stay open after a rejected decision"
    );
    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Resumable),
        "run must stay suspended after a rejected decision"
    );
    assert_eq!(
        journal.strict_calls()?.len(),
        0,
        "rejected decision must not reach the journal"
    );
    assert_eq!(
        journal_bytes_written(&shard, run)?,
        bytes_before,
        "counter must not advance for a rejected decision"
    );
    Ok(())
}

#[test]
fn submit_after_deadline_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92012);
    let (mut shard, _journal) = make_running_shard(run)?;

    open_test_gate(&mut shard, run, 7)?;
    let mut late = test_input(7, GateDecision::Approve);
    late.decided_at_ms = 1_001;
    let err = shard
        .submit_gate_decision(late)
        .err()
        .ok_or("expected a late decision to be rejected")?;
    match err {
        RuntimeError::GateEvaluationFailed { source } => {
            assert!(
                matches!(
                    source,
                    crate::manual_gate::GateError::DecisionAfterTimeout { .. }
                ),
                "source must be the after-timeout evaluation error"
            );
        }
        _ => return Err("wrong typed late decision error".into()),
    }
    assert_eq!(
        shard.open_gate_for(run),
        Some(GateId::new(7)),
        "gate must stay open after a late decision"
    );
    Ok(())
}

#[test]
fn escalate_timed_out_gate_fails_run() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92013);
    let (mut shard, journal) = make_running_shard(run)?;

    open_test_gate(&mut shard, run, 7)?;
    let record = shard.escalate_timed_out_gate(GateId::new(7), 2_000)?;

    assert_eq!(record.outcome, GateOutcome::Escalated);
    assert_eq!(record.decision, None);
    assert!(record.receipt.is_none());
    assert_eq!(
        shard.open_gate_for(run),
        None,
        "gate must close on escalation"
    );
    assert!(
        !shard.run_state_contains(run),
        "run state must be gone after escalation"
    );
    assert!(
        shard.terminal_runs_contains(run),
        "run must be terminal after escalation"
    );
    let calls = journal.strict_calls()?;
    assert_eq!(
        calls.len(),
        1,
        "escalation must journal exactly one decision event"
    );
    assert_eq!(calls[0].0, record, "journaled record must match the returned record");
    Ok(())
}

#[test]
fn escalate_before_deadline_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92014);
    let (mut shard, _journal) = make_running_shard(run)?;

    open_test_gate(&mut shard, run, 7)?;
    let err = shard
        .escalate_timed_out_gate(GateId::new(7), 1_000)
        .err()
        .ok_or("expected escalation before the deadline to be rejected")?;
    match err {
        RuntimeError::GateEvaluationFailed { source } => {
            assert_eq!(
                source,
                crate::manual_gate::GateError::NotTimedOut,
                "source must be the not-timed-out evaluation error"
            );
        }
        _ => return Err("wrong typed early escalation error".into()),
    }
    assert_eq!(
        shard.open_gate_for(run),
        Some(GateId::new(7)),
        "gate must stay open after a rejected escalation"
    );
    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Resumable),
        "run must stay suspended after a rejected escalation"
    );
    Ok(())
}

#[test]
fn journal_failure_leaves_gate_open() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92015);
    let (mut shard, journal) = make_running_shard(run)?;

    shard.seed_journal_sequence(run, vb_storage::EventSeq::new(5))?;
    let seq_before = shard.journal_sequence_for(run);
    open_test_gate(&mut shard, run, 7)?;
    let bytes_before = journal_bytes_written(&shard, run)?;

    journal.set_strict_error(RuntimeError::UnsupportedOperation {
        operation: "test_gate_journal_failure",
    })?;
    let err = shard
        .submit_gate_decision(test_input(7, GateDecision::Approve))
        .err()
        .ok_or("expected the journal failure to propagate")?;
    match err {
        RuntimeError::UnsupportedOperation { operation } => {
            assert_eq!(operation, "test_gate_journal_failure");
        }
        _ => return Err("wrong typed journal error propagated".into()),
    }
    assert_eq!(
        shard.open_gate_for(run),
        Some(GateId::new(7)),
        "gate must stay open when journaling fails"
    );
    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Resumable),
        "run must stay suspended when journaling fails"
    );
    assert_eq!(
        journal_bytes_written(&shard, run)?,
        bytes_before,
        "counter must not advance when journaling fails"
    );
    assert_eq!(
        shard.journal_sequence_for(run),
        seq_before,
        "sequence must not advance when journaling fails"
    );
    Ok(())
}

#[test]
fn submit_break_glass_approves_without_role() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92016);
    let (mut shard, _journal) = make_running_shard(run)?;

    open_test_gate(&mut shard, run, 7)?;
    let mut input = test_input(7, GateDecision::BreakGlass);
    input.approver_role = ApproverRole::new("viewer".to_string()).unwrap();
    input.break_glass_justification = Some("prod is down".to_string());
    let record = shard.submit_gate_decision(input)?;

    assert_eq!(record.outcome, GateOutcome::BreakGlassApproved);
    assert!(record.break_glass);
    assert!(record.receipt.is_some(), "break-glass must carry a receipt");
    assert_eq!(
        shard.open_gate_for(run),
        None,
        "gate must close on break-glass approval"
    );
    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Running),
        "run must resume after break-glass approval"
    );
    Ok(())
}

#[test]
fn command_queue_flow_opens_and_decides_gate() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(92017);
    let (mut shard, journal) = make_running_shard(run)?;

    shard.enqueue(ShardCommand::OpenGate {
        gate: test_gate(run, 7),
        opened_at_ms: 0,
        requester: person("bob"),
    })?;
    assert!(shard.tick()?, "open-gate tick must continue");
    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Resumable),
        "run must be suspended after the open-gate command"
    );

    shard.enqueue(ShardCommand::SubmitGateDecision {
        input: test_input(7, GateDecision::Approve),
    })?;
    assert!(shard.tick()?, "submit-decision tick must continue");
    assert_eq!(
        shard.open_gate_for(run),
        None,
        "gate must close after the submit-decision command"
    );
    assert_eq!(
        shard.runtime_state_get(run),
        Some(RuntimeState::Running),
        "run must resume after the submit-decision command"
    );
    let calls = journal.strict_calls()?;
    assert_eq!(
        calls.len(),
        1,
        "command flow must journal exactly one decision event"
    );
    assert_eq!(
        calls[0].0.outcome,
        GateOutcome::Approved,
        "journaled outcome must be approval"
    );
    Ok(())
}
