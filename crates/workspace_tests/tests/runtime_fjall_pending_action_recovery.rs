#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;

use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutputReady, ActionTicket,
    Idempotency, RetrySafety, SideEffect,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ListId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::RuntimeError;
use vb_runtime::engine::compute_idempotency_key;
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::admission::submit_artifact_with_contracts;
use vb_storage::{EventSeq, FjallConfig, FjallJournal, JournalEvent, encode_slot_written_extra};

fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("invalid shard count {value}"))
}

fn strict_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        policy: RuntimePolicy::Strict,
    }
}

fn node(id: u16, output: Option<u16>, next: Option<u16>, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: output.map(SlotIdx::new),
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind,
    }
}

fn action_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    workflow_from_parts(
        "fjall_pending_action_recovery",
        Box::from([
            node(
                0,
                Some(1),
                Some(1),
                CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::ZERO,
                },
            ),
            node(
                1,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            ),
        ]),
        Box::from([]),
        2,
    )
}

fn wait_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    workflow_from_parts(
        "fjall_wait_timer_recovery",
        Box::from([
            node(
                0,
                None,
                Some(1),
                CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::ZERO,
                },
            ),
            node(
                1,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
            ),
        ]),
        Box::from([]),
        1,
    )
}

fn ask_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    workflow_from_parts(
        "fjall_ask_timer_recovery",
        Box::from([
            node(
                0,
                None,
                Some(1),
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::ZERO,
                    timeout_slot: None,
                },
            ),
            node(
                1,
                None,
                Some(2),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(1),
                },
            ),
            node(
                2,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            ),
        ]),
        Box::from([]),
        2,
    )
}

fn timed_ask_then_finish_workflow() -> Result<CompiledWorkflow, String> {
    workflow_from_parts(
        "fjall_timed_ask_recovery",
        Box::from([
            node(
                0,
                None,
                Some(1),
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::ZERO,
                    timeout_slot: Some(SlotIdx::new(1)),
                },
            ),
            node(
                1,
                None,
                Some(2),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(2),
                },
            ),
            node(
                2,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
            ),
        ]),
        Box::from([]),
        3,
    )
}

fn workflow_from_parts(
    name: &str,
    nodes: Box<[CompiledNode]>,
    constants: Box<[ConstValue]>,
    slot_count: u16,
) -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        name: Box::from(name),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes,
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants,
        slot_count,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    let hash_bytes = postcard::to_allocvec(&parts).map_err(|err| err.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&hash_bytes).into());
    CompiledWorkflow::try_from_parts(parts).map_err(|err| err.to_string())
}

fn required_capability(action: ActionId) -> Capability {
    Capability::new(Box::from("fjall.pending-action.recovery"), action)
}

fn action_contract(action: ActionId) -> Result<ActionContract, String> {
    Ok(ActionContract {
        id: action,
        name: ActionName::new("fjall-recovery-action").map_err(|err| err.to_string())?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([required_capability(action)]),
    })
}

fn divergent_action_contract(action: ActionId) -> Result<ActionContract, String> {
    let mut contract = action_contract(action)?;
    contract.max_output_bytes = contract
        .max_output_bytes
        .checked_add(1)
        .ok_or_else(|| String::from("action contract max_output_bytes overflow"))?;
    Ok(contract)
}

fn action_contract_abi_digest(contract: &ActionContract) -> Result<WorkflowDigest, String> {
    let bytes = postcard::to_allocvec(contract).map_err(|err| err.to_string())?;
    Ok(WorkflowDigest::from_bytes(blake3::hash(&bytes).into()))
}

fn accepted_artifact_action_abi_digest(
    artifact: &vb_storage::admission::AcceptedArtifact,
    action: ActionId,
) -> Result<WorkflowDigest, String> {
    let contract = artifact
        .action_contracts
        .iter()
        .find(|contract| contract.id == action)
        .ok_or_else(|| format!("accepted artifact missing contract for action {action:?}"))?;
    action_contract_abi_digest(contract)
}

fn zero_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0u8; 32])
}

fn action_grants(action: ActionId) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([required_capability(action)]))
}

fn open_journal(path: &Path) -> Result<Arc<FjallJournal>, String> {
    FjallJournal::open(path, Some(FjallConfig::default()))
        .map(Arc::new)
        .map_err(|err| err.to_string())
}

fn pending_ticket(run: RunId, action: ActionId) -> ActionTicket {
    ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action,
        attempt: 1,
        idempotency_key: compute_idempotency_key(run, SeqNo::ZERO, action),
        capacity: 1,
    }
}

