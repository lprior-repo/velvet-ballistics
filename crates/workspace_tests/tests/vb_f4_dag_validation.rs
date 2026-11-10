#![forbid(unsafe_code)]
use vb_core::input_validation::validate_input_value;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{
    CompiledWorkflow, RunId, RuntimePolicy, SlotValue, Taint, ValueStore, WorkflowDigest,
};
use vb_storage::{
    FjallJournal, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission, WorkflowSourceRecord,
};
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const RUN: RunId = RunId::new(94_477);
const SOURCE: &str = "version: velvet-ballistics/v1\nname: shared_dag\nwhen: {manual: {}}\ninputs:\n  data: {is: list, of: any, max: 2}\nsteps:\n  - id: answer\n    set: {output: answer, value: '42'}\n  - id: done\n    finish: {result: answer}\n";

fn fixture() -> TestResult<(CompiledWorkflow, ValueStore, SlotValue)> {
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes())?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let mut root = SlotValue::List(store.insert_list(Box::from([SlotValue::I64(7)]))?);
    for _ in 0..19 {
        root = SlotValue::List(store.insert_list(Box::from([root, root]))?);
    }
    let cp = store.checkpoint_v1();
    assert_eq!(cp.lists.len(), 20);
    assert_eq!(cp.lists.iter().map(|list| list.len()).sum::<usize>(), 39);
    Ok((workflow, store, root))
}

#[test]
fn twenty_list_dag_direct_validation() -> TestResult {
    let (workflow, store, root) = fixture()?;
    let declaration = workflow.input_slots().first().ok_or("input")?;
    assert_eq!(
        validate_input_value(
            declaration,
            root,
            workflow.symbols_count(),
            Some(&store.checkpoint_v1())
        ),
        Ok(())
    );
    Ok(())
}

#[test]
fn twenty_list_dag_public_bundle() -> TestResult {
    let (workflow, store, root) = fixture()?;
    let declaration = workflow.input_slots().first().ok_or("input")?;
    let result = OwnedRootBundle::new(
        Box::new([(declaration.slot, root, Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    );
    assert_eq!(
        result
            .as_ref()
            .map(|_| ())
            .map_err(|error| format!("{error:?}")),
        Ok(())
    );
    let bundle = result?;
    assert_eq!(bundle.root(declaration.slot), Some((root, Taint::Clean)));
    assert_eq!(bundle.checkpoint().lists, store.checkpoint_v1().lists);
    Ok(())
}

#[test]
fn twenty_list_dag_record_admission_and_restore() -> TestResult {
    let (workflow, store, root) = fixture()?;
    let declaration = workflow.input_slots().first().ok_or("input")?;
    let bundle = OwnedRootBundle::new(
        Box::new([(declaration.slot, root, Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE.as_bytes()).into()),
        source: SOURCE.as_bytes().to_vec(),
    };
    let record =
        OwnedRunInputsRecordV1::new(RUN, artifact.digest, source.digest, &bundle, &workflow)?;
    let retained_bytes = postcard::to_allocvec(&record)?;
    let submission = RunSubmission::owned(
        source,
        artifact.clone(),
        record,
        RunHeaderRecord {
            run: RUN,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )?;
    let directory = tempfile::tempdir()?;
    {
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    }
    let journal = FjallJournal::open(directory.path(), None)?;
    let events = journal.events_for_run_full(RUN)?;
    let admission = events
        .iter()
        .find(|event| {
            matches!(
                event,
                vb_storage::JournalEvent::RunAdmission { .. }
                    | vb_storage::JournalEvent::RunAdmissionBudget { .. }
            )
        })
        .ok_or("admission")?;
    let retained = journal
        .admitted_run_inputs(RUN, artifact.digest, &workflow, admission)?
        .ok_or("record")?;
    let vb_storage::RetainedRunInputs::Owned(retained) = retained else {
        return Err("owned record expected".into());
    };
    assert_eq!(postcard::to_allocvec(&retained)?, retained_bytes);
    let restored = retained.restore(&workflow)?;
    assert_eq!(restored.root(declaration.slot), Some((root, Taint::Clean)));
    assert_eq!(restored.checkpoint(), bundle.checkpoint());
    Ok(())
}

#[test]
fn defaults_pass_preserves_real_shared_list_dag() -> TestResult {
    let (workflow, store, root) = fixture()?;
    let declaration = workflow.input_slots().first().ok_or("input")?;
    let mut checkpoint = store.checkpoint_v1();
    let before = checkpoint.clone();
    assert_eq!(
        vb_core::input_schema::apply_input_schema_defaults(
            declaration.kind,
            &declaration.constraint,
            root,
            Taint::Clean,
            &mut checkpoint
        ),
        Ok(Taint::Clean)
    );
    assert_eq!(checkpoint, before);
    Ok(())
}

#[path = "vb_f4_dag_validation/controls.rs"]
mod controls;

#[path = "vb_f4_dag_validation/properties.rs"]
mod properties;
