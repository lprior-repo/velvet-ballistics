use vb_compile::compile_workflow;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value_store::ObjectField;
use vb_core::{
    CompiledWorkflow, InputSlotKind, RunId, RuntimePolicy, SlotIdx, SlotValue, Taint, ValueStore,
    WorkflowDigest,
};
use vb_storage::{
    AcceptedArtifact, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission, WorkflowSourceRecord,
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const SOURCE: &str = r#"version: velvet-ballistics/v1
name: owned_inputs
when:
  manual: {}
inputs:
  entries: { is: list, of: any, max: 8 }
  flag: { is: boolean }
steps:
  - id: done
    finish:
      result: "$input.entries"
"#;

pub(super) struct Fixture {
    pub(super) workflow: CompiledWorkflow,
    pub(super) artifact: AcceptedArtifact,
    pub(super) source: WorkflowSourceRecord,
    pub(super) bundle: OwnedRootBundle,
    pub(super) run: RunId,
}

pub(super) fn input_slot(wf: &CompiledWorkflow, kind: InputSlotKind) -> TestResult<SlotIdx> {
    wf.input_slots()
        .iter()
        .find(|v| v.kind == kind)
        .map(|v| v.slot)
        .ok_or_else(|| "declared input missing".into())
}

fn arenas(
    wf: &CompiledWorkflow,
    first: i64,
    item: Taint,
) -> TestResult<(ValueStore, vb_core::ListId)> {
    let mut store = ValueStore::with_max_slots(wf.resource_contract().max_slots);
    let key = store.insert_symbol("child")?;
    let blob = store.insert_blob(vec![3u8, 1, 4])?;
    let child = store.insert_list_with_taint(
        Box::from([SlotValue::I64(first), SlotValue::I64(1), SlotValue::I64(7)]),
        Box::from([Taint::Clean, item, Taint::Secret]),
    )?;
    let object = store.insert_object(
        vec![ObjectField::with_taint(
            key,
            SlotValue::List(child),
            Taint::Secret,
        )]
        .into_boxed_slice(),
    )?;
    let middle = store.insert_list(vec![SlotValue::Blob(blob)].into_boxed_slice())?;
    let root = store.insert_list_with_taint(
        Box::from([
            SlotValue::I64(7),
            SlotValue::Object(object),
            SlotValue::List(middle),
            SlotValue::I64(7),
        ]),
        Box::from([
            Taint::Clean,
            Taint::Secret,
            Taint::Clean,
            Taint::DerivedFromSecret,
        ]),
    )?;
    Ok((store, root))
}

pub(super) fn fixture(first: i64, item: Taint, root: Taint) -> TestResult<Fixture> {
    let workflow = compile_workflow(SOURCE.as_bytes()).map_err(|e| format!("{e:?}"))?;
    let (store, root_id) = arenas(&workflow, first, item)?;
    let entries = input_slot(&workflow, InputSlotKind::List)?;
    let flag = input_slot(&workflow, InputSlotKind::Bool)?;
    let roots = Box::from([
        (flag, SlotValue::Bool(true), Taint::Clean),
        (entries, SlotValue::List(root_id), root),
    ]);
    let bundle = OwnedRootBundle::new(roots, store.checkpoint_v1(), &workflow)?;
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE.as_bytes()).into()),
        source: SOURCE.as_bytes().to_vec(),
    };
    Ok(Fixture {
        workflow,
        artifact,
        source,
        bundle,
        run: RunId::new(17),
    })
}

pub(super) fn record(f: &Fixture) -> TestResult<OwnedRunInputsRecordV1> {
    Ok(OwnedRunInputsRecordV1::new(
        f.run,
        f.artifact.digest,
        f.source.digest,
        &f.bundle,
        &f.workflow,
    )?)
}

pub(super) fn header(f: &Fixture) -> RunHeaderRecord {
    RunHeaderRecord {
        run: f.run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: f.workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    }
}

pub(super) fn submission(f: &Fixture) -> TestResult<RunSubmission> {
    Ok(RunSubmission::owned(
        f.source.clone(),
        f.artifact.clone(),
        record(f)?,
        header(f),
    )?)
}