fn ready_output(slot: SlotIdx, value: SlotValue) -> Result<ActionOutputReady, String> {
    let encoded = encoded_slot_value(value)?;
    let encoded_len = u32::try_from(encoded.len()).map_err(|err| err.to_string())?;
    Ok(ActionOutputReady {
        output_slot: slot,
        value,
        taint: Taint::Clean,
        encoded_len,
    })
}

fn encoded_slot_value(value: SlotValue) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(&value).map_err(|err| err.to_string())
}

fn write_pending_action_events(
    journal: &FjallJournal,
    run: RunId,
    workflow: &CompiledWorkflow,
    artifact_digest: WorkflowDigest,
    ticket: ActionTicket,
    action_abi_digest: WorkflowDigest,
) -> Result<(), String> {
    write_pending_action_events_custom(
        journal,
        run,
        workflow.digest(),
        artifact_digest,
        action_grants(ticket.action),
        RuntimePolicy::Strict,
        ticket,
        action_abi_digest,
    )
}

fn write_pending_action_events_custom(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    artifact_digest: WorkflowDigest,
    granted_capabilities: CapabilitySet,
    policy: RuntimePolicy,
    ticket: ActionTicket,
    action_abi_digest: WorkflowDigest,
) -> Result<(), String> {
    write_pending_action_events_with_input(
        journal,
        run,
        workflow_digest,
        artifact_digest,
        granted_capabilities,
        policy,
        ticket,
        action_abi_digest,
        SlotValue::I64(0),
    )
}

fn write_pending_action_events_with_input(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    artifact_digest: WorkflowDigest,
    granted_capabilities: CapabilitySet,
    policy: RuntimePolicy,
    ticket: ActionTicket,
    action_abi_digest: WorkflowDigest,
    input_value: SlotValue,
) -> Result<(), String> {
    let input_bytes = encoded_slot_value(input_value)?;
    let input_extra =
        encode_slot_written_extra(Taint::Clean, None).map_err(|err| format!("{err:?}"))?;
    let events = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: workflow_digest,
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest,
            granted_capabilities,
            policy,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::ZERO,
            value: Some(input_bytes),
            extra: Some(input_extra),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::ActionScheduledTicket {
            run,
            seq: EventSeq::new(4),
            ticket,
            input: SlotIdx::ZERO,
            output: SlotIdx::new(1),
            action_abi_digest,
        },
    ];
    for event in events {
        journal
            .append_strict(&event)
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn stored_action_schedule(
    events: &[JournalEvent],
    action: ActionId,
) -> Result<(ActionTicket, WorkflowDigest), String> {
    for event in events {
        if let JournalEvent::ActionScheduledTicket {
            ticket,
            action_abi_digest,
            ..
        } = event
            && ticket.action == action
        {
            return Ok((*ticket, *action_abi_digest));
        }
    }
    Err(format!(
        "expected stored ActionScheduledTicket for action {action:?}"
    ))
}

fn has_stored_action_schedule(events: &[JournalEvent]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. }))
}

fn write_wait_boundary_events(
    journal: &FjallJournal,
    run: RunId,
    workflow: &CompiledWorkflow,
    artifact_digest: WorkflowDigest,
) -> Result<(), String> {
    let events = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: workflow.digest(),
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest,
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            attempt: 1,
        },
    ];
    append_events(journal, events)
}

fn write_ask_boundary_events(
    journal: &FjallJournal,
    run: RunId,
    workflow: &CompiledWorkflow,
    artifact_digest: WorkflowDigest,
) -> Result<(), String> {
    let events = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: workflow.digest(),
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest,
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            attempt: 1,
        },
    ];
    append_events(journal, events)
}

fn write_ask_boundary_events_with_prompt(
    journal: &FjallJournal,
    run: RunId,
    workflow: &CompiledWorkflow,
    artifact_digest: WorkflowDigest,
    prompt: SlotValue,
) -> Result<(), String> {
    let prompt_bytes = encoded_slot_value(prompt)?;
    let prompt_extra =
        encode_slot_written_extra(Taint::Clean, None).map_err(|err| format!("{err:?}"))?;
    let events = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: workflow.digest(),
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest,
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::ZERO,
            value: Some(prompt_bytes),
            extra: Some(prompt_extra),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::ZERO,
            attempt: 1,
        },
    ];
    append_events(journal, events)
}

