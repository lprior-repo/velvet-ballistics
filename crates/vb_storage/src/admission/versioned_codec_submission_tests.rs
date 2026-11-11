use super::*;
use crate::{JournalEvent, RunHeaderRecord, RunInputsRecord, RunSubmission, WorkflowSourceRecord};
use vb_core::{RunId, WorkflowId};

fn submission(held: SchemaVersion, policy: RuntimePolicy) -> TestResult<RunSubmission> {
    let workflow = bound(&["key"])?;
    let mut accepted = prepare_compiled_artifact(&workflow, policy, &[])?;
    accepted.artifact_version = held;
    bind_artifact_digest(&mut accepted)?;
    let source = b"versioned source".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let inputs = RunInputsRecord::new(
        RunId::new(7),
        accepted.digest,
        digest,
        &workflow,
        Box::new([]),
    )?;
    Ok(RunSubmission::new(
        WorkflowSourceRecord { digest, source },
        accepted,
        inputs,
        header(&workflow),
    )?)
}

fn header(workflow: &CompiledWorkflow) -> RunHeaderRecord {
    RunHeaderRecord {
        run: RunId::new(7),
        workflow_id: WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    }
}

fn checked_decision(held: SchemaVersion, policy: RuntimePolicy, count: usize) -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = crate::FjallJournal::open(dir.path(), None)?;
    journal.commit_run_submission(&submission(held, policy)?, policy)?;
    let events = journal.events_for_run(RunId::new(7))?;
    assert_eq!(events.len(), count);
    if count == 0 {
        return Ok(());
    }
    let JournalEvent::CompatibilityDecisionEvent { run, seq, decision } =
        events.get(2).ok_or("missing decision")?
    else {
        return Err("expected compatibility event at sequence2".into());
    };
    assert_eq!(*run, RunId::new(7));
    assert_eq!(*seq, EventSeq::new(2));
    let expected = crate::compat::decide_boundary(
        crate::compat::artifact_ir_version_inputs_for(held, SchemaVersion::new(2, 0, 0)),
        crate::compat::schema_version_digest(held),
        crate::compat::schema_version_digest(SchemaVersion::new(2, 0, 0)),
    );
    assert_eq!(*decision, expected);
    Ok(())
}

#[test]
fn versioned_codec_submission_v2_checked_and_relaxed_policies() -> TestResult {
    checked_decision(SchemaVersion::new(2, 0, 0), RuntimePolicy::Strict, 3)?;
    checked_decision(SchemaVersion::new(2, 1, 0), RuntimePolicy::Journaled, 3)?;
    checked_decision(SchemaVersion::new(2, 0, 0), RuntimePolicy::Relaxed, 0)
}
