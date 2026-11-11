use super::*;
use crate::compat::{CompatibilityClass, SchemaVersion};
use crate::{AcceptedArtifact, EventSeq};
use vb_core::workflow::WorkflowIrV2;

type FixtureResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn construct(
    workflow: &CompiledWorkflow,
    artifact: AcceptedArtifact,
    value: i64,
) -> Result<RunSubmission, JournalError> {
    let source = b"original source fixture".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let inputs = RunInputsRecord::new(
        RunId::new(7),
        artifact.digest,
        digest,
        workflow,
        Box::from([(SlotIdx::new(0), SlotValue::I64(value))]),
    )?;
    RunSubmission::new(
        WorkflowSourceRecord { digest, source },
        artifact,
        inputs,
        header(workflow),
    )
}

fn header(workflow: &CompiledWorkflow) -> RunHeaderRecord {
    RunHeaderRecord {
        run: RunId::new(7),
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    }
}

fn rejected_without_writes(
    workflow: &CompiledWorkflow,
    mut artifact: AcceptedArtifact,
    check: fn(JournalError) -> TestResult,
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    assert_empty(&journal)?;
    crate::admission::bind_artifact_digest(&mut artifact)?;
    match construct(workflow, artifact, 5) {
        Err(error) => check(error)?,
        Ok(_) => return Err("unsupported or mismatched IR constructed a submission".into()),
    }
    assert_empty(&journal)
}

fn exact_future(error: JournalError) -> TestResult {
    match error {
        JournalError::ArtifactVersionIncompatible {
            held,
            required,
            class,
        } => {
            assert_eq!(held, SchemaVersion::new(3, 0, 0));
            assert_eq!(required, SchemaVersion::new(2, 0, 0));
            assert_eq!(class, CompatibilityClass::Rejected);
            Ok(())
        }
        other => Err(format!("expected future-version rejection, found {other:?}").into()),
    }
}

pub(super) fn future_rejection(policy: RuntimePolicy) -> TestResult {
    let workflow = super::super::tests::workflow()?;
    let mut artifact = crate::admission::prepare_compiled_artifact(&workflow, policy, &[])?;
    artifact.artifact_version = SchemaVersion::new(3, 0, 0);
    rejected_without_writes(&workflow, artifact.clone(), exact_future)?;
    artifact.ir.clear();
    rejected_without_writes(&workflow, artifact, exact_future)
}

fn exact_malformed(error: JournalError) -> TestResult {
    match error {
        JournalError::ArtifactMalformed => Ok(()),
        other => Err(format!("expected outer2/V1 mismatch, found {other:?}").into()),
    }
}

pub(super) fn wrong_wire_rejection(policy: RuntimePolicy) -> TestResult {
    let workflow = super::super::tests::workflow()?;
    let mut artifact = crate::admission::prepare_compiled_artifact(&workflow, policy, &[])?;
    artifact.artifact_version = SchemaVersion::new(2, 0, 0);
    rejected_without_writes(&workflow, artifact, exact_malformed)
}

pub(super) fn bound_workflow() -> FixtureResult<CompiledWorkflow> {
    let mut parts = super::super::tests::workflow()?.to_parts();
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    let mut ir = WorkflowIrV2 {
        wire_version: 2,
        parts,
        symbol_payloads: Box::new([]),
    };
    let mut preimage = vec![0x01, 0xff, 0x56, 0x42];
    preimage.extend_from_slice(&postcard::to_allocvec(&ir)?);
    ir.parts.digest = WorkflowDigest::from_bytes(blake3::hash(&preimage).into());
    Ok(CompiledWorkflow::try_from_parts_with_symbols(
        ir.parts,
        ir.symbol_payloads,
    )?)
}

pub(super) fn relaxed_submission(
    workflow: &CompiledWorkflow,
    version: SchemaVersion,
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    let artifact =
        crate::admission::prepare_compiled_artifact(workflow, RuntimePolicy::Relaxed, &[])?;
    assert_eq!(artifact.artifact_version, version);
    assert_eq!(artifact.accepted_at_seq, EventSeq::new(0));
    let submission = construct(workflow, artifact.clone(), 5)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Relaxed)?;
    assert_eq!(journal.events_for_run(RunId::new(7))?.len(), 0);
    check_retained(&journal, workflow, &artifact)
}

fn check_retained(
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
    expected: &AcceptedArtifact,
) -> TestResult {
    let inputs = journal
        .run_inputs(RunId::new(7))?
        .ok_or("missing retained inputs")?;
    assert_eq!(inputs.inputs(), [(SlotIdx::new(0), SlotValue::I64(5))]);
    assert_eq!(inputs.artifact_digest(), expected.digest);
    let stored = journal
        .compiled_ir(expected.digest)?
        .ok_or("missing artifact")?;
    let restored: AcceptedArtifact = postcard::from_bytes(&stored.ir)?;
    assert_eq!(&restored, expected);
    let decoded = crate::admission::verify_accepted_artifact(&restored)?;
    assert_eq!(&decoded, workflow);
    assert_eq!(decoded.symbol_payloads(), workflow.symbol_payloads());
    assert_eq!(journal.run_header(RunId::new(7))?, Some(header(workflow)));
    let source = journal
        .workflow_source(inputs.source_digest())?
        .ok_or("missing retained source")?;
    assert_eq!(source.source, b"original source fixture");
    Ok(())
}