fn append_events<const N: usize>(
    journal: &FjallJournal,
    events: [JournalEvent; N],
) -> Result<(), String> {
    for event in events {
        journal
            .append_strict(&event)
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn assert_cannot_resume(
    result: Result<(), RuntimeError>,
    expected_reason: &'static str,
) -> Result<(), String> {
    match result {
        Err(RuntimeError::RecoveryCannotResume { reason }) if reason == expected_reason => Ok(()),
        Err(other) => Err(format!(
            "expected RecoveryCannotResume({expected_reason}), got {other:?}"
        )),
        Ok(()) => Err(format!(
            "expected RecoveryCannotResume({expected_reason}), got Ok(())"
        )),
    }
}

fn assert_tick_cannot_resume(
    result: Result<bool, RuntimeError>,
    expected_reason: &'static str,
) -> Result<(), String> {
    match result {
        Err(RuntimeError::RecoveryCannotResume { reason }) if reason == expected_reason => Ok(()),
        Err(other) => Err(format!(
            "expected RecoveryCannotResume({expected_reason}), got {other:?}"
        )),
        Ok(value) => Err(format!(
            "expected RecoveryCannotResume({expected_reason}), got Ok({value})"
        )),
    }
}

fn assert_product_cannot_resume(
    product: RuntimeRecoveryProduct,
    expected_reason: &'static str,
) -> Result<(), String> {
    match product {
        RuntimeRecoveryProduct::CannotResume(product) if product.reason() == expected_reason => {
            Ok(())
        }
        other => Err(format!(
            "expected typed CannotResume({expected_reason}), got {other:?}"
        )),
    }
}

#[derive(Clone, Copy)]
enum DigestChoice {
    AcceptedArtifact,
    WorkflowSource,
    Exact(WorkflowDigest),
}

fn choose_digest(
    choice: DigestChoice,
    artifact: &vb_storage::admission::AcceptedArtifact,
    workflow: &CompiledWorkflow,
) -> WorkflowDigest {
    match choice {
        DigestChoice::AcceptedArtifact => artifact.digest,
        DigestChoice::WorkflowSource => workflow.digest(),
        DigestChoice::Exact(digest) => digest,
    }
}

fn choose_grants(grants: Option<CapabilitySet>, action: ActionId) -> CapabilitySet {
    match grants {
        Some(value) => value,
        None => action_grants(action),
    }
}

fn choose_action_abi_digest(
    digest: Option<WorkflowDigest>,
    accepted_digest: WorkflowDigest,
) -> WorkflowDigest {
    match digest {
        Some(value) => value,
        None => accepted_digest,
    }
}

fn assert_pending_recovery_rejects(
    run: RunId,
    workflow_digest: DigestChoice,
    artifact_digest: DigestChoice,
    granted_capabilities: Option<CapabilitySet>,
    policy: RuntimePolicy,
    action_abi_digest: Option<WorkflowDigest>,
    expected_reason: &'static str,
) -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contracts = Box::from([action_contract(action)?]);
    let ticket = pending_ticket(run, action);

    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    let accepted_abi_digest = accepted_artifact_action_abi_digest(&artifact, action)?;
    write_pending_action_events_custom(
        &journal,
        run,
        choose_digest(workflow_digest, &artifact, &workflow),
        choose_digest(artifact_digest, &artifact, &workflow),
        choose_grants(granted_capabilities, action),
        policy,
        ticket,
        choose_action_abi_digest(action_abi_digest, accepted_abi_digest),
    )?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict(journal);
    let recovered = Runtime::new(shard_count(1)?, strict_config(), shared);
    assert_cannot_resume(recovered.recover_and_resume(run), expected_reason)
}

#[test]
fn runtime_submit_drive_persists_recoverable_fjall_pending_action_abi_digest() -> Result<(), String>
{
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_700);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contracts = Box::from([action_contract(action)?]);

    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    let accepted_digest = accepted_artifact_action_abi_digest(&artifact, action)?;
    assert_ne!(accepted_digest, zero_digest());

    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), shared);
    runtime
        .submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow,
            Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
            action_grants(action),
            artifact.action_contracts.clone(),
        )
        .map_err(|err| err.to_string())?;
    runtime.tick_all().map_err(|err| err.to_string())?;

    let events = journal
        .events_for_run_full(run)
        .map_err(|err| err.to_string())?;
    let (stored_ticket, stored_digest) = stored_action_schedule(&events, action)?;
    assert_eq!(stored_digest, accepted_digest);
    assert_ne!(stored_digest, zero_digest());
    drop(runtime);
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict(journal);
    let mut recovered = Runtime::new(shard_count(1)?, strict_config(), shared);
    recovered
        .recover_and_resume(run)
        .map_err(|err| err.to_string())?;
    recovered.tick_all().map_err(|err| err.to_string())?;
    let recovered_ticket = recovered
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or_else(|| String::from("expected recovered pending action ticket"))?;
    assert_eq!(recovered_ticket, stored_ticket);
    Ok(())
}

