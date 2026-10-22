//! Real runtime producer fixture for vb-ko29.5 idempotency tests.
//!
//! Uses the actual Runtime, StorageRuntimeJournal, FjallJournal, and real
//! action contracts to produce genuine tickets and journal events.
//! No fabricated or struct-literal tickets.

use std::sync::Arc;


use vb_core::action::{
    ActionContract, ActionName, ActionOutputReady, ActionTicket, Idempotency, RetrySafety,
    SideEffect,
};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{FjallJournal, JournalEvent};

pub(crate) fn shard_count() -> Result<std::num::NonZeroUsize, String> {
    std::num::NonZeroUsize::new(1).ok_or_else(|| "shard count must be non-zero".to_string())
}
// ABI digest is now extracted from the journal event after scheduling.
fn canonical_workflow_digest(parts: &WorkflowParts) -> Result<WorkflowDigest, String> {
    let mut canonical = parts.clone();
    canonical.digest = WorkflowDigest::from_bytes([0u8; 32]);
    let bytes =
        postcard::to_allocvec(&canonical).map_err(|e| format!("workflow encode failed: {e}"))?;
    Ok(WorkflowDigest::from_bytes(*blake3::hash(&bytes).as_bytes()))
}

fn produce_workflow() -> Result<CompiledWorkflow, String> {
    let do_node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("ko29-5-producer"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::from([do_node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    parts.digest = canonical_workflow_digest(&parts)?;
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| format!("producer workflow invalid: {e:?}"))
}

fn action_contract() -> Result<ActionContract, String> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("ko29-5-action").map_err(|e| e.to_string())?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([]),
    })
}


fn action_output(value: SlotValue) -> Result<ActionOutputReady, String> {
    let encoded =
        postcard::to_allocvec(&value).map_err(|e| format!("slot encoding failed: {e}"))?;
    let encoded_len =
        u32::try_from(encoded.len()).map_err(|e| format!("slot length overflowed u32: {e}"))?;
    Ok(ActionOutputReady {
        output_slot: SlotIdx::new(1),
        value,
        taint: Taint::Clean,
        encoded_len,
    })
}

fn pending_ticket(runtime: &Runtime, run: RunId) -> Result<ActionTicket, String> {
    runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or_else(|| format!("run {run:?} has no pending action ticket"))
}

/// Run the producer workflow once with the given RunId, completing the action.
/// Returns the actual ticket and the action contract's ABI digest.
pub(crate) fn produce_completed_run_with_abi(
    path: &std::path::Path,
    run: RunId,
) -> Result<(ActionTicket, WorkflowDigest), String> {
    let journal = Arc::new(FjallJournal::open(path, None).map_err(|e| e.to_string())?);
    let workflow = produce_workflow()?;
    let contract = action_contract()?;
    let contracts = Box::from([contract]);
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
    )
    .map_err(|e| format!("artifact submit failed: {e}"))?;

    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
    runtime
        .submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow,
            Box::from([(SlotIdx::ZERO, SlotValue::I64(0))]),
            CapabilitySet::empty(),
            contracts,
        )
        .map_err(|e| format!("submit failed: {e}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

    let ticket = pending_ticket(&runtime, run)?;
    let journal_events = journal
        .events_for_run(run)
        .map_err(|e| e.to_string())?;
    let scheduled_event = journal_events
        .iter()
        .find(|e| matches!(e, JournalEvent::ActionScheduledTicket { .. }));
    let abi_digest = match scheduled_event {
        Some(JournalEvent::ActionScheduledTicket { action_abi_digest, .. }) => {
            *action_abi_digest
        }
        _ => {
            return Err("scheduled ticket event not found in journal".to_string());
        }
    };
    runtime
        .complete_action_with_output(ticket, action_output(SlotValue::I64(42))?)
        .map_err(|e| format!("complete_action failed: {e}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed: {e}"))?;

    let counters = runtime.counters_snapshot();
    if counters.runs_completed != 1 {
        return Err(format!(
            "expected 1 completed run, got {}",
            counters.runs_completed
        ));
    }

    Ok((ticket, abi_digest))
}