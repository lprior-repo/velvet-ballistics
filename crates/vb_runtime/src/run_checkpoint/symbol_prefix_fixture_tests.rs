#![cfg(test)]

use super::*;
use vb_core::ids::{ActionId, ListId, ObjectId, SlotIdx, SymbolId};
use vb_core::policy::RuntimePolicy;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts, node};
use vb_core::{CapabilitySet, CoreError, SlotValue, Taint};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture_nodes() -> Box<[CompiledNode]> {
    Box::from([
        node(
            StepIdx::ZERO,
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO,
            },
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(1)),
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
            None,
            None,
        ),
    ])
}

fn fixture_parts(symbols_count: u32) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("checkpoint-prefix"),
        digest: WorkflowDigest::from_bytes([42; 32]),
        semantic_source_identity: None,
        nodes: fixture_nodes(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    }
}

pub(crate) fn prefix_workflow(prefix: Vec<Box<str>>) -> TestResult<CompiledWorkflow> {
    let count = u32::try_from(prefix.len())?;
    let mut parts = fixture_parts(count);
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    let ir = vb_core::workflow::WorkflowIrV2 {
        wire_version: 2,
        parts: parts.clone(),
        symbol_payloads: prefix.clone().into(),
    };
    let mut preimage = vec![1_u8, 255, 86, 66];
    preimage.extend(postcard::to_allocvec(&ir)?);
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&preimage).into());
    Ok(CompiledWorkflow::try_from_parts_with_symbols(
        parts,
        prefix.into(),
    )?)
}

pub(crate) fn legacy_workflow() -> TestResult<CompiledWorkflow> {
    Ok(CompiledWorkflow::try_from_parts(fixture_parts(0))?)
}

pub(crate) fn symbol_table(symbols: &[&str]) -> ValueStoreCheckpointV1 {
    let mut checkpoint = ValueStore::with_max_slots(32).checkpoint_v1();
    checkpoint.symbols = symbols.iter().map(|text| Box::from(*text)).collect();
    checkpoint
}

pub(crate) fn populated_store() -> Result<ValueStore, CoreError> {
    let mut checkpoint = symbol_table(&["key", "tail", "key"]);
    checkpoint
        .lists
        .push(Box::from([SlotValue::Object(ObjectId::new(0))]));
    checkpoint.list_taints.push(Box::from([Taint::Secret]));
    checkpoint
        .objects
        .push(Box::from([vb_core::value_store::ObjectField {
            key: SymbolId::new(2),
            value: SlotValue::Symbol(SymbolId::new(1)),
            taint: Taint::Secret,
        }]));
    checkpoint.blobs.push(vec![7_u8, 11].into());
    ValueStore::from_checkpoint_v1(checkpoint)
}

pub(crate) fn fixture_contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: vb_core::action::ActionName::new("checkpoint-action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: vb_core::action::Idempotency::AtLeastOnceExternal,
        side_effect: vb_core::action::SideEffect::None,
        retry_safety: vb_core::action::RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

fn failed_frame() -> TestResult<RunFrame> {
    let mut frame = RunFrame::new(RunId::new(71), StepIdx::ZERO, 2, 2)?;
    frame.write_slot_with_taint(
        SlotIdx::ZERO,
        SlotValue::List(ListId::new(0)),
        Taint::Secret,
    )?;
    frame.mark_running(StepIdx::ZERO)?;
    frame.mark_failed(StepIdx::ZERO)?;
    frame.increment_executed()?;
    Ok(frame)
}

pub(crate) fn fixture_admission(workflow: &CompiledWorkflow) -> TestResult<RunAdmission> {
    Ok(RunAdmission::new(
        WorkflowDigest::from_bytes([43; 32]),
        RunId::new(71),
        CapabilitySet::empty(),
        RuntimePolicy::Relaxed,
    )
    .with_policy_pin(vb_storage::RunPolicyPin::Pinned(
        vb_storage::admission::compute_policy_digest(workflow)?,
    )))
}

fn failed_state(workflow: CompiledWorkflow) -> TestResult<RunState> {
    let admission = fixture_admission(&workflow)?;
    Ok(RunState {
        registered_execution: crate::shard::types::RegisteredExecutionMode::Live,
        frame: failed_frame()?,
        run_attempt: NonZeroU16::MIN,
        workflow,
        store: populated_store()?,
        action_attempts: Box::from([0, 0]),
        admission: Some(admission),
        run_started_millis: Some(101),
        collect_states: CollectStates::new(),
        journal_bytes_written: 19,
        action_contracts: Box::from([fixture_contract()?]),
    })
}

pub(crate) fn retry_checkpoint(workflow: &CompiledWorkflow) -> TestResult<RunCheckpointV1> {
    let state = failed_state(workflow.clone())?;
    Ok(RunCheckpointV1::capture(
        RunId::new(71),
        WorkflowDigest::from_bytes([43; 32]),
        NonZeroU16::new(2).ok_or("invalid fixture attempt")?,
        StepIdx::ZERO,
        9,
        &state,
    )?)
}