#[test]
fn runtime_submit_drive_without_action_abi_authority_fails_before_ticket_persist()
-> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_699);
    let workflow = action_then_finish_workflow()?;
    let contracts: Box<[ActionContract]> = Box::from([]);

    let journal = open_journal(temp.path())?;
    submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
        .map_err(|err| err.to_string())?;
    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), shared);
    runtime
        .submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow,
            Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
            CapabilitySet::empty(),
            contracts,
        )
        .map_err(|err| err.to_string())?;

    assert_tick_cannot_resume(runtime.tick_all(), "action_abi_digests_missing")?;
    let events = journal
        .events_for_run_full(run)
        .map_err(|err| err.to_string())?;
    assert!(!has_stored_action_schedule(&events));
    Ok(())
}

#[test]
fn strict_submit_rejects_unbound_action_abi_before_ticket_persist() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_698);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let accepted_contracts: Box<[ActionContract]> = Box::from([action_contract(action)?]);
    let submitted_contracts: Box<[ActionContract]> =
        Box::from([divergent_action_contract(action)?]);

    let journal = open_journal(temp.path())?;
    let artifact = submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &accepted_contracts,
    )
    .map_err(|err| err.to_string())?;
    let expected = accepted_artifact_action_abi_digest(&artifact, action)?;
    let submitted = action_contract_abi_digest(
        submitted_contracts
            .first()
            .ok_or_else(|| String::from("missing submitted action contract"))?,
    )?;
    assert_ne!(expected, submitted);

    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), shared);
    let result = runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
        action_grants(action),
        submitted_contracts,
    );
    match result {
        Err(RuntimeError::AdmissionActionAbiDigestMismatch {
            action: rejected_action,
            expected: rejected_expected,
            submitted: rejected_submitted,
        }) => {
            assert_eq!(rejected_action, action);
            assert_eq!(rejected_expected, expected);
            assert_eq!(rejected_submitted, submitted);
        }
        other => {
            return Err(format!(
                "expected AdmissionActionAbiDigestMismatch, got {other:?}"
            ));
        }
    }
    runtime.tick_all().map_err(|err| err.to_string())?;
    let events = journal
        .events_for_run_full(run)
        .map_err(|err| err.to_string())?;
    assert!(!has_stored_action_schedule(&events));
    Ok(())
}

#[test]
fn recover_and_resume_rejects_source_digest_as_artifact_digest() -> Result<(), String> {
    assert_pending_recovery_rejects(
        RunId::new(7_710),
        DigestChoice::WorkflowSource,
        DigestChoice::WorkflowSource,
        None,
        RuntimePolicy::Strict,
        None,
        "artifact_digest_mismatch",
    )
}

#[test]
fn recover_and_resume_rejects_workflow_source_digest_mismatch() -> Result<(), String> {
    assert_pending_recovery_rejects(
        RunId::new(7_711),
        DigestChoice::Exact(WorkflowDigest::from_bytes([0x71; 32])),
        DigestChoice::AcceptedArtifact,
        None,
        RuntimePolicy::Strict,
        None,
        "workflow_digest_mismatch",
    )
}

#[test]
fn recover_and_resume_rejects_admission_policy_mismatch() -> Result<(), String> {
    assert_pending_recovery_rejects(
        RunId::new(7_712),
        DigestChoice::WorkflowSource,
        DigestChoice::AcceptedArtifact,
        None,
        RuntimePolicy::Journaled,
        None,
        "admission_policy_mismatch",
    )
}

#[test]
fn recover_and_resume_rejects_capability_mismatch() -> Result<(), String> {
    assert_pending_recovery_rejects(
        RunId::new(7_713),
        DigestChoice::WorkflowSource,
        DigestChoice::AcceptedArtifact,
        Some(CapabilitySet::empty()),
        RuntimePolicy::Strict,
        None,
        "admission_capabilities_mismatch",
    )
}

#[test]
fn recover_and_resume_rejects_action_abi_digest_mismatch() -> Result<(), String> {
    assert_pending_recovery_rejects(
        RunId::new(7_714),
        DigestChoice::WorkflowSource,
        DigestChoice::AcceptedArtifact,
        None,
        RuntimePolicy::Strict,
        Some(WorkflowDigest::from_bytes([0x72; 32])),
        "action_abi_digest_mismatch",
    )
}

