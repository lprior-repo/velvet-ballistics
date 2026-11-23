#![cfg(test)]

use std::path::PathBuf;
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, StepIdx, WorkflowDigest, WorkflowId};
use vb_storage::{
    AcceptedArtifact, EventSeq, FjallJournal, JournalEvent, RunHeaderRecord, RunInputsRecord,
    RunSubmission, WorkflowSourceRecord,
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: replay_exit_codes\nwhen: {manual: {}}\nsteps:\n  - id: first\n    set: {output: first_value, value: '1'}\n  - id: second\n    set: {output: second_value, value: '2'}\n  - id: done\n    finish: {result: first_value}\n";

pub(super) struct Fixture {
    pub(super) path: PathBuf,
    pub(super) run: RunId,
    directory: Option<tempfile::TempDir>,
}

impl Fixture {
    pub(super) fn at_path(path: PathBuf) -> Self {
        Self {
            path,
            run: RunId::new(8921),
            directory: None,
        }
    }

    pub(super) fn new() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let mut fixture = Self::at_path(directory.path().join("journal"));
        fixture.directory = Some(directory);
        let workflow = vb_compile::compile_workflow(SOURCE).map_err(|e| format!("{e:?}"))?;
        let journal = FjallJournal::open(&fixture.path, None)?;
        let prepared = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            &[],
        )?;
        journal.commit_run_submission(
            &submission(&workflow, fixture.run, prepared.clone())?,
            RuntimePolicy::Strict,
        )?;
        assert_issued(&journal, fixture.run, prepared)?;
        assert_clean_prefix(&journal, fixture.run)?;
        Ok(fixture)
    }

    pub(super) fn started(&self, seq: u64, step: u16) -> JournalEvent {
        JournalEvent::StepStarted {
            run: self.run,
            seq: EventSeq::new(seq),
            step: StepIdx::new(step),
            attempt: 1,
        }
    }

    pub(super) fn records(&self) -> TestResult<(Vec<Option<Vec<u8>>>, Vec<u8>)> {
        let journal = FjallJournal::open(&self.path, None)?;
        let events = (0..8)
            .map(|seq| journal.get_event_bytes(self.run, EventSeq::new(seq)))
            .collect::<Result<Vec<_>, _>>()?;
        let inputs = journal.run_inputs(self.run)?.ok_or("retained inputs")?;
        let source = journal.workflow_source(inputs.source_digest())?;
        let artifact = journal.compiled_ir(inputs.artifact_digest())?;
        Ok((
            events,
            postcard::to_allocvec(&(journal.run_header(self.run)?, inputs, source, artifact))?,
        ))
    }
}

fn assert_clean_prefix(journal: &FjallJournal, run: RunId) -> TestResult {
    let events = vb_storage::recovery::recover_full_journal(
        journal,
        run,
        &mut vb_storage::recovery::ActionReplayTracker::new(),
        &[],
        &[],
    )?;
    assert_eq!(events.len(), 4);
    Ok(())
}

fn submission(
    workflow: &CompiledWorkflow,
    run: RunId,
    artifact: AcceptedArtifact,
) -> TestResult<RunSubmission> {
    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE).into()),
        source: SOURCE.to_vec(),
    };
    let inputs = RunInputsRecord::new(
        run,
        artifact.digest,
        source.digest,
        workflow,
        Box::default(),
    )?;
    let header = RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    };
    Ok(RunSubmission::new(source, artifact, inputs, header)?)
}

fn assert_issued(journal: &FjallJournal, run: RunId, mut expected: AcceptedArtifact) -> TestResult {
    let inputs = journal.run_inputs(run)?.ok_or("issued inputs")?;
    let record = journal
        .compiled_ir(inputs.artifact_digest())?
        .ok_or("issued artifact")?;
    let actual: AcceptedArtifact = postcard::from_bytes(&record.ir)?;
    expected.accepted_at_seq = EventSeq::new(1);
    expected.digest = vb_storage::admission::accepted_artifact_digest(&expected)?;
    expected.verification.digest = expected.digest;
    assert_eq!(actual, expected);
    assert_eq!(inputs.artifact_digest(), actual.digest);
    Ok(())
}

pub(super) fn assert_success_report(fixture: &Fixture, value: &serde_json::Value) -> TestResult {
    let journal = FjallJournal::open(&fixture.path, None)?;
    let header = journal.run_header(fixture.run)?.ok_or("header")?;
    assert_eq!(
        value,
        &serde_json::json!({
            "schema_version": "velvet-ballistics/cli-output/v1",
            "kind": "replay_report", "run_id": "8921", "recovered": 4, "terminal": null,
            "events": [
                {"run": 8921, "seq": 0, "type": "RunAccepted", "workflow": format!("{:?}", header.compiled_digest)},
                {"run": 8921, "seq": 1, "type": "RunAdmissionBudget"},
                {"run": 8921, "seq": 2, "type": "CompatibilityDecisionEvent"},
                {"run": 8921, "seq": 3, "type": "RunBudgetOriginEvent"}
            ]
        })
    );
    Ok(())
}
