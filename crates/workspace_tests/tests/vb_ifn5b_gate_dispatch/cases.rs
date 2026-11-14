use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::action_pack::{CapabilityScopeSpec, RecoveryClass};
use vb_core::capability::Capability;
use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, BoundApprovalReceipt, BoundManualGate, GateBinding, GateDecision,
    GateDecisionInput, GateDigest, GateId, ManualGate, RiskClass, SeparationOfDuties,
};
use vb_core::policy::{DecisionCode, PolicyDigest, PolicyRule, RuntimePolicy};
use vb_core::value::SlotValue;
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, WorkflowParts,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::{RegisteredExecutionError, Runtime};
use vb_storage::executor::{ExecutorCatalog, ExecutorError, FjallRegister};
use vb_storage::{FjallConfig, FjallJournal, JournalEvent};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn person(value: &str) -> Result<ApproverId, Box<dyn std::error::Error>> {
    ApproverId::new(value.into()).ok_or_else(|| "invalid test approver".into())
}

fn role(value: &str) -> Result<ApproverRole, Box<dyn std::error::Error>> {
    ApproverRole::new(value.into()).ok_or_else(|| "invalid test role".into())
}

fn workflow() -> Result<CompiledWorkflow, String> {
    let mut nodes = Vec::new();
    for index in 0..9_u16 {
        nodes.push(vb_core::workflow::node(
            StepIdx::new(index),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(index + 1)),
        ));
    }
    nodes.push(vb_core::workflow::node(
        StepIdx::new(9),
        CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(1),
        },
        Some(SlotIdx::new(2)),
        Some(StepIdx::new(10)),
    ));
    nodes.push(vb_core::workflow::node(
        StepIdx::new(10),
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(1),
        },
        Some(SlotIdx::new(3)),
        Some(StepIdx::new(11)),
    ));
    nodes.push(vb_core::workflow::node(
        StepIdx::new(11),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(3),
        },
        None,
        None,
    ));
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::from([CompiledInputSlot {
            slot: SlotIdx::ZERO,
            kind: InputSlotKind::Any,
            constraint: CompiledInputConstraint::required(),
        }]),
        name: Box::from("vb_ifn5b_gate_dispatch"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::ConstValue::I64(42)]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    let bytes = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&bytes).into());
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

fn contract() -> Result<ActionContract, Box<dyn std::error::Error>> {
    Ok(ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("vb_ifn5b_gate_dispatch")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1_000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::Writes,
        retry_safety: RetrySafety::KeyRequired,
        required_capabilities: Box::from([Capability::new(
            "velvet.register.write".into(),
            ActionId::new(1),
        )]),
    })
}

fn preceding_contract() -> Result<ActionContract, Box<dyn std::error::Error>> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("vb_ifn5b_preceding_contract")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

fn scope() -> CapabilityScopeSpec {
    CapabilityScopeSpec {
        namespace: 23,
        key: 41,
    }
}