#[test]
fn recover_and_resume_rehydrates_fjall_pending_action_ticket_after_reopen() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_701);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contracts = Box::from([action_contract(action)?]);

    let expected_ticket = pending_ticket(run, action);
    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    let contract = contracts
        .first()
        .ok_or_else(|| String::from("missing action contract"))?;
    let action_abi_digest = action_contract_abi_digest(contract)?;
    write_pending_action_events(
        &journal,
        run,
        &workflow,
        artifact.digest,
        expected_ticket,
        action_abi_digest,
    )?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict(journal);
    let mut recovered = Runtime::new(shard_count(1)?, strict_config(), shared);
    recovered
        .recover_and_resume(run)
        .map_err(|err| err.to_string())?;
    recovered.tick_all().map_err(|err| err.to_string())?;

    let ticket = recovered
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or_else(|| String::from("expected recovered pending action ticket"))?;
    assert_eq!(
        ticket.idempotency_key,
        compute_idempotency_key(ticket.run, ticket.seq, ticket.action)
    );
    assert_eq!(ticket.run, run);
    assert_eq!(ticket.step, StepIdx::ZERO);
    assert_eq!(ticket.action, action);
    assert_eq!(ticket.idempotency_key, expected_ticket.idempotency_key);

    recovered
        .fail_action(ticket, ActionFailure::from(ActionFailureCode::InvalidInput))
        .map_err(|err| err.to_string())?;
    recovered.tick_all().map_err(|err| err.to_string())?;

    assert_eq!(recovered.counters_snapshot().runs_failed, 1);
    assert_eq!(recovered.list_active_runs(8, None), Vec::new());
    Ok(())
}

#[test]
fn recovered_fjall_pending_action_can_complete_after_reopen() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_702);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contracts = Box::from([action_contract(action)?]);

    let expected_ticket = pending_ticket(run, action);
    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    let contract = contracts
        .first()
        .ok_or_else(|| String::from("missing action contract"))?;
    let action_abi_digest = action_contract_abi_digest(contract)?;
    write_pending_action_events(
        &journal,
        run,
        &workflow,
        artifact.digest,
        expected_ticket,
        action_abi_digest,
    )?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict(journal);
    let mut recovered = Runtime::new(shard_count(1)?, strict_config(), shared);
    recovered
        .recover_and_resume(run)
        .map_err(|err| err.to_string())?;
    recovered.tick_all().map_err(|err| err.to_string())?;

    let ticket = recovered
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or_else(|| String::from("expected recovered pending action ticket"))?;
    let output = ready_output(SlotIdx::new(1), SlotValue::I64(42))?;
    assert_eq!(
        recovered.complete_action_with_output(ticket, output),
        Ok(())
    );
    recovered.tick_all().map_err(|err| err.to_string())?;

    assert_eq!(recovered.counters_snapshot().runs_completed, 1);
    assert_eq!(recovered.list_active_runs(8, None), Vec::new());
    Ok(())
}

#[test]
fn recover_and_resume_fails_closed_for_store_dependent_fjall_pending_action_after_reopen()
-> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_706);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contracts = Box::from([action_contract(action)?]);

    let expected_ticket = pending_ticket(run, action);
    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    let contract = contracts
        .first()
        .ok_or_else(|| String::from("missing action contract"))?;
    let action_abi_digest = action_contract_abi_digest(contract)?;
    write_pending_action_events_with_input(
        &journal,
        run,
        workflow.digest(),
        artifact.digest,
        action_grants(action),
        RuntimePolicy::Strict,
        expected_ticket,
        action_abi_digest,
        SlotValue::List(ListId::new(0)),
    )?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict(journal);
    let recovered = Runtime::new(shard_count(1)?, strict_config(), shared);
    assert_product_cannot_resume(
        recovered
            .recover_product(run)
            .map_err(|err| err.to_string())?,
        "store_missing",
    )?;
    assert_cannot_resume(recovered.recover_and_resume(run), "store_missing")?;
    assert_eq!(recovered.list_active_runs(8, None), Vec::new());
    Ok(())
}

#[test]
fn recover_and_resume_fails_closed_for_fjall_wait_timer_after_reopen() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_703);
    let workflow = wait_then_finish_workflow()?;
    let contracts: Box<[ActionContract]> = Box::from([]);

    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    write_wait_boundary_events(&journal, run, &workflow, artifact.digest)?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict(journal);
    let recovered = Runtime::new(shard_count(1)?, strict_config(), shared);

    assert_product_cannot_resume(
        recovered
            .recover_product(run)
            .map_err(|err| err.to_string())?,
        "pending_timers",
    )?;
    assert_cannot_resume(recovered.recover_and_resume(run), "pending_timers")?;
    assert_eq!(recovered.list_active_runs(8, None), Vec::new());
    Ok(())
}

