use super::support::TestResult;
use vb_core::{RunId, RuntimePolicy, SlotValue, WorkflowDigest};
use vb_storage::{
    FjallJournal, JournalError, RecordKind, RetainedRunInputs, RunInputsRecord, RunSubmission,
};

const SOURCE: &str = r#"version: velvet-ballistics/v1
name: scalar_compat
when:
  manual: {}
inputs:
  number: { is: number }
steps:
  - id: done
    finish:
      result: "$input.number"
"#;

type Prepared = (
    vb_core::CompiledWorkflow,
    vb_storage::AcceptedArtifact,
    vb_storage::WorkflowSourceRecord,
    RunSubmission,
    vb_core::WorkflowDigest,
);

fn prepared() -> TestResult<Prepared> {
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes()).map_err(|e| format!("{e:?}"))?;
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let run = RunId::new(29);
    let source = vb_storage::WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE.as_bytes()).into()),
        source: SOURCE.as_bytes().to_vec(),
    };
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
    let expected = vb_storage::inputs::compute_input_digest(inputs.inputs())?;
    let submission = RunSubmission::new(
        source.clone(),
        artifact.clone(),
        inputs,
        vb_storage::RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )?;
    Ok((workflow, artifact, source, submission, expected))
}

#[test]
fn scalar_submission_keeps_kind4_digest_and_reader_semantics() -> TestResult {
    let (workflow, artifact, source, submission, expected) = prepared()?;
    let run = RunId::new(29);
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    assert_eq!(RecordKind::RunInputs.id(), 4);
    assert_eq!(
        journal
            .run_inputs(run)?
            .ok_or("scalar missing")?
            .input_digest(),
        expected
    );
    assert!(matches!(
        journal.retained_run_inputs(run, artifact.digest, source.digest, &workflow)?,
        Some(RetainedRunInputs::Scalar(_))
    ));
    assert!(matches!(
        journal.owned_run_inputs(run, artifact.digest, source.digest, &workflow),
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: 4,
            payload_kind: 42
        })
    ));
    Ok(())
}