fn storage_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.evidence/r6")
        .join(format!("gate-dispatch-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

fn approval_for(
    runtime: &mut Runtime,
    journal: &FjallJournal,
    run: RunId,
    artifact: &vb_storage::AcceptedArtifact,
    gate_id: GateId,
    decision: GateDecision,
) -> Result<BoundApprovalReceipt, Box<dyn std::error::Error>> {
    let _pending = gate_ticket_binding::pending_ticket_from_events(runtime, journal, run)?;
    let input = journal
        .events_for_run_full(run)?
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunAdmission {
                input_digest: vb_storage::RunInputBinding::Digest(digest),
                ..
            }
            | JournalEvent::RunAdmissionBudget {
                input_digest: vb_storage::RunInputBinding::Digest(digest),
                ..
            } => Some(*digest),
            _ => None,
        })
        .ok_or("durable admitted input digest missing")?;
    let gate = BoundManualGate {
        gate: ManualGate {
            id: gate_id,
            run,
            action: ActionId::new(1),
            risk: RiskClass::High,
            compensatable: false,
            required_role: role("deploy-approver")?,
            separation_of_duties: SeparationOfDuties::DistinctFromRequester,
            evidence_bundle_digest: GateDigest::from_bytes([7; 32]),
            prompt_digest: GateDigest::from_bytes([8; 32]),
            timeout_ms: 5_000,
            escalation_policy_digest: GateDigest::from_bytes([9; 32]),
            break_glass_allowed: true,
            approval_reason_required: true,
        },
        binding: GateBinding {
            artifact_digest: GateDigest::from_bytes(artifact.digest.as_bytes()),
            policy_digest: GateDigest::from_bytes(artifact.policy_digest.as_bytes()),
            input_digest: GateDigest::from_bytes(input.as_bytes()),
            attempt: 1,
        },
    };
    let requester = person("requester")?;
    runtime.open_bound_gate(run, gate, 100, requester.clone())?;
    runtime.tick_all()?;
    runtime.submit_bound_gate_decision(
        run,
        GateDecisionInput {
            gate: gate_id,
            decision,
            approver: person("approver")?,
            approver_role: role("deploy-approver")?,
            requester,
            reason: Some(String::from("reviewed deployment evidence")),
            evidence_shown: true,
            evidence_bundle_digest: GateDigest::from_bytes([7; 32]),
            delegated_to: None,
            break_glass_justification: (decision == GateDecision::BreakGlass)
                .then(|| String::from("production incident")),
            decided_at_ms: 200,
        },
    )?;
    runtime.tick_all()?;
    runtime.tick_all()?;
    journal
        .events_for_run_full(run)?
        .iter()
        .find_map(|event| match event {
            JournalEvent::GateDecisionBoundEvent { record, .. }
                if record.record.gate == gate_id =>
            {
                record.bound_receipt.clone()
            }
            _ => None,
        })
        .ok_or_else(|| "durable approval receipt missing".into())
}
fn pending_ticket(
    runtime: &mut Runtime,
    run: RunId,
) -> Result<vb_core::ActionTicket, Box<dyn std::error::Error>> {
    for _ in 0..16 {
        if let Some(ticket) = runtime.lookup_pending_action_ticket(run, 9) {
            return Ok(ticket);
        }
        runtime.tick_all()?;
    }
    Err("registered action did not become pending".into())
}

struct DispatchSnapshot {
    run: RunId,
    ticket: vb_core::ActionTicket,
    events: Vec<JournalEvent>,
}

fn assert_no_effect(
    adapter: &FjallRegister,
    runtime: &mut Runtime,
    journal: &FjallJournal,
    before: &DispatchSnapshot,
) -> TestResult {
    assert_eq!(adapter.effect_count()?, 0);
    assert_eq!(adapter.value(scope())?, None);
    assert_eq!(journal.events_for_run_full(before.run)?, before.events);
    assert_eq!(
        runtime.lookup_pending_action_ticket(before.run, u64::from(before.ticket.step.get())),
        Some(before.ticket),
    );
    Ok(())
}

#[path = "cases/assertions.rs"]
mod assertions;
pub(super) use assertions::assert_nonapproval_refusals;
#[path = "cases/gate_bound.rs"]
mod gate_bound;
#[path = "cases/gate_break_glass.rs"]
mod gate_break_glass;
#[path = "cases/gate_cold_missing_receipt.rs"]
mod gate_cold_missing_receipt;
#[path = "cases/gate_cold_recovery.rs"]
mod gate_cold_recovery;
#[path = "cases/gate_nonapproval_consumer.rs"]
mod gate_nonapproval_consumer;
#[path = "cases/gate_opening_metadata.rs"]
mod gate_opening_metadata;
#[path = "cases/gate_policy_tamper.rs"]
mod gate_policy_tamper;
#[path = "cases/gate_reconcile.rs"]
mod gate_reconcile;
#[path = "cases/gate_retry_ticket.rs"]
mod gate_retry_ticket;
#[path = "cases/gate_ticket_binding.rs"]
mod gate_ticket_binding;
#[path = "cases/gate_ticket_boundaries.rs"]
mod gate_ticket_boundaries;
#[path = "cases/owned_submission.rs"]
mod owned_submission;