#[test]
fn recovered_fjall_open_ask_can_answer_after_reopen() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_704);
    let workflow = ask_then_finish_workflow()?;
    let contracts: Box<[ActionContract]> = Box::from([]);

    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    write_ask_boundary_events(&journal, run, &workflow, artifact.digest)?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict(journal);
    let mut recovered = Runtime::new(shard_count(1)?, strict_config(), shared);

    match recovered
        .recover_product(run)
        .map_err(|err| err.to_string())?
    {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(product.boundary_kind(), RecoveredRunBoundaryKind::OpenAsk);
        }
        other => {
            return Err(format!(
                "expected open-ask resumable product, got {other:?}"
            ));
        }
    }
    recovered
        .recover_and_resume(run)
        .map_err(|err| err.to_string())?;
    recovered.tick_all().map_err(|err| err.to_string())?;
    let answer = vb_runtime::shard::AskAnswer::with_encoded_len(
        vb_runtime::shard::AskTicket {
            run,
            ask_step: StepIdx::ZERO,
            resume_step: StepIdx::new(1),
        },
        SlotIdx::new(1),
        SlotValue::I64(99),
        Taint::Clean,
        1,
    );
    recovered
        .answer_ask(answer)
        .map_err(|err| err.to_string())?;
    recovered.tick_all().map_err(|err| err.to_string())?;

    assert_eq!(recovered.counters_snapshot().runs_completed, 1);
    assert_eq!(recovered.list_active_runs(8, None), Vec::new());
    Ok(())
}

#[test]
fn recovered_fjall_open_ask_with_scalar_prompt_can_answer_after_reopen() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_707);
    let workflow = ask_then_finish_workflow()?;
    let contracts: Box<[ActionContract]> = Box::from([]);

    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    write_ask_boundary_events_with_prompt(
        &journal,
        run,
        &workflow,
        artifact.digest,
        SlotValue::I64(11),
    )?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict(journal);
    let mut recovered = Runtime::new(shard_count(1)?, strict_config(), shared);

    match recovered
        .recover_product(run)
        .map_err(|err| err.to_string())?
    {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(product.boundary_kind(), RecoveredRunBoundaryKind::OpenAsk);
        }
        other => {
            return Err(format!(
                "expected scalar open-ask resumable product, got {other:?}"
            ));
        }
    }
    recovered
        .recover_and_resume(run)
        .map_err(|err| err.to_string())?;
    recovered.tick_all().map_err(|err| err.to_string())?;
    let answer = vb_runtime::shard::AskAnswer::with_encoded_len(
        vb_runtime::shard::AskTicket {
            run,
            ask_step: StepIdx::ZERO,
            resume_step: StepIdx::new(1),
        },
        SlotIdx::new(1),
        SlotValue::I64(101),
        Taint::Clean,
        1,
    );
    recovered
        .answer_ask(answer)
        .map_err(|err| err.to_string())?;
    recovered.tick_all().map_err(|err| err.to_string())?;

    assert_eq!(recovered.counters_snapshot().runs_completed, 1);
    assert_eq!(recovered.list_active_runs(8, None), Vec::new());
    Ok(())
}

#[test]
fn recover_and_resume_fails_closed_for_store_dependent_fjall_open_ask_after_reopen()
-> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_708);
    let workflow = ask_then_finish_workflow()?;
    let contracts: Box<[ActionContract]> = Box::from([]);

    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    write_ask_boundary_events_with_prompt(
        &journal,
        run,
        &workflow,
        artifact.digest,
        SlotValue::List(ListId::new(0)),
    )?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict(journal);
    let recovered = Runtime::new(shard_count(1)?, strict_config(), shared);
    assert_product_cannot_resume(
        recovered
            .recover_product(run)
            .map_err(|err| err.to_string())?,
        "store_missing",
    )?;
    assert_cannot_resume(recovered.recover_and_resume(run), "store_missing")?;
    assert_eq!(recovered.list_active_runs(8, None), Vec::new());
    Ok(())
}

#[test]
fn recover_and_resume_fails_closed_for_fjall_timed_ask_after_reopen() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_705);
    let workflow = timed_ask_then_finish_workflow()?;
    let contracts: Box<[ActionContract]> = Box::from([]);

    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    write_ask_boundary_events(&journal, run, &workflow, artifact.digest)?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict(journal);
    let recovered = Runtime::new(shard_count(1)?, strict_config(), shared);

    assert_product_cannot_resume(
        recovered
            .recover_product(run)
            .map_err(|err| err.to_string())?,
        "pending_asks",
    )?;
    assert_cannot_resume(recovered.recover_and_resume(run), "pending_asks")?;
    assert_eq!(recovered.list_active_runs(8, None), Vec::new());
    Ok(())
}

