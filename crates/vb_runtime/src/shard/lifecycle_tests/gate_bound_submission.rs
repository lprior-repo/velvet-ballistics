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
    opened: Mutex<Vec<(BoundManualGate, u64, ApproverId, u64)>>,
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

    fn opened(&self) -> RuntimeResult<Vec<(BoundManualGate, u64, ApproverId, u64)>> {
        self.opened
            .lock()
            .map(|calls| calls.clone())
            .map_err(|_| RuntimeError::JournalPoisoned)
    }
}

impl RuntimeJournal for BoundJournal {
    fn durability_profile(&self) -> Option<vb_storage::DurabilityProfile> {
        Some(vb_storage::DurabilityProfile::Volatile)
    }

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
        seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        self.opened
            .lock()
            .map_err(|_| RuntimeError::JournalPoisoned)?
            .push((gate, opened_at_ms, requester, seq.get()));
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

fn bound_gate_for_state(
    shard: &Shard,
    run: RunId,
) -> Result<BoundManualGate, Box<dyn std::error::Error>> {
    let state = shard.run_state_get(run).ok_or("run state missing")?;
    let admission = state.admission.as_ref().ok_or("admission missing")?;
    let input_digest = match admission.input_digest() {
        vb_storage::RunInputBinding::Digest(digest) => digest,
        vb_storage::RunInputBinding::LegacyUnbound => {
            return Err("submitted run has no input digest".into());
        }
    };
    let policy_digest = match admission.policy_pin() {
        vb_storage::RunPolicyPin::Pinned(digest) => digest,
        vb_storage::RunPolicyPin::Unverified => {
            return Err("bound test run needs pinned policy".into());
        }
    };
    let mut gate = bound_gate(run);
    gate.binding = GateBinding {
        artifact_digest: GateDigest::from_bytes(admission.artifact_digest().as_bytes()),
        policy_digest: GateDigest::from_bytes(policy_digest.as_bytes()),
        input_digest: GateDigest::from_bytes(input_digest.as_bytes()),
        attempt: state.run_attempt.get(),
    };
    Ok(gate)
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

fn running_shard(run: RunId) -> Result<(Shard, Arc<BoundJournal>), Box<dyn std::error::Error>> {
    running_shard_with_config(run, small_config())
}

fn running_shard_with_config(
    run: RunId,
    config: crate::shard::ShardConfig,
) -> Result<(Shard, Arc<BoundJournal>), Box<dyn std::error::Error>> {
    let journal = BoundJournal::shared();
    let mut shard = Shard::new_with_journal(config, journal.clone());
    let workflow = require_workflow("chain", chain_workflow())?;
    let policy_digest = vb_storage::admission::compute_policy_digest(&workflow)?;
    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        caps: CapabilitySet::empty(),
        action_contracts: Box::from([]),
        inputs: Box::from([]),
    })?;
    shard.tick()?;
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    let state = shard.run_state_get_mut(run).ok_or("run state missing")?;
    let admission = state.admission.take().ok_or("run admission missing")?;
    state.admission =
        Some(admission.with_policy_pin(vb_storage::RunPolicyPin::Pinned(policy_digest)));
    Ok((shard, journal))
}

fn make_shard(run: RunId) -> Result<(Shard, Arc<BoundJournal>), Box<dyn std::error::Error>> {
    let (mut shard, journal) = running_shard(run)?;
    shard.enqueue(ShardCommand::OpenBoundGate {
        gate: bound_gate_for_state(&shard, run)?,
        opened_at_ms: 0,
        requester: person("bob"),
    })?;
    shard.tick()?;
    Ok((shard, journal))
}

mod cases {
    use super::*;
    include!("gate_bound_submission_cases.rs");
}

mod r5_authority {
    use super::*;
    include!("gate_bound_authority_r5.rs");
}

mod r5_registry {
    use super::*;
    include!("gate_bound_registry_r5.rs");
    include!("gate_bound_capacity.rs");
}
