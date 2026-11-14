#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;

use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::manual_gate::{
    ApproverId, ApproverRole, BoundManualGate, GateBinding, GateDecision, GateDecisionInput,
    GateDigest, GateId, ManualGate, OperatorTrust, RiskClass, SeparationOfDuties,
};
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};
use vb_ipc::client::IpcClient;
use vb_ipc::{IpcCommand, IpcPayload, MaxPayloadBytes};
use vb_runtime::journal::{RuntimeJournal, StorageRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{FjallJournal, JournalEvent, RecordKind};

fn workflow() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let mut nodes = Vec::new();
    for step in 0..9_u16 {
        nodes.push(vb_core::workflow::node(
            StepIdx::new(step),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(step + 1)),
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
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: Box::from("bound_gate_ipc"),
        digest: WorkflowDigest::from_bytes([0; 32]),
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
    let canonical = postcard::to_allocvec(&parts)?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&canonical).into());
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

fn gate(
    run: RunId,
    artifact_digest: WorkflowDigest,
    policy_digest: WorkflowDigest,
    input_digest: WorkflowDigest,
) -> Result<BoundManualGate, Box<dyn std::error::Error>> {
    let digest = |byte| GateDigest::from_bytes([byte; 32]);
    Ok(BoundManualGate {
        gate: ManualGate {
            id: GateId::new(7),
            run,
            action: ActionId::new(9),
            risk: RiskClass::High,
            compensatable: false,
            required_role: ApproverRole::new("deploy-approver".to_string())
                .ok_or("invalid approver role")?,
            separation_of_duties: SeparationOfDuties::DistinctFromRequester,
            evidence_bundle_digest: digest(1),
            prompt_digest: digest(2),
            timeout_ms: 1_000,
            escalation_policy_digest: digest(3),
            break_glass_allowed: true,
            approval_reason_required: true,
        },
        binding: GateBinding {
            artifact_digest: GateDigest::from_bytes(artifact_digest.as_bytes()),
            policy_digest: GateDigest::from_bytes(policy_digest.as_bytes()),
            input_digest: GateDigest::from_bytes(input_digest.as_bytes()),
            attempt: 1,
        },
    })
}

#[test]
fn unix_socket_command_18_records_local_claim_in_reopened_kind_55()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(78101);
    let storage_dir = tempfile::tempdir()?;
    let socket_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.evidence/tmp");
    std::fs::create_dir_all(&socket_directory)?;
    let socket_path = socket_directory
        .canonicalize()?
        .join(format!("vb-ifn5b-{}.sock", std::process::id()));
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }
    let storage = Arc::new(FjallJournal::open(storage_dir.path(), None)?);
    let runtime_journal = StorageRuntimeJournal::shared_strict(Arc::clone(&storage));
    let mut config = ShardConfig::default();
    config.policy = vb_core::policy::RuntimePolicy::Strict;
    config.step_budget_per_tick = 4;
    let shared: Arc<dyn RuntimeJournal> = runtime_journal.clone();
    let workflow = workflow()?;
    let policy_digest = vb_storage::admission::compute_policy_digest(&workflow)?;
    let accepted = vb_storage::admission::submit_artifact(
        &storage,
        &workflow,
        vb_core::policy::RuntimePolicy::Strict,
    )?;
    let artifact_digest = accepted.digest;
    let mut runtime = Runtime::new(NonZeroUsize::MIN, config, shared);
    let input_digest = vb_storage::inputs::compute_input_digest(&[])?;
    runtime.submit_direct(run, workflow)?;
    runtime.tick_all()?;
    let bound_gate = gate(run, artifact_digest, policy_digest, input_digest)?;
    runtime.open_bound_gate(
        run,
        bound_gate.clone(),
        0,
        ApproverId::new("bob".to_string()).ok_or("invalid requester")?,
    )?;
    runtime.tick_all()?;

    let mut server = vb_ipc::server::IpcServer::bind(&socket_path)?;
    let mut client = IpcClient::connect(&socket_path)?;
    server.poll_once(&mut runtime, Some(std::time::Duration::from_millis(100)))?;
    let payload = IpcPayload::SubmitBoundGateDecision {
        run_id: run,
        input: GateDecisionInput {
            gate: GateId::new(7),
            decision: GateDecision::Approve,
            approver: ApproverId::new("alice".to_string()).ok_or("invalid approver")?,
            approver_role: ApproverRole::new("deploy-approver".to_string())
                .ok_or("invalid approver role")?,
            requester: ApproverId::new("bob".to_string()).ok_or("invalid requester")?,
            reason: Some("reviewed evidence".to_string()),
            evidence_shown: true,
            evidence_bundle_digest: GateDigest::from_bytes([1; 32]),
            delegated_to: None,
            break_glass_justification: None,
            decided_at_ms: 500,
        },
    };
    client.send_command(IpcCommand::SubmitBoundGateDecision, 1818, &payload)?;
    server.poll_once(&mut runtime, Some(std::time::Duration::from_millis(100)))?;
    server.poll_once(&mut runtime, Some(std::time::Duration::from_millis(100)))?;
    let (header, response) = client.recv_response(MaxPayloadBytes::DEFAULT)?;
    assert_eq!(header.command, IpcCommand::SubmitBoundGateDecision);
    assert_eq!(header.correlation, 1818);
    assert_eq!(
        response,
        vb_ipc::server::IpcResponse::AcceptedRun { run_id: run.get() }
    );
    runtime.tick_all()?;

    let events = storage.events_for_run(run)?;
    let bound_kinds = events
        .iter()
        .map(JournalEvent::record_kind)
        .filter(|kind| {
            matches!(
                kind,
                RecordKind::GateOpenedBound | RecordKind::GateDecisionBound
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        bound_kinds,
        [RecordKind::GateOpenedBound, RecordKind::GateDecisionBound]
    );
    let opening_index = events
        .iter()
        .position(|event| matches!(event, JournalEvent::GateOpenedBoundEvent { .. }))
        .ok_or("durable kind-54 opening missing")?;
    let decision = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::GateDecisionBoundEvent { record, .. } => Some(record),
            _ => None,
        })
        .ok_or("durable kind-55 decision missing")?;
    let decision_index = events
        .iter()
        .position(|event| matches!(event, JournalEvent::GateDecisionBoundEvent { .. }))
        .ok_or("durable kind-55 index missing")?;
    assert_eq!(
        decision.operator_trust,
        Some(OperatorTrust::LocalSocketClaim)
    );
    let receipt = decision
        .bound_receipt
        .as_ref()
        .ok_or("approval receipt missing")?;
    assert!(vb_runtime::manual_gate::verify_bound_receipt(
        receipt,
        &bound_gate
    )?);
    let resumed_index = events
        .iter()
        .position(|event| matches!(event, JournalEvent::RunResumed { .. }))
        .ok_or("valid approval did not resume the run")?;
    assert!(opening_index < decision_index && decision_index < resumed_index);
    drop(client);
    drop(server);
    drop(runtime);
    drop(runtime_journal);
    drop(storage);

    let reopened = FjallJournal::open(storage_dir.path(), None)?;
    let persisted = reopened.events_for_run(run)?;
    assert_eq!(persisted, events);
    Ok(())
}