// ---------------------------------------------------------------------------
// FINDING-002 witness-carries-through contract test (vb-w25-runtime-a2).
//
// Asserts that the storage-typed `RecoveryFrameSeedProduct` cannot-resume
// witness survives the runtime classification step. The runtime API
// (`Runtime::recover_product` → `RuntimeRecoveryProduct::CannotResume
// { reason }`) and the storage typed entry point
// (`vb_storage::recovery::recover_runtime_frame_seed_from_events`)
// MUST agree on the cannot-resume flag pattern for the same journal
// events; if the runtime classification step erased the storage
// witness (the FINDING-002 typestate bypass) this test would fail.
//
// Exercises three cannot-resume reasons:
//   - store_missing     — value-store-required slot values, no boundary
//   - pending_timers    — unresolved WaitScheduledEvent
//   - pending_asks      — unresolved AskScheduledEvent (timed variant)
// ---------------------------------------------------------------------------

/// Runtime product plus the independent storage-recovered value-store
/// evidence for a single seeded slot value, used by the FINDING-002
/// store_missing non-tautology test.
struct StoreMissingProbe {
    product: RuntimeRecoveryProduct,
    storage_requires_value_store: bool,
}

/// Whether the storage-recovered seed for `run` carries a slot value that
/// requires the cold value store (`List`/`Object`/`Blob`) — the exact
/// durable evidence the runtime uses to decide `store_missing`. Recovered
/// independently via the storage typed entry point so the runtime decision
/// can be checked against durable storage evidence.
fn storage_seed_requires_value_store(
    journal: &Arc<FjallJournal>,
    run: RunId,
) -> Result<bool, String> {
    let events = journal
        .events_for_run_full(run)
        .map_err(|err| err.to_string())?;
    let product = vb_storage::recovery::recover_runtime_frame_seed_from_events(&events)
        .map_err(|err| format!("typed storage recovery failed: {err}"))?;
    Ok(product.seed().slots.iter().any(|entry| {
        matches!(
            entry.value,
            SlotValue::List(_) | SlotValue::Object(_) | SlotValue::Blob(_)
        )
    }))
}

/// Writes a pending-action run seeded with `input_value`, reopens the
/// journal, records the independent storage value-store evidence, and
/// returns the runtime `recover_product` classification. The setup is
/// byte-identical across calls except for `input_value`, so the only
/// variable driving the runtime `store_missing` classification is the slot
/// value type.
fn recover_store_missing_probe(
    input_value: SlotValue,
    run: RunId,
) -> Result<StoreMissingProbe, String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let workflow = action_then_finish_workflow()?;
    let contracts = Box::from([action_contract(ActionId::new(0))?]);

    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    let ticket = pending_ticket(run, ActionId::new(0));
    let contract = contracts
        .first()
        .ok_or_else(|| String::from("missing action contract"))?;
    let action_abi_digest = action_contract_abi_digest(contract)?;
    write_pending_action_events_with_input(
        &journal,
        run,
        workflow.digest(),
        artifact.digest,
        action_grants(ActionId::new(0)),
        RuntimePolicy::Strict,
        ticket,
        action_abi_digest,
        input_value,
    )?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let storage_requires_value_store = storage_seed_requires_value_store(&journal, run)?;
    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let recovered = Runtime::new(shard_count(1)?, strict_config(), shared);
    let product = recovered
        .recover_product(run)
        .map_err(|err| err.to_string())?;
    Ok(StoreMissingProbe {
        product,
        storage_requires_value_store,
    })
}

fn assert_storage_typed_witness_carries_through(
    journal: &Arc<FjallJournal>,
    run: RunId,
    expected_reason: &'static str,
    assert_storage_flag: impl Fn(&vb_storage::recovery::RecoveryCannotResumeState) -> bool,
) -> Result<(), String> {
    let events = journal
        .events_for_run_full(run)
        .map_err(|err| err.to_string())?;
    let typed_product = vb_storage::recovery::recover_runtime_frame_seed_from_events(&events)
        .map_err(|err| format!("typed storage recovery failed: {err}"))?;
    let state = typed_product.cannot_resume_state();
    if !assert_storage_flag(&state) {
        return Err(format!(
            "typed storage `RecoveryFrameSeedProduct` witness missing flag for \
             cannot-resume reason {expected_reason:?}: state={state:?}"
        ));
    }
    Ok(())
}

