#![cfg(test)]

use std::path::{Path, PathBuf};
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, StepIdx, WorkflowDigest, WorkflowId};
use vb_storage::{
    AcceptedArtifact, EventSeq, FjallJournal, JournalEvent, RunHeaderRecord, RunInputsRecord,
    RunSubmission, WorkflowSourceRecord,
    constants::KEYSPACE_RUN_EVENT,
    types::{KeyspaceProfile, keyspace_options_for},
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

    pub(super) fn records(
        &self,
    ) -> TestResult<(Vec<Option<Vec<u8>>>, Vec<u8>, Vec<(Vec<u8>, Vec<u8>)>)> {
        let (events, metadata) = {
            let journal = FjallJournal::open(&self.path, None)?;
            let events = (0..8)
                .map(|seq| journal.get_event_bytes(self.run, EventSeq::new(seq)))
                .collect::<Result<Vec<_>, _>>()?;
            let inputs = journal.run_inputs(self.run)?.ok_or("retained inputs")?;
            let source = journal.workflow_source(inputs.source_digest())?;
            let artifact = journal.compiled_ir(inputs.artifact_digest())?;
            let metadata =
                postcard::to_allocvec(&(journal.run_header(self.run)?, inputs, source, artifact))?;
            (events, metadata)
        };
        Ok((events, metadata, raw_event_inventory(&self.path)?))
    }
}

fn raw_event_inventory(path: &Path) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let database = fjall::Database::builder(path).open()?;
    let events = database.keyspace(KEYSPACE_RUN_EVENT, || {
        keyspace_options_for(KeyspaceProfile::Hot)
    })?;
    Ok(events
        .iter()
        .map(|guard| {
            let (key, value) = guard.into_inner()?;
            Ok((key.to_vec(), value.to_vec()))
        })
        .collect::<Result<Vec<_>, fjall::Error>>()?)
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
    let events = journal
        .events_for_run_full(fixture.run)?
        .iter()
        .map(expected_event_json)
        .collect::<TestResult<Vec<_>>>()?;
    assert_eq!(events.len(), 4);
    assert_eq!(
        value,
        &serde_json::json!({
            "schema_version": "velvet-ballistics/cli-output/v1",
            "kind": "replay_report", "run_id": "8921", "recovered": 4, "terminal": null,
            "events": events.clone()
        })
    );
    assert_eq!(
        events.first(),
        Some(&serde_json::json!({
            "run": fixture.run.get(), "seq": 0, "type": "RunAccepted",
            "workflow": format!("{:?}", header.compiled_digest)
        }))
    );
    Ok(())
}

fn expected_event_json(event: &JournalEvent) -> TestResult<serde_json::Value> {
    use serde_json::json;

    let value = match event {
        JournalEvent::RunAccepted { workflow, .. } => {
            json!({"type":"RunAccepted","workflow":format!("{workflow:?}")})
        }
        JournalEvent::RunAdmissionBudget {
            artifact_digest,
            input_digest,
            policy_pin,
            granted_capabilities,
            policy,
            budget,
            ..
        } => json!({
            "type":"RunAdmissionBudget",
            "artifact_digest":format!("{artifact_digest:?}"),
            "input_digest":format!("{input_digest:?}"),
            "policy_pin":format!("{policy_pin:?}"),
            "granted_capabilities":format!("{granted_capabilities:?}"),
            "policy":format!("{policy:?}"),
            "budget":format!("{budget:?}")
        }),
        JournalEvent::CompatibilityDecisionEvent { decision, .. } => {
            json!({"type":"CompatibilityDecisionEvent","decision":format!("{decision:?}")})
        }
        JournalEvent::RunBudgetOriginEvent { started_millis, .. } => {
            json!({"type":"RunBudgetOriginEvent","started_millis":started_millis})
        }
        _ => return Err("unexpected event in clean replay fixture".into()),
    };
    let mut fields = value
        .as_object()
        .cloned()
        .ok_or("event projection must be an object")?;
    fields.insert("run".into(), json!(event.run_id().get()));
    fields.insert("seq".into(), json!(event.seq().get()));
    Ok(serde_json::Value::Object(fields))
}
