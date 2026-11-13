use std::sync::{Arc, Mutex};

use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, BoundGateOutcomeRecord, BoundManualGate, GateBinding, GateDecision,
    GateDecisionInput, GateDigest, GateId, GateOutcome, ManualGate, OperatorTrust, RiskClass,
    SeparationOfDuties,
};
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};

use crate::RuntimeError;
use crate::error::RuntimeResult;
use crate::journal::{RuntimeJournal, RuntimeJournalEvent};
use crate::shard::types::{RuntimeState, Shard, ShardCommand};

use super::{require_workflow, small_config};

#[derive(Debug, Default)]
struct BoundJournal {
    decisions: Mutex<Vec<BoundGateOutcomeRecord>>,
    opened: Mutex<Vec<(BoundManualGate, u64, ApproverId)>>,
}

impl BoundJournal {
    fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn decisions(&self) -> RuntimeResult<Vec<BoundGateOutcomeRecord>> {
        self.decisions
            .lock()
            .map(|calls| calls.clone())
            .map_err(|_| RuntimeError::JournalPoisoned)
    }

    fn opened(&self) -> RuntimeResult<Vec<(BoundManualGate, u64, ApproverId)>> {
        self.opened
            .lock()
            .map(|calls| calls.clone())
            .map_err(|_| RuntimeError::JournalPoisoned)
    }
}

impl RuntimeJournal for BoundJournal {
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
        _record: vb_core::manual_gate::GateOutcomeRecord,
        _seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    fn append_gate_opened_strict(
        &self,
        _gate: ManualGate,
        _opened_at_ms: u64,
        _requester: ApproverId,
        _seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    fn append_gate_decision_bound_strict(
        &self,
        record: BoundGateOutcomeRecord,
        _seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        self.decisions
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push(record);
        Ok(())
    }

    fn append_gate_opened_bound_strict(
        &self,
        gate: BoundManualGate,
        opened_at_ms: u64,
        requester: ApproverId,
        _seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        self.opened
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push((gate, opened_at_ms, requester));
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
    ApproverRole::new("deploy-approver".to_string()).expect("role")
}

fn person(value: &str) -> ApproverId {
    ApproverId::new(value.to_string()).expect("person")
}

fn bound_gate(run: RunId) -> BoundManualGate {
    BoundManualGate {
        gate: ManualGate {
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
        },
        binding: GateBinding {
            artifact_digest: digest(4),
            policy_digest: digest(5),
            input_digest: digest(6),
            attempt: 1,
        },
    }
}

fn bound_gate_for_state(shard: &Shard, run: RunId) -> BoundManualGate {
    let state = shard.run_state_get(run).expect("run state");
    let admission = state.admission.as_ref().expect("admission");
    let input_digest = match admission.input_digest() {
        vb_storage::RunInputBinding::Digest(digest) => digest,
        vb_storage::RunInputBinding::LegacyUnbound => panic!("submitted run has no input digest"),
    };
    let mut gate = bound_gate(run);
    gate.binding = GateBinding {
        artifact_digest: GateDigest::from_bytes(admission.artifact_digest().as_bytes()),
        policy_digest: digest(5),
        input_digest: GateDigest::from_bytes(input_digest.as_bytes()),
        attempt: state.run_attempt.get(),
    };
    gate
}

fn input(decision: GateDecision) -> GateDecisionInput {
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
        source_metadata: None,
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

fn make_shard(run: RunId) -> Result<(Shard, Arc<BoundJournal>), Box<dyn std::error::Error>> {
    let journal = BoundJournal::shared();
    let mut shard = Shard::new_with_journal(small_config(), journal.clone());
    let workflow = require_workflow("chain", chain_workflow())?;
    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        caps: CapabilitySet::empty(),
        action_contracts: Box::from([]),
        inputs: Box::from([]),
    })?;
    shard.tick()?;
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    shard.enqueue(ShardCommand::OpenBoundGate {
        gate: bound_gate_for_state(&shard, run),
        opened_at_ms: 0,
        requester: person("bob"),
    })?;
    shard.tick()?;
    Ok((shard, journal))
}

#[test]
fn open_bound_gate_rejects_each_zero_digest_before_journal() -> Result<(), Box<dyn std::error::Error>>
{
    for field in 0..3 {
        let run = RunId::new(94010 + field);
        let journal = BoundJournal::shared();
        let mut shard = Shard::new_with_journal(small_config(), journal.clone());
        let workflow = require_workflow("chain", chain_workflow())?;
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            caps: CapabilitySet::empty(),
            action_contracts: Box::from([]),
            inputs: Box::from([]),
        })?;
        shard.tick()?;
        let mut gate = bound_gate(run);
        match field {
            0 => gate.binding.artifact_digest = GateDigest::from_bytes([0; 32]),
            1 => gate.binding.policy_digest = GateDigest::from_bytes([0; 32]),
            _ => gate.binding.input_digest = GateDigest::from_bytes([0; 32]),
        }
        shard.enqueue(ShardCommand::OpenBoundGate {
            gate,
            opened_at_ms: 0,
            requester: person("bob"),
        })?;
        let err = shard.tick().unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGateOpenedRecord));
        assert!(journal.opened()?.is_empty());
        assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    }
    Ok(())
}

