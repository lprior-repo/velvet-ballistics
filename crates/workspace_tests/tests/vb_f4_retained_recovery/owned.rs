use super::*;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{CompiledWorkflow, ObjectField, SlotIdx, Taint};
use vb_runtime::runtime::completion_observation::OwnedCompletionScalar;

pub(super) struct Case {
    pub(super) directory: tempfile::TempDir,
    pub(super) workflow: CompiledWorkflow,
    pub(super) artifact: vb_storage::AcceptedArtifact,
    pub(super) source: WorkflowSourceRecord,
    pub(super) input: SlotIdx,
    pub(super) record_bytes: Vec<u8>,
    pub(super) checkpoint: vb_core::value_store::ValueStoreCheckpointV1,
}

impl Case {
    pub(super) fn new(expression: &str) -> TestResult<Self> {
        let source = format!(
            "version: velvet-ballistics/v1\nname: retained_recursive\nwhen: {{manual: {{}}}}\ninputs:\n  data: {{is: object, extra: reject, fields: {{profile: {{is: object, default: {{}}, extra: reject, fields: {{count: {{is: number, min: 2, default: 17}}}}}}, left: {{is: list, of: number, max: 3}}, right: {{is: list, of: number, max: 3}}}}}}\nsteps:\n  - id: select\n    set: {{output: answer, value: '{expression}'}}\n  - id: done\n    finish: {{result: '$answer'}}\n"
        );
        let workflow = vb_compile::compile_workflow(source.as_bytes())?;
        let input = workflow.input_slots().first().ok_or("input")?.slot;
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let left = store.insert_symbol("left")?;
        let right = store.insert_symbol("right")?;
        let list = store.insert_list(Box::from([
            SlotValue::I64(7),
            SlotValue::I64(1),
            SlotValue::I64(7),
        ]))?;
        let object = store.insert_object(Box::from([
            ObjectField::clean(left, SlotValue::List(list)),
            ObjectField::clean(right, SlotValue::List(list)),
        ]))?;
        let bundle = OwnedRootBundle::new(
            Box::from([(input, SlotValue::Object(object), Taint::Clean)]),
            store.checkpoint_v1(),
            &workflow,
        )?;
        let artifact = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            &[],
        )?;
        let source = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into()),
            source: source.into_bytes(),
        };
        let record = vb_storage::OwnedRunInputsRecordV1::new(
            RUN,
            artifact.digest,
            source.digest,
            &bundle,
            &workflow,
        )?;
        let record_bytes = postcard::to_allocvec(&record)?;
        let submission = RunSubmission::owned(
            source.clone(),
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
        Ok(Self {
            directory,
            workflow,
            artifact,
            source,
            input,
            record_bytes,
            checkpoint: bundle.checkpoint().clone(),
        })
    }

    pub(super) fn open(&self) -> TestResult<Arc<FjallJournal>> {
        Ok(Arc::new(FjallJournal::open(self.directory.path(), None)?))
    }
}

fn finish(expression: &str, expected: OwnedCompletionScalar) -> TestResult {
    let case = Case::new(expression)?;
    let journal = case.open()?;
    let mut runtime = runtime(journal.clone());
    runtime.recover_and_resume(RUN)?;
    let mut slots = [CompletionObservationSlot::default()];
    let outcome = runtime
        .tick_all_observed(&mut slots)
        .map_err(|error| format!("observe: {error:?}"))?;
    outcome.execution?;
    outcome
        .observation
        .map_err(|error| format!("observe conflict: {error:?}"))?;
    assert_eq!(
        slots
            .first()
            .and_then(|slot| slot.first)
            .ok_or("completion")?
            .result,
        CompletionValueObservation::Available(expected)
    );
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    let record = journal
        .owned_run_inputs(
            RUN,
            case.artifact.digest,
            case.source.digest,
            &case.workflow,
        )?
        .ok_or("retained record")?;
    assert_eq!(record.checkpoint(), &case.checkpoint);
    assert_eq!(postcard::to_allocvec(&record)?, case.record_bytes);
    Ok(())
}

#[test]
fn pristine_owned_recursive_default_is_read_after_fjall_reopen() -> TestResult {
    finish("$input.data.profile.count", OwnedCompletionScalar::I64(17))
}

#[test]
fn pristine_owned_shared_list_identity_is_preserved_after_fjall_reopen() -> TestResult {
    finish(
        "$input.data.left == $input.data.right",
        OwnedCompletionScalar::Bool(true),
    )
}
