use super::super::{SOURCE, compiled_source};
use tempfile::TempDir;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value::join_taint;
use vb_core::{CompiledNodeKind as K, SlotValue, Taint, WorkflowDigest};
use vb_storage::{FjallJournal, RunHeaderRecord, RunSubmission, WorkflowSourceRecord};

const RUN: vb_core::RunId = vb_core::RunId::new(41_702);
type FixtureResult<T = ()> = Result<T, String>;

pub(super) struct Case {
    pub(super) directory: TempDir,
    pub(super) source_digest: WorkflowDigest,
    pub(super) body: vb_core::StepIdx,
    pub(super) output: vb_core::SlotIdx,
    pub(super) finish: vb_core::StepIdx,
    pub(super) finish_result: vb_core::SlotIdx,
    pub(super) lane_items: [(SlotValue, Taint); 3],
}

pub(super) fn case() -> FixtureResult<Case> {
    let workflow = compiled_source()?;
    let mut body = None;
    let mut finish = None;
    let mut finish_result = None;
    for node in &workflow.to_parts().nodes {
        match &node.kind {
            K::ForEachConcurrentStart {
                body: lane,
                at_once,
                ..
            } => {
                assert_eq!(at_once.get(), 2, "owned fixture must use at_once: 2");
                body = Some(*lane);
            }
            K::Finish { result } => {
                finish = Some(node.id);
                finish_result = Some(*result);
            }
            _ => {}
        }
    }
    let body = body.ok_or_else(|| "compiled source lacks concurrent ForEach".to_owned())?;
    let finish = finish.ok_or_else(|| "compiled source lacks Finish".to_owned())?;
    let finish_result = finish_result.ok_or_else(|| "Finish result slot missing".to_owned())?;
    let output = workflow
        .node(body)
        .and_then(|node| node.output.slot())
        .ok_or_else(|| format!("lane body {body:?} has no output slot"))?;
    let lane_items = [
        (SlotValue::I64(11), Taint::Secret),
        (SlotValue::I64(22), Taint::Clean),
        (SlotValue::I64(33), Taint::DerivedFromSecret),
    ];
    let items = lane_items.map(|(value, _)| value);
    let item_taints = lane_items.map(|(_, taint)| taint);
    let mut store = vb_core::ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let root_taint = item_taints.iter().copied().fold(Taint::Clean, join_taint);
    let root = store
        .insert_list_with_taint(items.into(), item_taints.into())
        .map_err(|error| format!("owned input list: {error}"))?;
    let input = workflow
        .input_slots()
        .first()
        .ok_or_else(|| "compiled input missing".to_owned())?
        .slot;
    let bundle = OwnedRootBundle::new(
        Box::from([(input, SlotValue::List(root), root_taint)]),
        store.checkpoint_v1(),
        &workflow,
    )
    .map_err(|error| format!("owned root bundle: {error}"))?;
    let input_store = bundle.checkpoint().clone();
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        vb_core::RuntimePolicy::Strict,
        &[],
    )
    .map_err(|error| format!("accepted artifact: {error:?}"))?;
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(SOURCE.as_bytes()).into());
    let source = WorkflowSourceRecord {
        digest: source_digest,
        source: SOURCE.as_bytes().to_vec(),
    };
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        RUN,
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )
    .map_err(|error| format!("owned run inputs: {error:?}"))?;
    let submission = RunSubmission::owned(
        source,
        artifact.clone(),
        inputs,
        RunHeaderRecord {
            run: RUN,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )
    .map_err(|error| format!("owned submission: {error:?}"))?;
    let directory = tempfile::tempdir().map_err(|error| format!("Fjall directory: {error}"))?;
    let journal = FjallJournal::open(directory.path(), None)
        .map_err(|error| format!("open Fjall: {error}"))?;
    journal
        .commit_run_submission(&submission, vb_core::RuntimePolicy::Strict)
        .map_err(|error| format!("strict submission: {error:?}"))?;
    let persisted = journal
        .owned_run_inputs(RUN, artifact.digest, source_digest, &workflow)
        .map_err(|error| format!("read owned receipt: {error:?}"))?
        .ok_or_else(|| "strict admission omitted owned receipt".to_owned())?;
    assert_eq!(persisted.checkpoint(), &input_store);
    drop(journal);
    Ok(Case {
        directory,
        source_digest,
        body,
        output,
        finish,
        finish_result,
        lane_items,
    })
}