#[test]
fn runtime_recover_product_cannot_resume_witness_carries_through_for_store_missing()
-> Result<(), String> {
    // FINDING-002 (bead vb-w25-runtime-a2) non-tautological closure.
    //
    // The prior form asserted `state.store_missing` on both the runtime
    // product and the storage typed witness for a single List-valued run.
    // That was tautological: the storage `RecoveryFrameSeed` classifier
    // (`RecoveryCannotResumeState::from_seed`) marks the ENTIRE live
    // `RunState` missing unconditionally via
    // `mark_missing_components(MissingRunStateComponents::ALL)`, so the
    // storage typed witness reports `store_missing = true` for ANY recovered
    // seed regardless of slot contents. Asserting that constant proved
    // nothing.
    //
    // The runtime layer REFINES that conservative storage witness: in
    // `vb_runtime::recovery::classify_full_recovery_resume` the
    // `store_missing` flag is retained only when a recovered slot value
    // actually requires the cold value store (`SlotValue::List` /
    // `Object` / `Blob`). This test drives two journal-event sets that
    // differ ONLY in the single seeded slot value and proves the runtime
    // product tracks the storage-recovered value-store evidence rather than
    // a constant:
    //   - value-store-required (List)  -> runtime `recover_product`
    //     classifies `store_missing` and returns
    //     `CannotResume { reason: "store_missing" }`.
    //   - scalar (I64), byte-identical otherwise -> runtime refines
    //     `store_missing` away and the product is `Resumable`.
    // In both cases the storage-recovered seed's slot payloads are
    // inspected independently so the runtime decision is proven to be
    // derived from durable storage evidence, not invented.

    // ---- value-store-required run: List slot value ----
    let list = recover_store_missing_probe(SlotValue::List(ListId::new(0)), RunId::new(7_710))?;
    if !list.storage_requires_value_store {
        return Err(String::from(
            "expected storage-recovered List run to carry a value-store-required slot value",
        ));
    }
    match list.product {
        RuntimeRecoveryProduct::CannotResume(product) => {
            if product.reason() != "store_missing" {
                return Err(format!(
                    "expected CannotResume(store_missing) for List run, got reason {:?}",
                    product.reason()
                ));
            }
            if !product.state().store_missing {
                return Err(String::from(
                    "runtime CannotResume witness dropped the store_missing flag for the List run",
                ));
            }
        }
        other => {
            return Err(format!(
                "expected CannotResume(store_missing) for List run, got {other:?}"
            ));
        }
    }

    // ---- scalar run: I64 slot value, otherwise byte-identical ----
    let scalar = recover_store_missing_probe(SlotValue::I64(101), RunId::new(7_713))?;
    if scalar.storage_requires_value_store {
        return Err(String::from(
            "expected storage-recovered scalar run to carry NO value-store-required slot value",
        ));
    }
    if !scalar.product.is_resumable() {
        return Err(format!(
            "expected Resumable for scalar-only run (store_missing refined away), got {:?}",
            scalar.product
        ));
    }
    Ok(())
}

#[test]
fn runtime_recover_product_cannot_resume_witness_carries_through_for_pending_timers()
-> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_711);
    let workflow = wait_then_finish_workflow()?;
    let contracts: Box<[ActionContract]> = Box::from([]);

    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    write_wait_boundary_events(&journal, run, &workflow, artifact.digest)?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let recovered = Runtime::new(shard_count(1)?, strict_config(), shared);

    let product = recovered
        .recover_product(run)
        .map_err(|err| err.to_string())?;
    assert_product_cannot_resume(product, "pending_timers")?;
    assert_storage_typed_witness_carries_through(&journal, run, "pending_timers", |state| {
        state.pending_timers
    })?;
    Ok(())
}

#[test]
fn runtime_recover_product_cannot_resume_witness_carries_through_for_pending_asks()
-> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|err| err.to_string())?;
    let run = RunId::new(7_712);
    let workflow = timed_ask_then_finish_workflow()?;
    let contracts: Box<[ActionContract]> = Box::from([]);

    let journal = open_journal(temp.path())?;
    let artifact =
        submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &contracts)
            .map_err(|err| err.to_string())?;
    write_ask_boundary_events(&journal, run, &workflow, artifact.digest)?;
    drop(journal);

    let journal = open_journal(temp.path())?;
    let shared = StorageRuntimeJournal::shared_strict(journal.clone());
    let recovered = Runtime::new(shard_count(1)?, strict_config(), shared);

    let product = recovered
        .recover_product(run)
        .map_err(|err| err.to_string())?;
    assert_product_cannot_resume(product, "pending_asks")?;
    assert_storage_typed_witness_carries_through(&journal, run, "pending_asks", |state| {
        state.pending_asks
    })?;
    Ok(())
}
