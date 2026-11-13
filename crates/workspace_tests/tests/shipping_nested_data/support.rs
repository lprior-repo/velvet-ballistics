//! Source fixture assembly, owned admission, terminal evidence, and step lookup.

use std::time::{SystemTime, UNIX_EPOCH};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value::join_taint;
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{
    CompiledWorkflow, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, Taint, ValueStore,
    WorkflowDigest, WorkflowId,
};
use vb_storage::{
    DecodedSlotWrittenExtra, JournalEvent, RunHeaderRecord, RunSubmission, WorkflowSourceRecord,
    decode_slot_written_extra,
};

use crate::source::{compound, indent};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn current_time_millis() -> TestResult<u64> {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    Ok(u64::try_from(millis)?)
}

pub(crate) const RUN: RunId = RunId::new(7_113);
const LEAF: &str = "- id: leaf\n  set: {output: leaf, value: '9'}\n";
const TAIL: &str = "- id: tail\n  set: {output: tail, value: '$input.items.0'}\n";

/// Builds the frozen helper's nested source: inner compound, per-outer tail,
/// outer compound, terminal over the outer control result.
pub(crate) fn nested_source(outer: &str, inner: &str) -> TestResult<String> {
    let mut body = compound(inner, "inner", LEAF)?;
    body.push_str(TAIL);
    let outer_body = compound(outer, "outer", &body)?;
    Ok(format!(
        "version: velvet-ballistics/v1\nname: shipping_nested_data\nwhen: {{manual: {{}}}}\ninputs: {{items: {{is: list, of: number, max: 2}}}}\nvars: {{sentinel: 42}}\nsteps:\n{}  - id: done\n    finish: {{result: '$steps.outer'}}\n",
        indent(&outer_body, 2)
    ))
}

pub(crate) struct Observed {
    pub terminal_value: SlotValue,
    pub terminal_taint: Taint,
    pub terminal_items: Option<Vec<SlotValue>>,
    pub leaf_successes: usize,
    pub cut_replay_selected: bool,
    pub full_replay_selected: bool,
    pub cut_completed: u64,
    pub leaf_terminal: Option<(SlotValue, Taint)>,
    pub budget_gather_pages: u32,
    pub budget_gather_items: u32,
}

pub(crate) struct Prepared {
    pub workflow: CompiledWorkflow,
    pub roots: Box<[(SlotIdx, SlotValue, Taint)]>,
    pub input_store: ValueStoreCheckpointV1,
    pub submission: RunSubmission,
    pub source_digest: WorkflowDigest,
    pub input_slot: SlotIdx,
}

/// Compiles, fully validates, and builds a real owned submission.
pub(crate) fn prepare(
    source: &str,
    values: &[SlotValue],
    taints: &[Taint],
) -> TestResult<Prepared> {
    prepare_with_contracts(source, values, taints, &[])
}

pub(crate) fn prepare_with_contracts(
    source: &str,
    values: &[SlotValue],
    taints: &[Taint],
    contracts: &[vb_core::action::ActionContract],
) -> TestResult<Prepared> {
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    vb_core::validate_compiled_workflow(&workflow.to_parts())?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    if let Some(metadata) = workflow.source_metadata() {
        vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)?;
    }
    let list = store.insert_list_with_taint(
        values.to_vec().into_boxed_slice(),
        taints.to_vec().into_boxed_slice(),
    )?;
    let root_taint = taints
        .iter()
        .fold(Taint::Clean, |joined, taint| join_taint(joined, *taint));
    let input = workflow
        .input_slots()
        .first()
        .ok_or("missing input slot")?
        .slot;
    let roots = Box::from([(input, SlotValue::List(list), root_taint)]);
    let bundle = OwnedRootBundle::new(roots, store.checkpoint_v1(), &workflow)?;
    let input_store = bundle.checkpoint().clone();
    let roots = bundle.roots().to_vec().into_boxed_slice();
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        RuntimePolicy::Strict,
        contracts,
    )?;
    let source_record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into()),
        source: source.as_bytes().to_vec(),
    };
    let source_digest = source_record.digest;
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        RUN,
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )?;
    let header = RunHeaderRecord {
        run: RUN,
        workflow_id: WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: current_time_millis()?,
    };
    let submission = RunSubmission::owned(source_record, artifact, inputs, header)?;
    Ok(Prepared {
        workflow,
        roots,
        input_store,
        submission,
        source_digest,
        input_slot: input,
    })
}

fn step_with(
    workflow: &CompiledWorkflow,
    predicate: impl Fn(&vb_core::CompiledNodeKind) -> bool,
) -> TestResult<StepIdx> {
    workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| predicate(&node.kind))
        .map(|node| node.id)
        .ok_or_else(|| "step not found".into())
}

pub(crate) fn leaf_step(workflow: &CompiledWorkflow) -> TestResult<StepIdx> {
    step_with(workflow, |kind| {
        matches!(
            kind,
            vb_core::CompiledNodeKind::SetConst { value }
                if workflow.constant(*value) == Some(&vb_core::ConstValue::I64(9))
        )
    })
}

pub(crate) fn succeeded(events: &[JournalEvent], step: StepIdx) -> usize {
    events
        .iter()
        .filter(
            |event| matches!(event, JournalEvent::StepSucceeded { step: recorded, .. } if *recorded == step),
        )
        .count()
}

pub(crate) fn terminal(events: &[JournalEvent]) -> TestResult<(SlotValue, Taint)> {
    let result = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .ok_or("missing RunFinished")?;
    let (value, extra) = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                slot,
                value: Some(value),
                extra,
                ..
            } if *slot == result => Some((value, extra)),
            _ => None,
        })
        .ok_or("missing terminal slot write")?;
    let value = postcard::from_bytes(value)?;
    let extra = extra.as_deref().ok_or("missing taint extra")?;
    let taint = match decode_slot_written_extra(extra).map_err(|error| format!("{error:?}"))? {
        DecodedSlotWrittenExtra::Envelope(envelope) => envelope.taint,
        DecodedSlotWrittenExtra::LegacyFrameExtra(_) => return Err("legacy taint extra".into()),
    };
    Ok((value, taint))
}