#[test]
fn resume_rejects_bound_gate_without_decision() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94009);
    let (mut shard, journal) = make_shard(run)?;
    let err = shard
        .handle_resume(run)
        .expect_err("bound gate must block resume");
    assert!(matches!(
        err,
        crate::shard::types::ResumeError::GateOpen { gate, .. } if gate == GateId::new(7)
    ));
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Resumable));
    assert!(journal.decisions()?.is_empty(), "resume must not journal");
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_open_and_approve_journals_bound_record_and_closes_gate()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94001);
    let (mut shard, journal) = make_shard(run)?;
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Resumable));
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));

    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: input(GateDecision::Approve),
        operator_trust: OperatorTrust::LocalSocketClaim,
    })?;
    shard.tick()?;

    let decisions = journal.decisions()?;
    assert_eq!(decisions.len(), 1);
    assert!(decisions[0].is_structurally_valid());
    assert_eq!(decisions[0].record.outcome, GateOutcome::Approved);
    assert_eq!(
        decisions[0].operator_trust,
        Some(OperatorTrust::LocalSocketClaim)
    );
    assert_eq!(
        decisions[0].binding,
        bound_gate_for_state(&shard, run).binding
    );
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn legacy_and_bound_paths_reject_each_others_gates() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94002);
    let (mut shard, _journal) = make_shard(run)?;
    shard.enqueue(ShardCommand::SubmitGateDecision {
        run,
        input: input(GateDecision::Approve),
    })?;
    let err = shard.tick().unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::BoundGateRequired { gate: 7 }
        }
    ));
    Ok(())
}

#[test]
fn bound_path_rejects_legacy_gate_without_binding() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94004);
    let journal = BoundJournal::shared();
    let mut shard = Shard::new_with_journal(small_config(), journal);
    let workflow = require_workflow("chain", chain_workflow())?;
    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        caps: CapabilitySet::empty(),
        action_contracts: Box::from([]),
        inputs: Box::from([]),
    })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::OpenGate {
        gate: bound_gate(run).gate,
        opened_at_ms: 0,
        requester: person("bob"),
    })?;
    shard.tick()?;
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: input(GateDecision::Approve),
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    let err = shard.tick().unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::LegacyGateWithoutBinding { gate: 7 }
        }
    ));
    Ok(())
}

#[test]
fn bound_deny_fails_run_without_receipt() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94005);
    let (mut shard, journal) = make_shard(run)?;
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: input(GateDecision::Deny),
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    shard.tick()?;
    let record = &journal.decisions()?[0];
    assert_eq!(record.record.outcome, GateOutcome::Denied);
    assert!(record.bound_receipt.is_none());
    assert!(!shard.run_state_contains(run));
    assert!(shard.terminal_runs_contains(run));
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_more_evidence_keeps_gate_and_original_deadline() -> Result<(), Box<dyn std::error::Error>>
{
    let run = RunId::new(94006);
    let (mut shard, journal) = make_shard(run)?;
    let mut more = input(GateDecision::RequestMoreEvidence);
    more.decided_at_ms = 1_000;
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: more,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    shard.tick()?;
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Resumable));
    let mut late = input(GateDecision::Approve);
    late.decided_at_ms = 1_001;
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: late,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    let err = shard.tick().unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::DecisionAfterTimeout { .. }
        }
    ));
    assert_eq!(
        journal.decisions()?.len(),
        1,
        "late decision is not journaled"
    );
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_delegate_restricts_next_approver() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94007);
    let (mut shard, _journal) = make_shard(run)?;
    let mut delegate = input(GateDecision::Delegate);
    delegate.delegated_to = Some(person("carol"));
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: delegate,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    shard.tick()?;
    let wrong = input(GateDecision::Approve);
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: wrong,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    let err = shard.tick().unwrap_err();
    assert!(matches!(err, RuntimeError::GateWrongDelegate { .. }));
    let mut right = input(GateDecision::Approve);
    right.approver = person("carol");
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: right,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    shard.tick()?;
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_timeout_escalation_fails_run_without_trust_or_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94008);
    let (mut shard, journal) = make_shard(run)?;
    shard.enqueue(ShardCommand::EscalateBoundGate {
        run,
        gate: GateId::new(7),
        now_ms: 1_001,
    })?;
    shard.tick()?;
    let record = &journal.decisions()?[0];
    assert_eq!(record.record.outcome, GateOutcome::Escalated);
    assert!(record.operator_trust.is_none());
    assert!(record.bound_receipt.is_none());
    assert!(!shard.run_state_contains(run));
    assert!(shard.terminal_runs_contains(run));
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}

#[test]
fn bound_break_glass_requires_evidence_before_journal() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94003);
    let (mut shard, journal) = make_shard(run)?;
    let mut break_glass = input(GateDecision::BreakGlass);
    break_glass.break_glass_justification = Some("prod is down".to_string());
    break_glass.evidence_shown = false;
    shard.enqueue(ShardCommand::SubmitBoundGateDecision {
        run,
        input: break_glass,
        operator_trust: OperatorTrust::InProcessCaller,
    })?;
    let err = shard.tick().unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::EvidenceNotShown
        }
    ));
    assert!(journal.decisions()?.is_empty());
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    Ok(())
}
