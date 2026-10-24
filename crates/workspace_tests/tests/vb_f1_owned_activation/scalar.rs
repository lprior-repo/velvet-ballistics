use super::support::{TestResult, config, runtime};
use vb_core::{RunId, RuntimePolicy, SlotValue, WorkflowDigest};
use vb_runtime::RuntimeError;
use vb_runtime::journal::StorageRuntimeJournal;
use vb_storage::{FjallJournal, RunInputsRecord, RunSubmission};

const SOURCE: &str = r#"version: velvet-ballistics/v1
name: scalar_activation_refusal
when:
  manual: {}
inputs:
  number: { is: number }
steps:
  - id: done
    finish: { result: "$input.number" }
"#;

fn submission() -> TestResult<(RunSubmission, WorkflowDigest)> {
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes()).map_err(|e| format!("{e:?}"))?;
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source = vb_storage::WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE.as_bytes()).into()),
        source: SOURCE.as_bytes().to_vec(),
    };
    let run = RunId::new(29);
    let slot = workflow
        .input_slots()
        .first()
        .ok_or("scalar input missing")?
        .slot;
    let inputs = RunInputsRecord::new(
        run,
        artifact.digest,
        source.digest,
        &workflow,
        Box::from([(slot, SlotValue::I64(91))]),
    )?;
    let header = super::support::header(run, &workflow);
    Ok((
        RunSubmission::new(source.clone(), artifact, inputs, header)?,
        source.digest,
    ))
}

#[test]
fn actual_scalar_submission_is_never_reinterpreted_as_owned() -> TestResult {
    let (submission, source) = submission()?;
    let directory = tempfile::tempdir()?;
    {
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    }
    let journal = std::sync::Arc::new(FjallJournal::open(directory.path(), None)?);
    let before = journal.events_for_run_full(RunId::new(29))?;
    let runtime = runtime(
        StorageRuntimeJournal::shared_strict(journal.clone()),
        config(10, 8, 8),
    )?;
    assert!(
        matches!(runtime.activate_owned_submission(RunId::new(29), source),
        Err(RuntimeError::StorageJournalAppend { source }) if matches!(*source,
            vb_storage::JournalError::RecordKindPayloadMismatch { envelope_kind: 4, payload_kind: 42 }))
    );
    assert_eq!(journal.events_for_run_full(RunId::new(29))?, before);
    assert_eq!(runtime.pending_boundary_snapshot(8).active_run_count(), 0);
    Ok(())
}
